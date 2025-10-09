//! Basic utilities for converting Typst types to krilla.

use krilla::geom as kg;
use krilla::geom::PathBuilder;
use krilla::paint as kp;
use krilla::tagging as kt;
use typst_library::layout::{Abs, Point, Sides, Size, Transform};
use typst_library::text::Font;
use typst_library::visualize::{Curve, CurveItem, FillRule, LineCap, LineJoin};

pub(crate) trait SidesExt<T> {
    /// Map to the [`kt::Sides`] struct assuming [`kt::WritingMode::LrTb`].
    fn to_lrtb_krilla(self) -> kt::Sides<T>;
}

impl<T> SidesExt<T> for Sides<T> {
    fn to_lrtb_krilla(self) -> kt::Sides<T> {
        kt::Sides {
            before: self.top,
            after: self.bottom,
            start: self.left,
            end: self.right,
        }
    }
}

pub(crate) trait SizeExt {
    fn to_krilla(&self) -> Option<kg::Size>;
}

impl SizeExt for Size {
    fn to_krilla(&self) -> Option<kg::Size> {
        kg::Size::from_wh(self.x.to_f32(), self.y.to_f32())
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

/// Display the font family of a font.
pub(crate) fn display_font(font: &Font) -> &str {
    &font.info().family
}

/// Convert a Typst path to a krilla path.
pub(crate) fn convert_path(path: &Curve, builder: &mut PathBuilder) {
    for item in &path.0 {
        match item {
            CurveItem::Move(p) => builder.move_to(p.x.to_f32(), p.y.to_f32()),
            CurveItem::Line(p) => builder.line_to(p.x.to_f32(), p.y.to_f32()),
            CurveItem::Cubic(p1, p2, p3) => builder.cubic_to(
                p1.x.to_f32(),
                p1.y.to_f32(),
                p2.x.to_f32(),
                p2.y.to_f32(),
                p3.x.to_f32(),
                p3.y.to_f32(),
            ),
            CurveItem::Close => builder.close(),
        }
    }
}
