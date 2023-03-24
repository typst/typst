//! Modifiable symbols.

mod emoji;
mod sym;

pub use emoji::emoji;
pub use sym::sym;
pub(crate) use sym::SYM;
use typst::eval::Scope;

pub(super) fn define(scope: &mut Scope) {
    scope.define("sym", sym());
    scope.define("emoji", emoji());
}
