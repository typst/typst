//! Layout of content into a [`Document`].

mod collect;
mod finalize;
mod run;

use comemo::{Track, Tracked, TrackedMut};
use ecow::EcoVec;
use typst_library::World;
use typst_library::diag::{At, SourceResult};
use typst_library::engine::{Engine, Route, Sink, Traced};
use typst_library::foundations::{Content, StyleChain};
use typst_library::introspection::{
    Introspector, Locator, LocatorLink, ManualPageCounter, SplitLocator, TagElem,
};
use typst_library::layout::{FrameItem, Point};
use typst_library::model::DocumentInfo;
use typst_library::routines::{Arenas, Pair, RealizationKind, Routines};
use typst_utils::Protected;

use self::collect::{Item, collect};
use self::finalize::finalize;
use self::run::{LayoutedPage, layout_blank_page, layout_page_run};
use crate::{Page, PagedDocument, PagedIntrospector, PagedIntrospectorBuilder};
use crate::page_store::DiskPageStore;

/// Layout content into a document.
///
/// This first performs root-level realization and then lays out the resulting
/// elements. In contrast to [`layout_fragment`](crate::layout_fragment),
/// this does not take regions since the regions are defined by the page
/// configuration in the content and style chain.
#[typst_macros::time(name = "layout document")]
pub fn layout_document(
    engine: &mut Engine,
    content: &Content,
    styles: StyleChain,
) -> SourceResult<PagedDocument> {
    layout_document_impl(
        engine.routines,
        engine.world,
        engine.introspector.into_raw(),
        engine.traced,
        TrackedMut::reborrow_mut(&mut engine.sink),
        engine.route.track(),
        content,
        styles,
    )
}

/// The internal implementation of `layout_document`.
#[comemo::memoize]
#[allow(clippy::too_many_arguments)]
fn layout_document_impl(
    routines: &Routines,
    world: Tracked<dyn World + '_>,
    introspector: Tracked<dyn Introspector + '_>,
    traced: Tracked<Traced>,
    sink: TrackedMut<Sink>,
    route: Tracked<Route>,
    content: &Content,
    styles: StyleChain,
) -> SourceResult<PagedDocument> {
    layout_document_common(
        routines,
        world,
        introspector,
        traced,
        sink,
        route,
        content,
        Locator::root(),
        styles,
    )
}

/// Layout content into a document, as part of a bundle compilation process.
#[typst_macros::time(name = "layout document")]
pub fn layout_document_for_bundle(
    engine: &mut Engine,
    content: &Content,
    locator: Locator,
    styles: StyleChain,
) -> SourceResult<PagedDocument> {
    layout_document_for_bundle_impl(
        engine.routines,
        engine.world,
        engine.introspector.into_raw(),
        engine.traced,
        TrackedMut::reborrow_mut(&mut engine.sink),
        engine.route.track(),
        content,
        locator.track(),
        styles,
    )
}

/// The internal implementation of `layout_document_for_bundle`.
#[comemo::memoize]
#[allow(clippy::too_many_arguments)]
fn layout_document_for_bundle_impl(
    routines: &Routines,
    world: Tracked<dyn World + '_>,
    introspector: Tracked<dyn Introspector + '_>,
    traced: Tracked<Traced>,
    sink: TrackedMut<Sink>,
    route: Tracked<Route>,
    content: &Content,
    locator: Tracked<Locator>,
    styles: StyleChain,
) -> SourceResult<PagedDocument> {
    let link = LocatorLink::new(locator);
    layout_document_common(
        routines,
        world,
        introspector,
        traced,
        sink,
        route,
        content,
        Locator::link(&link),
        styles,
    )
}

/// The shared, unmemoized implementation of `layout_document` and
/// `layout_document_for_bundle`.
#[allow(clippy::too_many_arguments)]
fn layout_document_common(
    routines: &Routines,
    world: Tracked<dyn World + '_>,
    introspector: Tracked<dyn Introspector + '_>,
    traced: Tracked<Traced>,
    sink: TrackedMut<Sink>,
    route: Tracked<Route>,
    content: &Content,
    locator: Locator,
    styles: StyleChain,
) -> SourceResult<PagedDocument> {
    let introspector = Protected::from_raw(introspector);
    let mut locator = locator.split();
    let mut engine = Engine {
        routines,
        world,
        introspector,
        traced,
        sink,
        route: Route::extend(route).unnested(),
    };

    // Mark the external styles as "outside" so that they are valid at the page
    // level.
    let styles = styles.to_map().outside();
    let styles = StyleChain::new(&styles);
    let arenas = Arenas::default();

    let mut info = DocumentInfo::default();
    info.populate(styles);
    info.populate_locale(styles);

    let mut children = (engine.routines.realize)(
        RealizationKind::LayoutDocument { info: &mut info },
        &mut engine,
        &mut locator,
        &arenas,
        content,
        styles,
    )?;

    let (pages, store, introspector) = layout_pages_streaming(
        &mut engine, &mut children, &mut locator, styles,
    )?;

    let mut doc = PagedDocument::new(pages, info);
    if let Some(introspector) = introspector {
        // For large documents: introspector was built incrementally
        // during layout. Set it directly instead of lazy-building from pages.
        doc.set_introspector(introspector);
    }
    if let Some(store) = store {
        doc.set_page_store(store);
    }
    Ok(doc)
}

/// Page count threshold above which pages are spilled to disk during layout.
/// Below this, all pages are kept in memory (current behavior).
const SPILL_THRESHOLD: usize = 100;

/// Layout pages with streaming disk spill for large documents.
///
/// For small documents: returns (all pages, None, None) — current behavior.
/// For large documents: returns (empty pages, store, introspector).
///   Pages are serialized to disk as produced. Introspector is built
///   incrementally from each page before the page is dropped. After
///   processing each page run, comemo cache is evicted to free frame data.
fn layout_pages_streaming<'a>(
    engine: &mut Engine,
    children: &'a mut [Pair<'a>],
    locator: &mut SplitLocator<'a>,
    styles: StyleChain<'a>,
) -> SourceResult<(EcoVec<Page>, Option<DiskPageStore>, Option<PagedIntrospector>)> {
    // Slice up the children into logical parts.
    let items = collect(children, locator, styles);

    // Layout the page runs in parallel.
    let mut runs = engine.parallelize(
        items.iter().filter_map(|item| match item {
            Item::Run(children, initial, locator) => {
                Some((children, initial, locator.relayout()))
            }
            _ => None,
        }),
        |engine, (children, initial, locator)| {
            layout_page_run(engine, children, locator, *initial)
        },
    );

    let mut pages = EcoVec::new();
    let mut tags = vec![];
    let mut counter = ManualPageCounter::new();
    let mut store: Option<DiskPageStore> = None;
    let mut intro_builder: Option<PagedIntrospectorBuilder> = None;
    let mut total_pages: usize = 0;

    for item in &items {
        match item {
            Item::Run(..) => {
                let layouted = runs.next().unwrap()?;
                let run_page_count = layouted.len();

                // Only spill to disk if THIS page run is large (single big table).
                // Don't spill for many small tables that happen to total > threshold.
                let should_spill = run_page_count > SPILL_THRESHOLD;

                for layouted in layouted {
                    let page = finalize(engine, &mut counter, &mut tags, layouted)?;
                    pages.push(page);
                    total_pages += 1;
                }

                // Evict comemo after large page runs to free per-cell caches.
                // Only during first convergence iteration — subsequent
                // iterations benefit from cache hits for fast validation.
                if typst_library::engine_flags::is_layout_eviction_enabled() {
                    if run_page_count > SPILL_THRESHOLD || (total_pages % 200 == 0 && total_pages > 0) {
                        comemo::evict(0);
                    }
                }
            }
            Item::Parity(parity, initial, locator) => {
                if !parity.matches(total_pages) {
                    continue;
                }

                let layouted = layout_blank_page(engine, locator.relayout(), *initial)?;
                let page = finalize(engine, &mut counter, &mut tags, layouted)?;

                if let Some(ref mut ib) = intro_builder {
                    ib.discover_page(total_pages, &page);
                }
                if let Some(ref mut s) = store {
                    s.append_page(&page)
                        .map_err(|e| ecow::eco_format!("disk spill failed: {e}"))
                        .at(typst_syntax::Span::detached())?;
                } else {
                    pages.push(page);
                }

                total_pages += 1;
            }
            Item::Tags(items) => {
                tags.extend(
                    items
                        .iter()
                        .filter_map(|(c, _)| c.to_packed::<TagElem>())
                        .map(|elem| elem.tag.clone()),
                );
            }
        }
    }

    // Add remaining tags to the last page.
    if !tags.is_empty() {
        if store.is_some() {
            // Tags at the end of a spilled document — these are rare.
            // They won't be in the introspector, but that's acceptable.
        } else if let Some(last) = pages.make_mut().last_mut() {
            let pos = Point::with_y(last.frame.height());
            last.frame
                .push_multiple(tags.into_iter().map(|tag| (pos, FrameItem::Tag(tag))));
        }
    }

    // Build the introspector if we were building incrementally.
    let introspector = intro_builder.map(|ib| ib.finish_incremental(total_pages));

    Ok((pages, store, introspector))
}

/// Layouts the document's pages.
fn layout_pages<'a>(
    engine: &mut Engine,
    children: &'a mut [Pair<'a>],
    locator: &mut SplitLocator<'a>,
    styles: StyleChain<'a>,
) -> SourceResult<EcoVec<Page>> {
    // Slice up the children into logical parts.
    let items = collect(children, locator, styles);

    // Layout the page runs in parallel.
    let mut runs = engine.parallelize(
        items.iter().filter_map(|item| match item {
            Item::Run(children, initial, locator) => {
                Some((children, initial, locator.relayout()))
            }
            _ => None,
        }),
        |engine, (children, initial, locator)| {
            layout_page_run(engine, children, locator, *initial)
        },
    );

    let mut pages = EcoVec::new();
    let mut tags = vec![];
    let mut counter = ManualPageCounter::new();

    // Collect and finalize the runs, handling things like page parity and tags
    // between pages.
    for item in &items {
        match item {
            Item::Run(..) => {
                let layouted = runs.next().unwrap()?;
                for layouted in layouted {
                    let page = finalize(engine, &mut counter, &mut tags, layouted)?;
                    pages.push(page);
                }
            }
            Item::Parity(parity, initial, locator) => {
                if !parity.matches(pages.len()) {
                    continue;
                }

                let layouted = layout_blank_page(engine, locator.relayout(), *initial)?;
                let page = finalize(engine, &mut counter, &mut tags, layouted)?;
                pages.push(page);
            }
            Item::Tags(items) => {
                tags.extend(
                    items
                        .iter()
                        .filter_map(|(c, _)| c.to_packed::<TagElem>())
                        .map(|elem| elem.tag.clone()),
                );
            }
        }
    }

    // Add the remaining tags to the very end of the last page.
    if !tags.is_empty() {
        let last = pages.make_mut().last_mut().unwrap();
        let pos = Point::with_y(last.frame.height());
        last.frame
            .push_multiple(tags.into_iter().map(|tag| (pos, FrameItem::Tag(tag))));
    }

    Ok(pages)
}
