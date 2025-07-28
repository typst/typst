use comemo::Tracked;
use ttf_parser::Tag;
use ttf_parser::math::MathValue;
use typst_library::World;
use typst_library::diag::{SourceResult, bail};
use typst_library::foundations::{Style, StyleChain};
use typst_library::layout::{Abs, Em, FixedAlignment, Frame, Point, Size};
use typst_library::math::{EquationElem, MathSize};
use typst_library::text::{Font, FontFeatures, FontFlags, TextElem, families, variant};
use typst_syntax::Span;
use typst_utils::LazyHash;

use super::{LeftRightAlternator, MathFragment, MathRun};

macro_rules! value {
    ($font:expr, $styles:expr, text: $text:ident, display: $display:ident $(,)?) => {
        match $styles.get(typst_library::math::EquationElem::size) {
            typst_library::math::MathSize::Display => value!($font, $display),
            _ => value!($font, $text),
        }
    };
    ($font:expr, $name:ident) => {
        $font
            .ttf()
            .tables()
            .math
            .and_then(|math| math.constants)
            .map(|constants| {
                crate::math::shared::Scaled::scaled(constants.$name(), &$font)
            })
            .unwrap()
    };
}

macro_rules! percent {
    ($font:expr, $name:ident) => {
        $font
            .ttf()
            .tables()
            .math
            .and_then(|math| math.constants)
            .map(|constants| constants.$name())
            .unwrap() as f64
            / 100.0
    };
}

/// How much less high scaled delimiters can be than what they wrap.
pub const DELIM_SHORT_FALL: Em = Em::new(0.1);

/// Converts some unit to an absolute length with the current font & font size.
pub trait Scaled {
    fn scaled(self, font: &Font) -> Em;
}

impl Scaled for i16 {
    fn scaled(self, font: &Font) -> Em {
        font.to_em(self)
    }
}

impl Scaled for u16 {
    fn scaled(self, font: &Font) -> Em {
        font.to_em(self)
    }
}

impl Scaled for MathValue<'_> {
    fn scaled(self, font: &Font) -> Em {
        self.value.scaled(font)
    }
}

/// Get the current math font.
#[comemo::memoize]
pub fn find_math_font(
    world: Tracked<dyn World + '_>,
    styles: StyleChain,
    span: Span,
) -> SourceResult<Font> {
    let variant = variant(styles);
    let Some(font) = families(styles).find_map(|family| {
        // Take the base font as the "main" math font.
        world
            .book()
            .select(family.as_str(), variant)
            .and_then(|id| world.font(id))
            .filter(|font| font.info().flags.contains(FontFlags::MATH))
            .filter(|_| family.covers().is_none())
    }) else {
        bail!(span, "current font does not support math");
    };
    Ok(font)
}

/// Styles something as cramped.
pub fn style_cramped() -> LazyHash<Style> {
    EquationElem::cramped.set(true).wrap()
}

/// Sets flac OpenType feature.
pub fn style_flac() -> LazyHash<Style> {
    TextElem::features
        .set(FontFeatures(vec![(Tag::from_bytes(b"flac"), 1)]))
        .wrap()
}

/// Sets dtls OpenType feature.
pub fn style_dtls() -> LazyHash<Style> {
    TextElem::features
        .set(FontFeatures(vec![(Tag::from_bytes(b"dtls"), 1)]))
        .wrap()
}

/// The style for subscripts in the current style.
pub fn style_for_subscript(styles: StyleChain) -> [LazyHash<Style>; 2] {
    [style_for_superscript(styles), EquationElem::cramped.set(true).wrap()]
}

/// The style for superscripts in the current style.
pub fn style_for_superscript(styles: StyleChain) -> LazyHash<Style> {
    EquationElem::size
        .set(match styles.get(EquationElem::size) {
            MathSize::Display | MathSize::Text => MathSize::Script,
            MathSize::Script | MathSize::ScriptScript => MathSize::ScriptScript,
        })
        .wrap()
}

/// The style for numerators in the current style.
pub fn style_for_numerator(styles: StyleChain) -> LazyHash<Style> {
    EquationElem::size
        .set(match styles.get(EquationElem::size) {
            MathSize::Display => MathSize::Text,
            MathSize::Text => MathSize::Script,
            MathSize::Script | MathSize::ScriptScript => MathSize::ScriptScript,
        })
        .wrap()
}

/// The style for denominators in the current style.
pub fn style_for_denominator(styles: StyleChain) -> [LazyHash<Style>; 2] {
    [style_for_numerator(styles), EquationElem::cramped.set(true).wrap()]
}

/// Styles to add font constants to the style chain.
pub fn style_for_script_scale(font: &Font) -> LazyHash<Style> {
    let constants = font.ttf().tables().math.and_then(|math| math.constants).unwrap();
    EquationElem::script_scale
        .set((
            constants.script_percent_scale_down(),
            constants.script_script_percent_scale_down(),
        ))
        .wrap()
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
    let AlignmentResult { points, width } = alignments(&rows);
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
