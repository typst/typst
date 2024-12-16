use ttf_parser::math::MathValue;
use typst_library::foundations::{Style, StyleChain};
use typst_library::layout::{Abs, Em, FixedAlignment, Frame, Point, Size, VAlignment};
use typst_library::math::{EquationElem, MathSize};
use typst_library::text::TextElem;
use typst_utils::LazyHash;

use super::{LeftRightAlternator, MathContext, MathFragment, MathRun};

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
            $crate::math::scaled_font_size($ctx, $styles),
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

/// Get the font size scaled with the `MathSize`.
pub fn scaled_font_size(ctx: &MathContext, styles: StyleChain) -> Abs {
    let factor = match EquationElem::size_in(styles) {
        MathSize::Display | MathSize::Text => 1.0,
        MathSize::Script => percent!(ctx, script_percent_scale_down),
        MathSize::ScriptScript => percent!(ctx, script_script_percent_scale_down),
    };
    factor * TextElem::size_in(styles)
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
    minimum_ascent_descent: Option<(Abs, Abs)>,
) -> Frame {
    let AlignmentResult { points, width } = alignments(&rows);
    let rows: Vec<_> = rows
        .into_iter()
        .map(|row| row.into_line_frame(&points, alternator))
        .collect();

    let padded_height = |height: Abs| {
        height.max(minimum_ascent_descent.map_or(Abs::zero(), |(a, d)| a + d))
    };

    let mut frame = Frame::soft(Size::new(
        width,
        rows.iter().map(|row| padded_height(row.height())).sum::<Abs>()
            + rows.len().saturating_sub(1) as f64 * gap,
    ));

    let mut y = Abs::zero();
    for (i, row) in rows.into_iter().enumerate() {
        let x = if points.is_empty() {
            align.position(width - row.width())
        } else {
            Abs::zero()
        };
        let ascent_padded_part = minimum_ascent_descent
            .map_or(Abs::zero(), |(a, _)| (a - row.ascent()))
            .max(Abs::zero());
        let pos = Point::new(x, y + ascent_padded_part);
        if i == baseline {
            frame.set_baseline(y + row.baseline() + ascent_padded_part);
        }
        y += padded_height(row.height()) + gap;
        frame.push_frame(pos, row);
    }

    frame
}

/// Determine the positions of the alignment points, according to the input rows combined.
pub fn alignments(rows: &[MathRun]) -> AlignmentResult {
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

    let mut points = widths;
    for i in 1..points.len() {
        let prev = points[i - 1];
        points[i] += prev;
    }
    AlignmentResult {
        width: points.last().copied().unwrap_or(pending_width),
        points,
    }
}

pub struct AlignmentResult {
    pub points: Vec<Abs>,
    pub width: Abs,
}
