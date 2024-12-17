//! Basic utilities for converting typst types to krilla.

use krilla::geom as kg;
use krilla::path as kp;
use krilla::color::rgb as kr;

use typst_library::layout::{Abs, Point, Size, Transform};
use typst_library::text::Font;
use typst_library::visualize::{Color, ColorSpace, FillRule, LineCap, LineJoin};

pub(crate) trait SizeExt {
    fn to_krilla(&self) -> kg::Size;
}

impl SizeExt for Size {
    fn to_krilla(&self) -> kg::Size {
        kg::Size::from_wh(self.x.to_f32(), self.y.to_f32()).unwrap()
    }
}

pub(crate) trait PointExt {
    fn to_krilla(&self) -> kg::Point;
}

impl PointExt for Point {
    fn to_krilla(&self) -> kg::Point {
        kg::Point::from_xy(self.x.to_f32(), self.y.to_f32())
    }
}

pub(crate) trait LineCapExt {
    fn to_krilla(&self) -> kp::LineCap;
}

impl LineCapExt for LineCap {
    fn to_krilla(&self) -> kp::LineCap {
        match self {
            LineCap::Butt => kp::LineCap::Butt,
            LineCap::Round => kp::LineCap::Round,
            LineCap::Square => kp::LineCap::Square,
        }
    }
}

pub(crate) trait LineJoinExt {
    fn to_krilla(&self) -> kp::LineJoin;
}

impl LineJoinExt for LineJoin {
    fn to_krilla(&self) -> kp::LineJoin {
        match self {
            LineJoin::Miter => kp::LineJoin::Miter,
            LineJoin::Round => kp::LineJoin::Round,
            LineJoin::Bevel => kp::LineJoin::Bevel,
        }
    }
}

pub(crate) trait TransformExt {
    fn to_krilla(&self) -> kg::Transform;
}

impl TransformExt for Transform {
    fn to_krilla(&self) -> kg::Transform {
        kg::Transform::from_row(
            self.sx.get() as f32,
            self.ky.get() as f32,
            self.kx.get() as f32,
            self.sy.get() as f32,
            self.tx.to_f32(),
            self.ty.to_f32(),
        )
    }
}

pub(crate) trait FillRuleExt {
    fn to_krilla(&self) -> kp::FillRule;
}

impl FillRuleExt for FillRule {
    fn to_krilla(&self) -> kp::FillRule {
        match self {
            FillRule::NonZero => kp::FillRule::NonZero,
            FillRule::EvenOdd => kp::FillRule::EvenOdd,
        }
    }
}

pub(crate) trait AbsExt {
    fn to_f32(self) -> f32;
}

impl AbsExt for Abs {
    fn to_f32(self) -> f32 {
        self.to_pt() as f32
    }
}

pub(crate) trait ColorExt {
    fn to_krilla_rgb(&self) -> (kr::Color, u8);
}

impl ColorExt for Color {
    /// Convert a color into a krilla RGB color and an alpha value.
    fn to_krilla_rgb(&self) -> (kr::Color, u8) {
        let components = self.to_space(ColorSpace::Srgb).to_vec4_u8();
        (
            kr::Color::new(components[0], components[1], components[2])
                .into(),
            components[3],
        )
    }
}

/// Display the font family and variant.
pub(crate) fn display_font(font: &Font) -> String {
    let font_family = &font.info().family;
    let font_variant = font.info().variant;
    format!("{} ({:?})", font_family, font_variant)
}
