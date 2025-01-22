#[path = "box.rs"]
mod box_;
mod collect;
mod deco;
mod finalize;
mod line;
mod linebreak;
mod prepare;
mod shaping;

pub use self::box_::layout_box;

use comemo::{Track, Tracked, TrackedMut};
use typst_library::diag::SourceResult;
use typst_library::engine::{Engine, Route, Sink, Traced};
use typst_library::foundations::{StyleChain, StyleVec};
use typst_library::introspection::{Introspector, Locator, LocatorLink};
use typst_library::layout::{Fragment, Size};
use typst_library::model::ParElem;
use typst_library::routines::Routines;
use typst_library::World;

use self::collect::{collect, Item, Segment, SpanMapper};
use self::deco::decorate;
use self::finalize::finalize;
use self::line::{apply_baseline_shift, commit, line, Line};
use self::linebreak::{linebreak, Breakpoint};
use self::prepare::{prepare, Preparation};
use self::shaping::{
    cjk_punct_style, is_of_cj_script, shape_range, ShapedGlyph, ShapedText,
    BEGIN_PUNCT_PAT, END_PUNCT_PAT,
};

/// Range of a substring of text.
type Range = std::ops::Range<usize>;

/// Layouts content inline.
pub fn layout_inline(
    engine: &mut Engine,
    children: &StyleVec,
    locator: Locator,
    styles: StyleChain,
    consecutive: bool,
    region: Size,
    expand: bool,
) -> SourceResult<Fragment> {
    layout_inline_impl(
        children,
        engine.routines,
        engine.world,
        engine.introspector,
        engine.traced,
        TrackedMut::reborrow_mut(&mut engine.sink),
        engine.route.track(),
        locator.track(),
        styles,
        consecutive,
        region,
        expand,
    )
}

/// The internal, memoized implementation of `layout_inline`.
#[comemo::memoize]
#[allow(clippy::too_many_arguments)]
fn layout_inline_impl(
    children: &StyleVec,
    routines: &Routines,
    world: Tracked<dyn World + '_>,
    introspector: Tracked<Introspector>,
    traced: Tracked<Traced>,
    sink: TrackedMut<Sink>,
    route: Tracked<Route>,
    locator: Tracked<Locator>,
    styles: StyleChain,
    consecutive: bool,
    region: Size,
    expand: bool,
) -> SourceResult<Fragment> {
    let link = LocatorLink::new(locator);
    let locator = Locator::link(&link);
    let mut engine = Engine {
        routines,
        world,
        introspector,
        traced,
        sink,
        route: Route::extend(route),
    };

    let mut locator = locator.split();

    // Collect all text into one string for BiDi analysis.
    let (text, segments, spans) =
        collect(children, &mut engine, &mut locator, &styles, region, consecutive)?;

    // Perform BiDi analysis and then prepares paragraph layout.
    let p = prepare(&mut engine, children, &text, segments, spans, styles)?;

    // Break the paragraph into lines.
    let lines = linebreak(&engine, &p, region.x - p.hang);

    // Turn the selected lines into frames.
    finalize(&mut engine, &p, &lines, styles, region, expand, &mut locator)
}
