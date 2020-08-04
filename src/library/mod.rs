//! The standard library.

mod align;
mod boxed;
mod font;
mod page;
mod spacing;
mod val;

pub use align::*;
pub use boxed::*;
pub use font::*;
pub use page::*;
pub use spacing::*;
pub use val::*;

use crate::func::prelude::*;
use crate::syntax::scope::Scope;

/// Create a scope with all standard library functions.
pub fn _std() -> Scope {
    let mut std = Scope::new(Box::new(val));

    std.insert("val", Box::new(val));
    std.insert("font", Box::new(font));
    std.insert("page", Box::new(page));
    std.insert("align", Box::new(align));
    std.insert("box", Box::new(boxed));
    std.insert("pagebreak", Box::new(pagebreak));
    std.insert("h", Box::new(h));
    std.insert("v", Box::new(v));

    std
}
