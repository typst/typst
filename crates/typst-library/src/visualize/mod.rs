//! Drawing and visualization.

mod color;
mod curve;
mod gradient;
mod image;
mod line;
mod paint;
mod path;
mod pattern;
mod polygon;
mod shape;
mod stroke;

pub use self::color::*;
pub use self::curve::*;
pub use self::gradient::*;
pub use self::image::*;
pub use self::line::*;
pub use self::paint::*;
pub use self::path::*;
pub use self::pattern::*;
pub use self::polygon::*;
pub use self::shape::*;
pub use self::stroke::*;

use crate::foundations::{category, Category, Scope};

/// Drawing and data visualization.
///
/// If you want to create more advanced drawings or plots, also have a look at
/// the [CetZ](https://github.com/johannes-wolf/cetz) package as well as more
/// specialized [packages]($universe) for your use case.
#[category]
pub static VISUALIZE: Category;

/// Hook up all visualize definitions.
pub(super) fn define(global: &mut Scope) {
    global.category(VISUALIZE);
    global.define_type::<Color>();
    global.define_type::<Gradient>();
    global.define_type::<Pattern>();
    global.define_type::<Stroke>();
    global.define_elem::<ImageElem>();
    global.define_elem::<LineElem>();
    global.define_elem::<RectElem>();
    global.define_elem::<SquareElem>();
    global.define_elem::<EllipseElem>();
    global.define_elem::<CircleElem>();
    global.define_elem::<PolygonElem>();
    global.define_elem::<CurveElem>();
    global.define_elem::<PathElem>();
}
