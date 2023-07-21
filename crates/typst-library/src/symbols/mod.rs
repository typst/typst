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

/// Represents a symbol as `Content`.  When laid out in math it
/// is converted into a `VarElem`. When laid out in other contexts
/// it is converted to a `TextElem`.
///
/// Display:  Symbol
/// Category: symbols
#[element]
pub struct SymbolElem {
    /// The symbol.
    #[required]
    pub character: char,
}
