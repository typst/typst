use comemo::{Track, Tracked, TrackedMut};
use ecow::EcoVec;
use typst_library::World;
use typst_library::diag::{At, SourceResult};
use typst_library::engine::{Engine, Route, Sink, Traced};
use typst_library::foundations::{Content, StyleChain};
use typst_library::introspection::{Introspector, Locator, LocatorLink, SplitLocator};
use typst_library::routines::{Arenas, FragmentKind, Pair, RealizationKind, Routines};
use typst_library::text::SmartQuoter;
use typst_utils::Protected;

use crate::convert::{ConversionLevel, Whitespace};
use crate::{HtmlElem, HtmlNode};

/// Produces HTML nodes from content contained in an HTML element that is
/// block-level by default.
#[typst_macros::time(name = "html block fragment")]
pub fn html_block_fragment(
    engine: &mut Engine,
    content: &Content,
    locator: Locator,
    styles: StyleChain,
    whitespace: Whitespace,
) -> SourceResult<EcoVec<HtmlNode>> {
    html_block_fragment_impl(
        engine.routines,
        engine.world,
        engine.introspector.into_raw(),
        engine.traced,
        TrackedMut::reborrow_mut(&mut engine.sink),
        engine.route.track(),
        content,
        locator.track(),
        styles,
        whitespace,
    )
}

/// The cached, internal implementation of [`html_fragment`].
#[comemo::memoize]
#[allow(clippy::too_many_arguments)]
fn html_block_fragment_impl(
    routines: &Routines,
    world: Tracked<dyn World + '_>,
    introspector: Tracked<Introspector>,
    traced: Tracked<Traced>,
    sink: TrackedMut<Sink>,
    route: Tracked<Route>,
    content: &Content,
    locator: Tracked<Locator>,
    styles: StyleChain,
    whitespace: Whitespace,
) -> SourceResult<EcoVec<HtmlNode>> {
    let introspector = Protected::from_raw(introspector);
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
    let children = realize_fragment(&mut engine, &mut locator, &arenas, content, styles)?;
    crate::convert::convert_to_nodes(
        &mut engine,
        &mut locator,
        children.iter().copied(),
        ConversionLevel::Block,
        whitespace,
    )
}

/// Produces HTML nodes from content contained in an HTML element that is
/// inline-level by default.
///
/// The difference to block-level content is that inline-level content has
/// shared smartquoting state with surrounding inline-level content. This
/// requires mutable state, which is at odds with memoization. However, the
/// caching granularity would be unnecessarily high anyway if every single
/// fragment was cached, so this works out pretty well together.
#[typst_macros::time(name = "html inline fragment")]
pub fn html_inline_fragment(
    engine: &mut Engine,
    content: &Content,
    locator: &mut SplitLocator,
    quoter: &mut SmartQuoter,
    styles: StyleChain,
    whitespace: Whitespace,
) -> SourceResult<EcoVec<HtmlNode>> {
    engine.route.increase();
    engine.route.check_html_depth().at(content.span())?;

    let arenas = Arenas::default();
    let children = realize_fragment(engine, locator, &arenas, content, styles)?;
    let result = crate::convert::convert_to_nodes(
        engine,
        locator,
        children.iter().copied(),
        ConversionLevel::Inline(quoter),
        whitespace,
    );

    engine.route.decrease();
    result
}

/// Realizes the body of an HTML fragment.
fn realize_fragment<'a>(
    engine: &mut Engine,
    locator: &mut SplitLocator,
    arenas: &'a Arenas,
    content: &'a Content,
    styles: StyleChain<'a>,
) -> SourceResult<Vec<Pair<'a>>> {
    (engine.routines.realize)(
        RealizationKind::HtmlFragment {
            // We ignore the `FragmentKind` because we handle both uniformly.
            kind: &mut FragmentKind::Block,
            is_phrasing: HtmlElem::is_phrasing,
        },
        engine,
        locator,
        arenas,
        content,
        styles,
    )
}
