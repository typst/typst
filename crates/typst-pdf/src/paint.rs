//! Convert paint types from typst to krilla.

use std::num::NonZeroUsize;
use krilla::geom::NormalizedF32;
use krilla::page::{NumberingStyle, PageLabel};
use typst_library::layout::{Abs, Angle, Quadrant, Ratio, Transform};
use typst_library::model::Numbering;
use typst_library::visualize::{ColorSpace, DashPattern, FillRule, FixedStroke, Gradient, Paint, RelativeTo};
use typst_utils::Numeric;
use crate::{content_old, AbsExt};
use crate::content_old::Transforms;
use crate::gradient_old::PdfGradient;
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

// TODO: Anti-aliasing

fn convert_gradient(
    gradient: &Gradient,
    on_text: bool,
    mut transforms: Transforms,
) -> usize {
    // Edge cases for strokes.
    if transforms.size.x.is_zero() {
        transforms.size.x = Abs::pt(1.0);
    }

    if transforms.size.y.is_zero() {
        transforms.size.y = Abs::pt(1.0);
    }
    let size = match gradient.unwrap_relative(on_text) {
        RelativeTo::Self_ => transforms.size,
        RelativeTo::Parent => transforms.container_size,
    };

    let rotation = gradient.angle().unwrap_or_else(Angle::zero);

    let transform = match gradient.unwrap_relative(on_text) {
        RelativeTo::Self_ => transforms.transform,
        RelativeTo::Parent => transforms.container_transform,
    };

    let scale_offset = match gradient {
        Gradient::Conic(_) => 4.0_f64,
        _ => 1.0,
    };

    let transform = transform
        .pre_concat(Transform::translate(
            offset_x * scale_offset,
            offset_y * scale_offset,
        ))
        .pre_concat(Transform::scale(
            Ratio::new(size.x.to_pt() * scale_offset),
            Ratio::new(size.y.to_pt() * scale_offset),
        ));

    let angle = Gradient::correct_aspect_ratio(rotation, size.aspect_ratio());

    match &gradient {
        Gradient::Linear(_) => {
            let (mut sin, mut cos) = (angle.sin(), angle.cos());

            // Scale to edges of unit square.
            let factor = cos.abs() + sin.abs();
            sin *= factor;
            cos *= factor;

            let (x1, y1, x2, y2): (f64, f64, f64, f64) = match angle.quadrant() {
                Quadrant::First => (0.0, 0.0, cos, sin),
                Quadrant::Second => (1.0, 0.0, cos + 1.0, sin),
                Quadrant::Third => (1.0, 1.0, cos + 1.0, sin + 1.0),
                Quadrant::Fourth => (0.0, 1.0, cos, sin + 1.0),
            };
        }
        Gradient::Radial(_) => {}
        Gradient::Conic(_) => {}
    }
}