use ttf_parser::math::MathValue;
use typst_library::foundations::{Style, StyleChain};
use typst_library::layout::{
    Abs, Em, FixedAlignment, Fr, Frame, Point, Size, VAlignment,
};
use typst_library::math::{EquationElem, GapSizing, MathSize};
use typst_utils::LazyHash;

use super::{EquationSizings, LeftRightAlternator, MathContext, MathFragment, MathRun};

macro_rules! scaled {
    ($ctx:expr, $styles:expr, text: $text:ident, display: $display:ident $(,)?) => {
        match typst_library::math::EquationElem::size_in($styles) {
            typst_library::math::MathSize::Display => scaled!($ctx, $styles, $display),
            _ => scaled!($ctx, $styles, $text),
        }
    };
    ($ctx:expr, $styles:expr, $name:ident) => {
        $crate::math::Scaled::scaled(
            $ctx.constants.$name(),
            $ctx,
            typst_library::text::TextElem::size_in($styles),
        )
    };
}

macro_rules! percent {
    ($ctx:expr, $name:ident) => {
        $ctx.constants.$name() as f64 / 100.0
    };
}

/// How much less high scaled delimiters can be than what they wrap.
pub const DELIM_SHORT_FALL: Em = Em::new(0.1);

/// Converts some unit to an absolute length with the current font & font size.
pub trait Scaled {
    fn scaled(self, ctx: &MathContext, font_size: Abs) -> Abs;
}

impl Scaled for i16 {
    fn scaled(self, ctx: &MathContext, font_size: Abs) -> Abs {
        ctx.font.to_em(self).at(font_size)
    }
}

impl Scaled for u16 {
    fn scaled(self, ctx: &MathContext, font_size: Abs) -> Abs {
        ctx.font.to_em(self).at(font_size)
    }
}

impl Scaled for MathValue<'_> {
    fn scaled(self, ctx: &MathContext, font_size: Abs) -> Abs {
        self.value.scaled(ctx, font_size)
    }
}

/// Styles something as cramped.
pub fn style_cramped() -> LazyHash<Style> {
    EquationElem::set_cramped(true).wrap()
}

/// The style for subscripts in the current style.
pub fn style_for_subscript(styles: StyleChain) -> [LazyHash<Style>; 2] {
    [style_for_superscript(styles), EquationElem::set_cramped(true).wrap()]
}

/// The style for superscripts in the current style.
pub fn style_for_superscript(styles: StyleChain) -> LazyHash<Style> {
    EquationElem::set_size(match EquationElem::size_in(styles) {
        MathSize::Display | MathSize::Text => MathSize::Script,
        MathSize::Script | MathSize::ScriptScript => MathSize::ScriptScript,
    })
    .wrap()
}

/// The style for numerators in the current style.
pub fn style_for_numerator(styles: StyleChain) -> LazyHash<Style> {
    EquationElem::set_size(match EquationElem::size_in(styles) {
        MathSize::Display => MathSize::Text,
        MathSize::Text => MathSize::Script,
        MathSize::Script | MathSize::ScriptScript => MathSize::ScriptScript,
    })
    .wrap()
}

/// The style for denominators in the current style.
pub fn style_for_denominator(styles: StyleChain) -> [LazyHash<Style>; 2] {
    [style_for_numerator(styles), EquationElem::set_cramped(true).wrap()]
}

/// Styles to add font constants to the style chain.
pub fn style_for_script_scale(ctx: &MathContext) -> LazyHash<Style> {
    EquationElem::set_script_scale((
        ctx.constants.script_percent_scale_down(),
        ctx.constants.script_script_percent_scale_down(),
    ))
    .wrap()
}

/// How a delimieter should be aligned when scaling.
pub fn delimiter_alignment(delimiter: char) -> VAlignment {
    match delimiter {
        '⌜' | '⌝' => VAlignment::Top,
        '⌞' | '⌟' => VAlignment::Bottom,
        _ => VAlignment::Horizon,
    }
}

/// Stack rows on top of each other.
///
/// Add a `gap` between each row and uses the baseline of the `baseline`-th
/// row for the whole frame. `alternator` controls the left/right alternating
/// alignment behavior of `AlignPointElem` in the rows.
pub fn stack(
    rows: Vec<MathRun>,
    align: FixedAlignment,
    gap: Abs,
    baseline: usize,
    alternator: LeftRightAlternator,
) -> Frame {
    let AlignmentResult { points, width, .. } = alignments(&rows, None);
    let rows: Vec<_> = rows
        .into_iter()
        .map(|row| row.into_line_frame(&points, alternator))
        .collect();

    let mut frame = Frame::soft(Size::new(
        width,
        rows.iter().map(|row| row.height()).sum::<Abs>()
            + rows.len().saturating_sub(1) as f64 * gap,
    ));

    let mut y = Abs::zero();
    for (i, row) in rows.into_iter().enumerate() {
        let x = if points.is_empty() {
            align.position(width - row.width())
        } else {
            Abs::zero()
        };
        let pos = Point::new(x, y);
        if i == baseline {
            frame.set_baseline(y + row.baseline());
        }
        y += row.height() + gap;
        frame.push_frame(pos, row);
    }

    frame
}

pub struct AlignmentResult {
    pub points: Vec<Abs>,
    pub width: Abs,
    pub padding: (Abs, Abs),
}

/// Determine the positions of the alignment points, according to the input
/// rows combined.
pub fn alignments(rows: &[MathRun], sizings: Option<EquationSizings>) -> AlignmentResult {
    let mut widths = Vec::<Abs>::new();

    let mut pending_width = Abs::zero();
    for row in rows {
        let mut width = Abs::zero();
        let mut alignment_index = 0;

        for fragment in row.iter() {
            if matches!(fragment, MathFragment::Align) {
                if alignment_index < widths.len() {
                    widths[alignment_index].set_max(width);
                } else {
                    widths.push(width.max(pending_width));
                }
                width = Abs::zero();
                alignment_index += 1;
            } else {
                width += fragment.width();
            }
        }
        if widths.is_empty() {
            pending_width.set_max(width);
        } else if alignment_index < widths.len() {
            widths[alignment_index].set_max(width);
        } else {
            widths.push(width.max(pending_width));
        }
    }

    if widths.is_empty() {
        widths.push(pending_width);
        let padding = add_gaps(&mut widths, sizings);
        return AlignmentResult { width: pending_width, points: vec![], padding };
    }

    let padding = add_gaps(&mut widths, sizings);
    let mut points = widths;
    for i in 1..points.len() {
        let prev = points[i - 1];
        points[i] += prev;
    }
    AlignmentResult {
        width: points.last().copied().unwrap(),
        points,
        padding,
    }
}

/// Inserts gaps between columns given by the alignments.
fn add_gaps(widths: &mut [Abs], sizings: Option<EquationSizings>) -> (Abs, Abs) {
    let Some(sizings) = sizings else {
        return (Abs::zero(), Abs::zero());
    };

    // Padding to be returned.
    let mut padding = [Abs::zero(), Abs::zero()];

    // Number of gaps between columns.
    let len = widths.len();
    let ngaps = len.div_ceil(2).saturating_sub(1);

    // Discard excess gaps or repeat the last gap to match the number of gaps.
    let mut gaps = sizings.gaps.to_vec();
    gaps.truncate(ngaps);
    if let Some(last_gap) = gaps.last().copied() {
        gaps.extend(std::iter::repeat_n(last_gap, ngaps.saturating_sub(gaps.len())));
    }

    // Sum of fractions of all fractional gaps.
    let mut fr = Fr::zero();

    // Resolve the size of all relative gaps and compute the sum of all
    // fractional gaps.
    let region_width = sizings.region_size_x;
    for (i, gap) in gaps.iter().enumerate() {
        match gap {
            GapSizing::Rel(v) => widths[1 + i * 2] += v.relative_to(region_width),
            GapSizing::Fr(v) => fr += *v,
        }
    }
    for (i, gap) in sizings.padding.iter().enumerate() {
        match gap {
            GapSizing::Rel(v) => padding[i] = v.relative_to(region_width),
            GapSizing::Fr(v) => fr += *v,
        }
    }

    // Size that is not used by fixed-size gaps.
    let remaining = region_width - (widths.iter().sum::<Abs>() + padding.iter().sum());

    // Distribute remaining space to fractional gaps.
    if !remaining.approx_empty() {
        for (i, gap) in gaps.iter().enumerate() {
            if let GapSizing::Fr(v) = gap {
                widths[1 + i * 2] += v.share(fr, remaining);
            }
        }
        for (i, gap) in sizings.padding.iter().enumerate() {
            if let GapSizing::Fr(v) = gap {
                padding[i] = v.share(fr, remaining);
            }
        }
    }

    (padding[0], padding[1])
}
