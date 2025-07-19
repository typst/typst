//! Layout of content into a [`Document`].

mod collect;
mod finalize;
mod run;

use std::num::NonZeroUsize;

use comemo::{Tracked, TrackedMut};
use typst_library::World;
use typst_library::diag::SourceResult;
use typst_library::engine::{Engine, Route, Sink, Traced};
use typst_library::foundations::{Content, StyleChain};
use typst_library::introspection::{
    Introspector, IntrospectorBuilder, Locator, ManualPageCounter, SplitLocator, TagElem,
};
use typst_library::layout::{FrameItem, Page, PagedDocument, Point, Position, Transform};
use typst_library::model::DocumentInfo;
use typst_library::routines::{Arenas, Pair, RealizationKind, Routines};

use self::collect::{Item, collect};
use self::finalize::finalize;
use self::run::{LayoutedPage, layout_blank_page, layout_page_run};

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
        engine.introspector,
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
    introspector: Tracked<Introspector>,
    traced: Tracked<Traced>,
    sink: TrackedMut<Sink>,
    route: Tracked<Route>,
    content: &Content,
    styles: StyleChain,
) -> SourceResult<PagedDocument> {
    let mut locator = Locator::root().split();
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
    let mut children = (engine.routines.realize)(
        RealizationKind::LayoutDocument { info: &mut info },
        &mut engine,
        &mut locator,
        &arenas,
        content,
        styles,
    )?;

    let pages = layout_pages(&mut engine, &mut children, &mut locator, styles)?;
    let introspector = introspect_pages(&pages);

    Ok(PagedDocument { pages, info, introspector })
}

/// Layouts the document's pages.
fn layout_pages<'a>(
    engine: &mut Engine,
    children: &'a mut [Pair<'a>],
    locator: &mut SplitLocator<'a>,
    styles: StyleChain<'a>,
) -> SourceResult<Vec<Page>> {
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

    let mut pages = vec![];
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
        let last = pages.last_mut().unwrap();
        let pos = Point::with_y(last.frame.height());
        last.frame
            .push_multiple(tags.into_iter().map(|tag| (pos, FrameItem::Tag(tag))));
    }

    Ok(pages)
}

/// Introspects pages.
#[typst_macros::time(name = "introspect pages")]
fn introspect_pages(pages: &[Page]) -> Introspector {
    let mut builder = IntrospectorBuilder::new();
    builder.pages = pages.len();
    builder.page_numberings.reserve(pages.len());
    builder.page_supplements.reserve(pages.len());

    // Discover all elements.
    let mut elems = Vec::new();
    for (i, page) in pages.iter().enumerate() {
        handle_page(&mut builder, &mut elems, page, i);
    }

    builder.finalize(elems)
}

#[typst_macros::time(name = "handle page")]
fn handle_page(
    builder: &mut IntrospectorBuilder,
    elems: &mut Vec<(Content, Position)>,
    page: &Page,
    i: usize,
) {
    builder.page_numberings.push(page.numbering.clone());
    builder.page_supplements.push(page.supplement.clone());
    builder.discover_in_frame(
        elems,
        &page.frame,
        NonZeroUsize::new(1 + i).unwrap(),
        Transform::identity(),
    );
}
