//! Convert basic primitive types from typst to krilla.

use typst_library::layout::{Point, Size, Transform};
use typst_library::visualize::{LineCap, LineJoin};

use crate::AbsExt;

pub(crate) fn size(s: Size) -> krilla::geom::Size {
    krilla::geom::Size::from_wh(s.x.to_f32(), s.y.to_f32()).unwrap()
}

pub(crate) fn point(p: Point) -> krilla::geom::Point {
    krilla::geom::Point::from_xy(p.x.to_f32(), p.y.to_f32())
}

pub(crate) fn linecap(l: LineCap) -> krilla::path::LineCap {
    match l {
        LineCap::Butt => krilla::path::LineCap::Butt,
        LineCap::Round => krilla::path::LineCap::Round,
        LineCap::Square => krilla::path::LineCap::Square,
    }
}

pub(crate) fn linejoin(l: LineJoin) -> krilla::path::LineJoin {
    match l {
        LineJoin::Miter => krilla::path::LineJoin::Miter,
        LineJoin::Round => krilla::path::LineJoin::Round,
        LineJoin::Bevel => krilla::path::LineJoin::Bevel,
    }
}

pub(crate) fn transform(t: Transform) -> krilla::geom::Transform {
    krilla::geom::Transform::from_row(
        t.sx.get() as f32,
        t.ky.get() as f32,
        t.kx.get() as f32,
        t.sy.get() as f32,
        t.tx.to_f32(),
        t.ty.to_f32(),
    )
}
