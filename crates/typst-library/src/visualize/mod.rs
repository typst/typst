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
    global.define("black", Color::BLACK);
    global.define("gray", Color::GRAY);
    global.define("silver", Color::SILVER);
    global.define("white", Color::WHITE);
    global.define("navy", Color::NAVY);
    global.define("blue", Color::BLUE);
    global.define("aqua", Color::AQUA);
    global.define("teal", Color::TEAL);
    global.define("eastern", Color::EASTERN);
    global.define("purple", Color::PURPLE);
    global.define("fuchsia", Color::FUCHSIA);
    global.define("maroon", Color::MAROON);
    global.define("red", Color::RED);
    global.define("orange", Color::ORANGE);
    global.define("yellow", Color::YELLOW);
    global.define("olive", Color::OLIVE);
    global.define("green", Color::GREEN);
    global.define("lime", Color::LIME);
}
