//! The standard library.

mod align;
mod boxed;
mod color;
mod font;
mod page;
mod spacing;

pub use align::*;
pub use boxed::*;
pub use color::*;
pub use font::*;
pub use page::*;
pub use spacing::*;

use crate::eval::{Scope, ValueFunc};

macro_rules! std {
    ($($name:literal => $func:expr),* $(,)?) => {
        /// Create a scope with all standard library functions.
        pub fn _std() -> Scope {
            let mut std = Scope::new();
            $(std.set($name, ValueFunc::new($func));)*
            std
        }
    };
}

std! {
    "align" => align,
    "box" => boxed,
    "font" => font,
    "h" => h,
    "page" => page,
    "pagebreak" => pagebreak,
    "rgb" => rgb,
    "v" => v,
}
