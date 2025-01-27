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
use typst_library::foundations::{Packed, StyleChain};
use typst_library::introspection::{Introspector, Locator, LocatorLink, SplitLocator};
use typst_library::layout::{Fragment, Size};
use typst_library::model::ParElem;
use typst_library::routines::{Arenas, Pair, RealizationKind, Routines};
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

/// Layouts the paragraph.
pub fn layout_par(
    elem: &Packed<ParElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    region: Size,
    expand: bool,
    situation: ParSituation,
) -> SourceResult<Fragment> {
    layout_par_impl(
        elem,
        engine.routines,
        engine.world,
        engine.introspector,
        engine.traced,
        TrackedMut::reborrow_mut(&mut engine.sink),
        engine.route.track(),
        locator.track(),
        styles,
        region,
        expand,
        situation,
    )
}

/// The internal, memoized implementation of `layout_par`.
#[comemo::memoize]
#[allow(clippy::too_many_arguments)]
fn layout_par_impl(
    elem: &Packed<ParElem>,
    routines: &Routines,
    world: Tracked<dyn World + '_>,
    introspector: Tracked<Introspector>,
    traced: Tracked<Traced>,
    sink: TrackedMut<Sink>,
    route: Tracked<Route>,
    locator: Tracked<Locator>,
    styles: StyleChain,
    region: Size,
    expand: bool,
    situation: ParSituation,
) -> SourceResult<Fragment> {
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

    let arenas = Arenas::default();
    let children = (engine.routines.realize)(
        RealizationKind::LayoutPar,
        &mut engine,
        &mut locator,
        &arenas,
        &elem.body,
        styles,
    )?;

    layout_inline(
        &mut engine,
        &children,
        &mut locator,
        styles,
        region,
        expand,
        Some(situation),
    )
}

/// Lays out realized content with inline layout.
#[allow(clippy::too_many_arguments)]
pub fn layout_inline<'a>(
    engine: &mut Engine,
    children: &[Pair<'a>],
    locator: &mut SplitLocator<'a>,
    styles: StyleChain<'a>,
    region: Size,
    expand: bool,
    par: Option<ParSituation>,
) -> SourceResult<Fragment> {
    // Collect all text into one string for BiDi analysis.
    let (text, segments, spans) =
        collect(children, engine, locator, styles, region, par)?;

    // Perform BiDi analysis and performs some preparation steps before we
    // proceed to line breaking.
    let p = prepare(engine, children, &text, segments, spans, styles, par)?;

    // Break the text into lines.
    let lines = linebreak(engine, &p, region.x - p.hang);

    // Turn the selected lines into frames.
    finalize(engine, &p, &lines, styles, region, expand, locator)
}

/// Distinguishes between a few different kinds of paragraphs.
///
/// In the form `Option<ParSituation>`, `None` implies that we are creating an
/// inline layout that isn't a semantic paragraph.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ParSituation {
    /// The paragraph is the first thing in the flow.
    First,
    /// The paragraph follows another paragraph.
    Consecutive,
    /// Any other kind of paragraph.
    Other,
}
