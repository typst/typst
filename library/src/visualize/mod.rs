//! Drawing and visualization.

mod image;
mod line;
mod shape;

use typst::eval::Scope;
use typst::model::Element as _;

pub use self::image::{ImageElem, ImageFit};
pub use self::line::LineElem;
pub use self::shape::{CircleElem, EllipseElem, RectElem, ShapeKind, SquareElem};

pub(super) fn define(scope: &mut Scope) {
    scope.define("image", ImageElem::func());
    scope.define("line", LineElem::func());
    scope.define("rect", RectElem::func());
    scope.define("square", SquareElem::func());
    scope.define("ellipse", EllipseElem::func());
    scope.define("circle", CircleElem::func());
}
