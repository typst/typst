//! Layout of content into a [`Document`].

mod collect;
mod finalize;
mod run;

use comemo::{Tracked, TrackedMut};
use typst_library::diag::SourceResult;
use typst_library::engine::{Engine, Route, Sink, Traced};
use typst_library::foundations::{Content, StyleChain};
use typst_library::introspection::{
    Introspector, Locator, ManualPageCounter, SplitLocator, TagElem,
};
use typst_library::layout::{FrameItem, Page, Point};
use typst_library::model::{Document, DocumentInfo};
use typst_library::routines::{Arenas, Pair, RealizationKind, Routines};
use typst_library::World;

use self::collect::{collect, Item};
use self::finalize::finalize;
use self::run::{layout_blank_page, layout_page_run, LayoutedPage};

/// Layout content into a document.
///
/// This first performs root-level realization and then lays out the resulting
/// elements. In contrast to [`layout_fragment`](crate::layout_fragment),
/// this does not take regions since the regions are defined by the page
/// configuration in the content and style chain.
#[typst_macros::time(name = "document")]
pub fn layout_document(
    engine: &mut Engine,
    content: &Content,
    styles: StyleChain,
) -> SourceResult<Document> {
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
) -> SourceResult<Document> {
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
        RealizationKind::Root(&mut info),
        &mut engine,
        &mut locator,
        &arenas,
        content,
        styles,
    )?;

    let pages = layout_pages(&mut engine, &mut children, locator, styles)?;
    let introspector = Introspector::new(&pages);

    Ok(Document { pages, info, introspector })
}

/// Layouts the document's pages.
fn layout_pages<'a>(
    engine: &mut Engine,
    children: &'a mut [Pair<'a>],
    locator: SplitLocator<'a>,
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
