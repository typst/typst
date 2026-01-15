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
pub use self::line::{LineMetricsRaw, measure_line};
pub use self::linebreak::linebreak_variable_width;
pub use self::shaping::{SharedShapingContext, create_shape_plan, get_font_and_covers};

use comemo::{Track, Tracked, TrackedMut};
use typst_library::World;
use typst_library::diag::SourceResult;
use typst_library::engine::{Engine, Route, Sink, Traced};
use typst_library::foundations::{Packed, Smart, StyleChain};
use typst_library::introspection::{Introspector, Locator, LocatorLink, SplitLocator};
use typst_library::layout::{Abs, AlignElem, Dir, FixedAlignment, Fragment, Frame, Size};
use typst_library::model::{
    EnumElem, FirstLineIndent, JustificationLimits, Linebreaks, ListElem, ParElem,
    ParLine, ParLineMarker, TermsElem,
};
use typst_library::routines::{Arenas, Pair, RealizationKind, Routines};
use typst_library::text::{Costs, Lang, TextElem};
use typst_utils::{Numeric, Protected, SliceExt};

use self::collect::{Item, Segment, SpanMapper, collect};
use self::deco::decorate;
use self::finalize::finalize;
use self::line::{Line, apply_shift, commit, line};
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
        engine.introspector.into_raw(),
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
            justify: elem.justify.get(styles),
            linebreaks: elem.linebreaks.get(styles),
            first_line_indent: elem.first_line_indent.get(styles),
            hanging_indent: elem.hanging_indent.resolve(styles),
        },
    )
}

/// Measure a paragraph without creating frames.
///
/// Returns line metrics and break positions that can be used to:
/// 1. Position the paragraph (using total_height)
/// 2. Compute exclusion zones for wrap-floats (using line_metrics)
/// 3. Later commit to frames via commit_par() (using break_info)
///
/// NOT memoized - the refinement loop needs fresh results each iteration.
pub fn measure_par_with_exclusions(
    elem: &Packed<ParElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    region: Size,
    expand: bool,
    situation: ParSituation,
    // exclusions: Option<&ParExclusions>,  // Uncomment in Phase 2
) -> SourceResult<ParMeasureResult> {
    let arenas = Arenas::default();
    let mut split_locator = locator.split();

    let children = (engine.routines.realize)(
        RealizationKind::LayoutPar,
        engine,
        &mut split_locator,
        &arenas,
        &elem.body,
        styles,
    )?;

    let base = ConfigBase {
        justify: elem.justify.get(styles),
        linebreaks: elem.linebreaks.get(styles),
        first_line_indent: elem.first_line_indent.get(styles),
        hanging_indent: elem.hanging_indent.resolve(styles),
    };

    measure_par_inner(
        engine,
        &children,
        &mut split_locator,
        styles,
        region,
        expand,
        Some(situation),
        &base,
    )
}

/// Inner measurement implementation.
fn measure_par_inner<'a>(
    engine: &mut Engine,
    children: &[Pair<'a>],
    locator: &mut SplitLocator<'a>,
    styles: StyleChain<'a>,
    region: Size,
    _expand: bool,
    par: Option<ParSituation>,
    base: &ConfigBase,
) -> SourceResult<ParMeasureResult> {
    let config = configuration(base, children, styles, par);
    let (text, segments, spans) = collect(children, engine, locator, &config, region)?;
    let p = prepare(engine, &config, &text, segments, spans)?;

    // Line breaking
    let width = region.x - config.hanging_indent;
    let lines = linebreak(engine, &p, width);

    // Extract metrics WITHOUT building frames
    let leading = styles.resolve(ParElem::leading);
    let mut total_height = Abs::zero();
    let mut line_metrics = Vec::with_capacity(lines.len());
    let mut break_info = Vec::with_capacity(lines.len());

    for (i, ln) in lines.iter().enumerate() {
        if i > 0 {
            total_height += leading;
        }

        let raw = measure_line(engine, ln);
        let metrics = LineMetrics {
            width: ln.width,
            height: raw.height(),
            ascent: raw.ascent,
            descent: raw.descent,
        };

        total_height += metrics.height;
        line_metrics.push(metrics);

        break_info.push(BreakInfo {
            end: ln.end,
            breakpoint: ln.breakpoint,
        });
    }

    Ok(ParMeasureResult {
        line_metrics,
        total_height,
        break_info,
    })
}

/// Commit a measured paragraph to frames.
///
/// Must be called with the SAME inputs as measure_par_with_exclusions().
/// If inputs differ, line reconstruction may produce different results.
pub fn commit_par(
    elem: &Packed<ParElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    region: Size,
    expand: bool,
    situation: ParSituation,
    measured: &ParMeasureResult,
    // exclusions: Option<&ParExclusions>,  // Uncomment in Phase 2
) -> SourceResult<ParCommitResult> {
    let arenas = Arenas::default();
    let mut split_locator = locator.split();

    let children = (engine.routines.realize)(
        RealizationKind::LayoutPar,
        engine,
        &mut split_locator,
        &arenas,
        &elem.body,
        styles,
    )?;

    let base = ConfigBase {
        justify: elem.justify.get(styles),
        linebreaks: elem.linebreaks.get(styles),
        first_line_indent: elem.first_line_indent.get(styles),
        hanging_indent: elem.hanging_indent.resolve(styles),
    };

    let config = configuration(&base, &children, styles, Some(situation));
    let (text, segments, spans) = collect(&children, engine, &mut split_locator, &config, region)?;
    let p = prepare(engine, &config, &text, segments, spans)?;

    // Reconstruct lines from stored break info (NOT re-running linebreak)
    let lines = reconstruct_lines(engine, &p, &measured.break_info);

    // Create frames using existing finalize
    let fragment = finalize(engine, &p, &lines, region, expand, &mut split_locator)?;

    Ok(ParCommitResult {
        frames: fragment.into_frames(),
    })
}

/// Reconstruct Line objects from stored break information.
///
/// Produces identical lines to original linebreak() because:
/// 1. Preparation rebuilt from same inputs
/// 2. line() is deterministic given (range, breakpoint, pred)
/// 3. Built sequentially for correct predecessor chain
fn reconstruct_lines<'a>(
    engine: &Engine,
    p: &'a Preparation<'a>,
    break_info: &[BreakInfo],
) -> Vec<Line<'a>> {
    let mut lines = Vec::with_capacity(break_info.len());
    let mut start = 0;

    for info in break_info {
        let prev = lines.last();
        let ln = line(engine, p, start..info.end, info.breakpoint, prev);
        start = info.end;
        lines.push(ln);
    }

    lines
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
            justify: shared.get(ParElem::justify),
            linebreaks: shared.get(ParElem::linebreaks),
            first_line_indent: shared.get(ParElem::first_line_indent),
            hanging_indent: shared.resolve(ParElem::hanging_indent),
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

    // DEBUG: Verify that reconstruct_lines produces identical lines to linebreak.
    // This is the core invariant for the measure/commit split.
    #[cfg(debug_assertions)]
    {
        // Extract break info from the lines (simulating what measure_par does)
        let break_info: Vec<BreakInfo> = lines
            .iter()
            .map(|ln| BreakInfo {
                end: ln.end,
                breakpoint: ln.breakpoint,
            })
            .collect();

        // Reconstruct lines from break info (simulating what commit_par does)
        let reconstructed = reconstruct_lines(engine, &p, &break_info);

        // Verify line count matches
        debug_assert_eq!(
            lines.len(),
            reconstructed.len(),
            "reconstruct_lines produced different number of lines"
        );

        // Verify each line matches
        for (i, (orig, recon)) in lines.iter().zip(reconstructed.iter()).enumerate() {
            debug_assert_eq!(
                orig.end, recon.end,
                "Line {i}: end position differs ({} vs {})",
                orig.end, recon.end
            );
            debug_assert_eq!(
                orig.breakpoint, recon.breakpoint,
                "Line {i}: breakpoint differs"
            );
            // Compare widths with small tolerance for floating point
            let width_diff = (orig.width - recon.width).abs();
            debug_assert!(
                width_diff < Abs::pt(0.001),
                "Line {i}: width differs by {width_diff:?}"
            );
        }
    }

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
    let font_size = shared.resolve(TextElem::size);
    let dir = shared.resolve(TextElem::dir);

    Config {
        justify,
        justification_limits: shared.get(ParElem::justification_limits),
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
                && shared.resolve(AlignElem::alignment).x == dir.start().into()
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
        numbering_marker: shared.get_cloned(ParLine::numbering).map(|numbering| {
            Packed::new(ParLineMarker::new(
                numbering,
                shared.get(ParLine::number_align),
                shared.get(ParLine::number_margin),
                // Delay resolving the number clearance until line numbers are
                // laid out to avoid inconsistent spacing depending on varying
                // font size.
                shared.get(ParLine::number_clearance),
            ))
        }),
        align: shared.get(AlignElem::alignment).fix(dir).x,
        font_size,
        dir,
        hyphenate: shared_get(children, shared, |s| s.get(TextElem::hyphenate))
            .map(|uniform| uniform.unwrap_or(justify)),
        lang: shared_get(children, shared, |s| s.get(TextElem::lang)),
        fallback: shared.get(TextElem::fallback),
        cjk_latin_spacing: shared.get(TextElem::cjk_latin_spacing).is_auto(),
        costs: shared.get(TextElem::costs),
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

/// Result of measuring a paragraph without creating frames.
#[derive(Debug, Clone)]
pub struct ParMeasureResult {
    /// Per-line metrics for exclusion zone calculations.
    pub line_metrics: Vec<LineMetrics>,
    /// Total height including leading between lines.
    pub total_height: Abs,
    /// Break information for reconstructing lines during commit.
    pub break_info: Vec<BreakInfo>,
}

/// Metrics for a single line.
#[derive(Debug, Clone, Copy)]
pub struct LineMetrics {
    /// Natural width of the line content.
    pub width: Abs,
    /// Total height (ascent + descent).
    pub height: Abs,
    /// Distance from top of line to baseline.
    pub ascent: Abs,
    /// Distance from baseline to bottom of line.
    pub descent: Abs,
}

/// Information needed to reconstruct a line break.
#[derive(Debug, Clone, Copy)]
pub struct BreakInfo {
    /// End byte offset of this line in the paragraph text.
    pub end: usize,
    /// The breakpoint type (normal, mandatory, or hyphen).
    pub breakpoint: Breakpoint,
}

/// Result of committing a measured paragraph to frames.
pub struct ParCommitResult {
    /// One frame per line, ready for positioning.
    pub frames: Vec<Frame>,
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
    /// Settings for justification.
    justification_limits: JustificationLimits,
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
    styles.get(ListElem::depth).0 > 0
        || !styles.get_cloned(EnumElem::parents).is_empty()
        || styles.get(TermsElem::within)
}
