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
    global.define("image", ImageElem::func());
    global.define("line", LineElem::func());
    global.define("rect", RectElem::func());
    global.define("square", SquareElem::func());
    global.define("ellipse", EllipseElem::func());
    global.define("circle", CircleElem::func());
    global.define("polygon", PolygonElem::func());
    global.define("path", PathElem::func());
}
