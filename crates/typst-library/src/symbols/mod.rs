//! Modifiable symbols.

mod emoji;
mod sym;

pub use emoji::*;
pub use sym::*;

use crate::prelude::*;

/// Hook up all symbol definitions.
pub(super) fn define(global: &mut Scope) {
    global.category("symbols");
    global.define_type::<Symbol>();
    global.define_module(sym());
    global.define_module(emoji());
}
