//! Convert paint types from typst to krilla.

use krilla::geom::NormalizedF32;
use typst_library::layout::Abs;
use typst_library::visualize::{ColorSpace, DashPattern, FillRule, FixedStroke, Paint};

use crate::primitive::{linecap, linejoin};
use crate::AbsExt;

pub(crate) fn fill(paint_: &Paint, fill_rule_: FillRule) -> krilla::path::Fill {
    let (paint, opacity) = paint(paint_);

    krilla::path::Fill {
        paint,
        rule: fill_rule(fill_rule_),
        opacity: NormalizedF32::new(opacity as f32 / 255.0).unwrap(),
    }
}

pub(crate) fn stroke(stroke: &FixedStroke) -> krilla::path::Stroke {
    let (paint, opacity) = paint(&stroke.paint);
    krilla::path::Stroke {
        paint,
        width: stroke.thickness.to_f32(),
        miter_limit: stroke.miter_limit.get() as f32,
        line_join: linejoin(stroke.join),
        line_cap: linecap(stroke.cap),
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

fn fill_rule(fill_rule: FillRule) -> krilla::path::FillRule {
    match fill_rule {
        FillRule::NonZero => krilla::path::FillRule::NonZero,
        FillRule::EvenOdd => krilla::path::FillRule::EvenOdd,
    }
}
