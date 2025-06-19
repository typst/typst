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
pub use self::shaping::create_shape_plan;

use comemo::{Track, Tracked, TrackedMut};
use typst_library::World;
use typst_library::diag::SourceResult;
use typst_library::engine::{Engine, Route, Sink, Traced};
use typst_library::foundations::{Packed, Resolve, Smart, StyleChain};
use typst_library::introspection::{Introspector, Locator, LocatorLink, SplitLocator};
use typst_library::layout::{Abs, AlignElem, Dir, FixedAlignment, Fragment, Size};
use typst_library::model::{
    EnumElem, FirstLineIndent, Linebreaks, ListElem, ParElem, ParLine, ParLineMarker,
    TermsElem,
};
use typst_library::routines::{Arenas, Pair, RealizationKind, Routines};
use typst_library::text::{Costs, Lang, TextElem};
use typst_utils::{Numeric, SliceExt};

use self::collect::{Item, Segment, SpanMapper, collect};
use self::deco::decorate;
use self::finalize::finalize;
use self::line::{Line, apply_baseline_shift, commit, line};
use self::linebreak::{Breakpoint, linebreak};
use self::prepare::{Preparation, prepare};
use self::shaping::{
    BEGIN_PUNCT_PAT, END_PUNCT_PAT, ShapedGlyph, ShapedText, cjk_punct_style,
    is_of_cj_script, shape_range,
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

    layout_inline_impl(
        &mut engine,
        &children,
        &mut locator,
        styles,
        region,
        expand,
        Some(situation),
        &ConfigBase {
            justify: elem.justify(styles),
            linebreaks: elem.linebreaks(styles),
            first_line_indent: elem.first_line_indent(styles),
            hanging_indent: elem.hanging_indent(styles),
        },
    )
}

/// Lays out realized content with inline layout.
pub fn layout_inline<'a>(
    engine: &mut Engine,
    children: &[Pair<'a>],
    locator: &mut SplitLocator<'a>,
    shared: StyleChain<'a>,
    region: Size,
    expand: bool,
) -> SourceResult<Fragment> {
    layout_inline_impl(
        engine,
        children,
        locator,
        shared,
        region,
        expand,
        None,
        &ConfigBase {
            justify: ParElem::justify_in(shared),
            linebreaks: ParElem::linebreaks_in(shared),
            first_line_indent: ParElem::first_line_indent_in(shared),
            hanging_indent: ParElem::hanging_indent_in(shared),
        },
    )
}

/// The internal implementation of [`layout_inline`].
#[allow(clippy::too_many_arguments)]
fn layout_inline_impl<'a>(
    engine: &mut Engine,
    children: &[Pair<'a>],
    locator: &mut SplitLocator<'a>,
    shared: StyleChain<'a>,
    region: Size,
    expand: bool,
    par: Option<ParSituation>,
    base: &ConfigBase,
) -> SourceResult<Fragment> {
    // Prepare configuration that is shared across the whole inline layout.
    let config = configuration(base, children, shared, par);

    // Collect all text into one string for BiDi analysis.
    let (text, segments, spans) = collect(children, engine, locator, &config, region)?;

    // Perform BiDi analysis and performs some preparation steps before we
    // proceed to line breaking.
    let p = prepare(engine, &config, &text, segments, spans)?;

    // Break the text into lines.
    let lines = linebreak(engine, &p, region.x - config.hanging_indent);

    // Turn the selected lines into frames.
    finalize(engine, &p, &lines, region, expand, locator)
}

/// Determine the inline layout's configuration.
fn configuration(
    base: &ConfigBase,
    children: &[Pair],
    shared: StyleChain,
    situation: Option<ParSituation>,
) -> Config {
    let justify = base.justify;
    let font_size = TextElem::size_in(shared);
    let dir = TextElem::dir_in(shared);

    Config {
        justify,
        linebreaks: base.linebreaks.unwrap_or_else(|| {
            if justify { Linebreaks::Optimized } else { Linebreaks::Simple }
        }),
        first_line_indent: {
            let FirstLineIndent { amount, all } = base.first_line_indent;
            if !amount.is_zero()
                && match situation {
                    // First-line indent for the first paragraph after a list
                    // bullet just looks bad.
                    Some(ParSituation::First) => all && !in_list(shared),
                    Some(ParSituation::Consecutive) => true,
                    Some(ParSituation::Other) => all,
                    None => false,
                }
                && AlignElem::alignment_in(shared).resolve(shared).x == dir.start().into()
            {
                amount.at(font_size)
            } else {
                Abs::zero()
            }
        },
        hanging_indent: if situation.is_some() {
            base.hanging_indent
        } else {
            Abs::zero()
        },
        numbering_marker: ParLine::numbering_in(shared).map(|numbering| {
            Packed::new(ParLineMarker::new(
                numbering,
                ParLine::number_align_in(shared),
                ParLine::number_margin_in(shared),
                // Delay resolving the number clearance until line numbers are
                // laid out to avoid inconsistent spacing depending on varying
                // font size.
                ParLine::number_clearance_in(shared),
            ))
        }),
        align: AlignElem::alignment_in(shared).fix(dir).x,
        font_size,
        dir,
        hyphenate: shared_get(children, shared, TextElem::hyphenate_in)
            .map(|uniform| uniform.unwrap_or(justify)),
        lang: shared_get(children, shared, TextElem::lang_in),
        fallback: TextElem::fallback_in(shared),
        cjk_latin_spacing: TextElem::cjk_latin_spacing_in(shared).is_auto(),
        costs: TextElem::costs_in(shared),
    }
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

/// Raw values from a `ParElem` or style chain. Used to initialize a [`Config`].
struct ConfigBase {
    justify: bool,
    linebreaks: Smart<Linebreaks>,
    first_line_indent: FirstLineIndent,
    hanging_indent: Abs,
}

/// Shared configuration for the whole inline layout.
struct Config {
    /// Whether to justify text.
    justify: bool,
    /// How to determine line breaks.
    linebreaks: Linebreaks,
    /// The indent the first line of a paragraph should have.
    first_line_indent: Abs,
    /// The indent that all but the first line of a paragraph should have.
    hanging_indent: Abs,
    /// Configuration for line numbering.
    numbering_marker: Option<Packed<ParLineMarker>>,
    /// The resolved horizontal alignment.
    align: FixedAlignment,
    /// The text size.
    font_size: Abs,
    /// The dominant direction.
    dir: Dir,
    /// A uniform hyphenation setting (only `Some(_)` if it's the same for all
    /// children, otherwise `None`).
    hyphenate: Option<bool>,
    /// The text language (only `Some(_)` if it's the same for all
    /// children, otherwise `None`).
    lang: Option<Lang>,
    /// Whether font fallback is enabled.
    fallback: bool,
    /// Whether to add spacing between CJK and Latin characters.
    cjk_latin_spacing: bool,
    /// Costs for various layout decisions.
    costs: Costs,
}

/// Get a style property, but only if it is the same for all of the children.
fn shared_get<T: PartialEq>(
    children: &[Pair],
    styles: StyleChain<'_>,
    getter: fn(StyleChain) -> T,
) -> Option<T> {
    let value = getter(styles);
    children
        .group_by_key(|&(_, s)| s)
        .all(|(s, _)| getter(s) == value)
        .then_some(value)
}

/// Whether we have a list ancestor.
///
/// When we support some kind of more general ancestry mechanism, this can
/// become more elegant.
fn in_list(styles: StyleChain) -> bool {
    ListElem::depth_in(styles).0 > 0
        || !EnumElem::parents_in(styles).is_empty()
        || TermsElem::within_in(styles)
}
