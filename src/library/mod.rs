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

use std::rc::Rc;

use crate::compute::scope::Scope;
use crate::prelude::*;

macro_rules! std {
    ($($name:literal => $func:expr),* $(,)?) => {
        /// Create a scope with all standard library functions.
        pub fn _std() -> Scope {
            let mut std = Scope::new();
            $(std.insert($name, wrap!($func));)*
            std
        }
    };
}

macro_rules! wrap {
    ($func:expr) => {
        Rc::new(|name, args, ctx| Box::pin($func(name, args, ctx)))
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
