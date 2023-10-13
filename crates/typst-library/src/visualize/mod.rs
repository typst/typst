//! Drawing and visualization.

mod image;
mod line;
mod path;
mod polygon;
mod shape;

pub use self::image::*;
pub use self::line::*;
pub use self::path::*;
pub use self::polygon::*;
pub use self::shape::*;

use crate::prelude::*;

/// Hook up all visualize definitions.
pub(super) fn define(global: &mut Scope) {
    global.category("visualize");
    global.define_type::<Color>();
    global.define_type::<Gradient>();
    global.define_type::<Stroke>();
    global.define_elem::<ImageElem>();
    global.define_elem::<LineElem>();
    global.define_elem::<RectElem>();
    global.define_elem::<SquareElem>();
    global.define_elem::<EllipseElem>();
    global.define_elem::<CircleElem>();
    global.define_elem::<PolygonElem>();
    global.define_elem::<PathElem>();
}
