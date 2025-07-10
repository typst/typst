use comemo::{Track, Tracked, TrackedMut};
use typst_library::diag::{At, SourceResult};
use typst_library::engine::{Engine, Route, Sink, Traced};
use typst_library::foundations::{Content, StyleChain};
use typst_library::introspection::{Introspector, Locator, LocatorLink};

use typst_library::routines::{Arenas, FragmentKind, RealizationKind, Routines};
use typst_library::World;

use crate::HtmlNode;

/// Produce HTML nodes from content.
#[typst_macros::time(name = "html fragment")]
pub fn html_fragment(
    engine: &mut Engine,
    content: &Content,
    locator: Locator,
    styles: StyleChain,
) -> SourceResult<Vec<HtmlNode>> {
    html_fragment_impl(
        engine.routines,
        engine.world,
        engine.introspector,
        engine.traced,
        TrackedMut::reborrow_mut(&mut engine.sink),
        engine.route.track(),
        content,
        locator.track(),
        styles,
    )
}

/// The cached, internal implementation of [`html_fragment`].
#[comemo::memoize]
#[allow(clippy::too_many_arguments)]
fn html_fragment_impl(
    routines: &Routines,
    world: Tracked<dyn World + '_>,
    introspector: Tracked<Introspector>,
    traced: Tracked<Traced>,
    sink: TrackedMut<Sink>,
    route: Tracked<Route>,
    content: &Content,
    locator: Tracked<Locator>,
    styles: StyleChain,
) -> SourceResult<Vec<HtmlNode>> {
    let link = LocatorLink::new(locator);
    let mut locator = Locator::link(&link).split();
    let mut engine = Engine {
        routines,
        world,
        introspector,
        traced,
        sink,
        route: Route::extend(route),
    };

    engine.route.check_html_depth().at(content.span())?;

    let arenas = Arenas::default();
    let children = (engine.routines.realize)(
        // No need to know about the `FragmentKind` because we handle both
        // uniformly.
        RealizationKind::HtmlFragment {
            kind: &mut FragmentKind::Block,
            is_inline: crate::convert::is_inline,
        },
        &mut engine,
        &mut locator,
        &arenas,
        content,
        styles,
    )?;

    crate::convert::convert_to_nodes(&mut engine, &mut locator, children.iter().copied())
}
