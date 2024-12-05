//! Convert paint types from typst to krilla.

use std::num::NonZeroUsize;
use krilla::geom::NormalizedF32;
use krilla::page::{NumberingStyle, PageLabel};
use typst_library::layout::Abs;
use typst_library::model::Numbering;
use typst_library::visualize::{ColorSpace, DashPattern, FillRule, FixedStroke, Paint};

use crate::AbsExt;
use crate::primitive::{FillRuleExt, LineCapExt, LineJoinExt};

pub(crate) fn fill(paint_: &Paint, fill_rule_: FillRule) -> krilla::path::Fill {
    let (paint, opacity) = paint(paint_);

    krilla::path::Fill {
        paint,
        rule: fill_rule_.as_krilla(),
        opacity: NormalizedF32::new(opacity as f32 / 255.0).unwrap(),
    }
}

pub(crate) fn stroke(stroke: &FixedStroke) -> krilla::path::Stroke {
    let (paint, opacity) = paint(&stroke.paint);
    krilla::path::Stroke {
        paint,
        width: stroke.thickness.to_f32(),
        miter_limit: stroke.miter_limit.get() as f32,
        line_join: stroke.join.as_krilla(),
        line_cap: stroke.cap.as_krilla(),
        opacity: NormalizedF32::new(opacity as f32 / 255.0).unwrap(),
        dash: stroke.dash.as_ref().map(|d| dash(d)),
    }
}

fn dash(dash: &DashPattern<Abs, Abs>) -> krilla::path::StrokeDash {
    krilla::path::StrokeDash {
        array: dash.array.iter().map(|e| e.to_f32()).collect(),
        offset: dash.phase.to_f32(),
    }
}

fn paint(paint: &Paint) -> (krilla::paint::Paint, u8) {
    match paint {
        Paint::Solid(c) => {
            let components = c.to_space(ColorSpace::Srgb).to_vec4_u8();
            (
                krilla::color::rgb::Color::new(
                    components[0],
                    components[1],
                    components[2],
                )
                .into(),
                components[3],
            )
        }
        Paint::Gradient(_) => (krilla::color::rgb::Color::black().into(), 255),
        Paint::Pattern(_) => (krilla::color::rgb::Color::black().into(), 255),
    }
}

pub(crate) trait PageLabelExt {
    fn generate(numbering: &Numbering, number: usize) -> Option<PageLabel>;
    fn arabic(number: usize) -> PageLabel;
}

impl PageLabelExt for PageLabel {
    /// Create a new `PageLabel` from a `Numbering` applied to a page
    /// number.
    fn generate(numbering: &Numbering, number: usize) -> Option<PageLabel> {
        {
            let Numbering::Pattern(pat) = numbering else {
                return None;
            };

            let (prefix, kind) = pat.pieces.first()?;

            // If there is a suffix, we cannot use the common style optimisation,
            // since PDF does not provide a suffix field.
            let style = if pat.suffix.is_empty() {
                use typst_library::model::NumberingKind as Kind;
                use krilla::page::NumberingStyle as Style;
                match kind {
                    Kind::Arabic => Some(Style::Arabic),
                    Kind::LowerRoman => Some(Style::LowerRoman),
                    Kind::UpperRoman => Some(Style::UpperRoman),
                    Kind::LowerLatin if number <= 26 => Some(Style::LowerAlpha),
                    Kind::LowerLatin if number <= 26 => Some(Style::UpperAlpha),
                    _ => None,
                }
            } else {
                None
            };

            // Prefix and offset depend on the style: If it is supported by the PDF
            // spec, we use the given prefix and an offset. Otherwise, everything
            // goes into prefix.
            let prefix = if style.is_none() {
                Some(pat.apply(&[number]))
            } else {
                (!prefix.is_empty()).then(|| prefix.clone())
            };

            let offset = style.and(NonZeroUsize::new(number));
            Some(PageLabel::new(style, prefix.map(|s| s.to_string()), offset))
        }
    }

    /// Creates an arabic page label with the specified page number.
    /// For example, this will display page label `11` when given the page
    /// number 11.
    fn arabic(number: usize) -> PageLabel {
        PageLabel::new(Some(NumberingStyle::Arabic), None, NonZeroUsize::new(number))
    }
}