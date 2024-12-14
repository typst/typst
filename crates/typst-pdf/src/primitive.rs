//! Convert basic primitive types from typst to krilla.

use typst_library::layout::{Point, Size, Transform};
use typst_library::visualize::{FillRule, LineCap, LineJoin};

use crate::AbsExt;

pub(crate) trait SizeExt {
    fn as_krilla(&self) -> krilla::geom::Size;
}

impl SizeExt for Size {
    fn as_krilla(&self) -> krilla::geom::Size {
        krilla::geom::Size::from_wh(self.x.to_f32(), self.y.to_f32()).unwrap()
    }
}

pub(crate) trait PointExt {
    fn as_krilla(&self) -> krilla::geom::Point;
}

impl PointExt for Point {
    fn as_krilla(&self) -> krilla::geom::Point {
        krilla::geom::Point::from_xy(self.x.to_f32(), self.y.to_f32())
    }
}

pub(crate) trait LineCapExt {
    fn as_krilla(&self) -> krilla::path::LineCap;
}

impl LineCapExt for LineCap {
    fn as_krilla(&self) -> krilla::path::LineCap {
        match self {
            LineCap::Butt => krilla::path::LineCap::Butt,
            LineCap::Round => krilla::path::LineCap::Round,
            LineCap::Square => krilla::path::LineCap::Square,
        }
    }
}

pub(crate) trait LineJoinExt {
    fn as_krilla(&self) -> krilla::path::LineJoin;
}

impl LineJoinExt for LineJoin {
    fn as_krilla(&self) -> krilla::path::LineJoin {
        match self {
            LineJoin::Miter => krilla::path::LineJoin::Miter,
            LineJoin::Round => krilla::path::LineJoin::Round,
            LineJoin::Bevel => krilla::path::LineJoin::Bevel,
        }
    }
}

pub(crate) trait TransformExt {
    fn as_krilla(&self) -> krilla::geom::Transform;
}

impl TransformExt for Transform {
    fn as_krilla(&self) -> krilla::geom::Transform {
        krilla::geom::Transform::from_row(
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
    fn as_krilla(&self) -> krilla::path::FillRule;
}

impl FillRuleExt for FillRule {
    fn as_krilla(&self) -> krilla::path::FillRule {
        match self {
            FillRule::NonZero => krilla::path::FillRule::NonZero,
            FillRule::EvenOdd => krilla::path::FillRule::EvenOdd,
        }
    }
}
