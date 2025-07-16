//! Drawing and visualization.

mod color;
mod curve;
mod gradient;
mod image;
mod line;
mod paint;
mod path;
mod polygon;
mod shape;
mod stroke;
mod tiling;

pub use self::color::*;
pub use self::curve::*;
pub use self::gradient::*;
pub use self::image::*;
pub use self::line::*;
pub use self::paint::*;
pub use self::path::*;
pub use self::polygon::*;
pub use self::shape::*;
pub use self::stroke::*;
pub use self::tiling::*;

use crate::foundations::{Element, Scope, Type};

/// Hook up all visualize definitions.
pub(super) fn define(global: &mut Scope) {
    global.start_category(crate::Category::Visualize);
    global.define_type::<Color>();
    global.define_type::<Gradient>();
    global.define_type::<Tiling>();
    global.define_type::<Stroke>();
    global.define_elem::<ImageElem>();
    global.define_elem::<LineElem>();
    global.define_elem::<RectElem>();
    global.define_elem::<SquareElem>();
    global.define_elem::<EllipseElem>();
    global.define_elem::<CircleElem>();
    global.define_elem::<PolygonElem>();
    global.define_elem::<CurveElem>();
    global
        .define("path", Element::of::<PathElem>())
        .deprecated("the `path` function is deprecated, use `curve` instead");
    global
        .define("pattern", Type::of::<Tiling>())
        .deprecated("the name `pattern` is deprecated, use `tiling` instead")
        .deprecated_until("0.15.0");
    global.reset_category();
}
