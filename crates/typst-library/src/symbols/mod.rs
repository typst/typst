//! Modifiable symbols.

mod emoji;
mod sym;

pub use self::emoji::*;
pub use self::sym::*;

use crate::foundations::{category, Category, Scope};

/// These two modules give names to symbols and emoji to make them easy to
/// insert with a normal keyboard. Alternatively, you can also always directly
/// enter Unicode symbols into your text and formulas. In addition to the
/// symbols listed below, math mode defines `dif` and `Dif`. These are not
/// normal symbol values because they also affect spacing and font style.
#[category]
pub static SYMBOLS: Category;

/// Hook up all `symbol` definitions.
pub(super) fn define(global: &mut Scope) {
    global.category(SYMBOLS);
    global.define_module(sym());
    global.define_module(emoji());
}
