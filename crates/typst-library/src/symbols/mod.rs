//! Modifiable symbols.

mod emoji;
mod sym;

pub use emoji::*;
pub use sym::*;

use crate::prelude::*;

/// Hook up all symbol definitions.
pub(super) fn define(global: &mut Scope) {
    global.define("sym", sym());
    global.define("emoji", emoji());
}
