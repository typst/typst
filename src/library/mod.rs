//! The standard library.

mod align;
mod boxed;
mod color;
mod font;
mod graphics;
mod page;
mod spacing;

pub use align::*;
pub use boxed::*;
pub use color::*;
pub use font::*;
pub use graphics::*;
pub use page::*;
pub use spacing::*;

use crate::eval::{Scope, ValueFunc};

macro_rules! std {
    ($($func:expr $(=> $name:expr)?),* $(,)?) => {
        /// Create a scope with all standard library functions.
        pub fn _std() -> Scope {
            let mut std = Scope::new();
            $(
                let _name = stringify!($func);
                $(let _name = $name;)?
                std.set(_name, ValueFunc::new($func));
            )*
            std
        }
    };
}

std! {
    align,
    boxed => "box",
    font,
    h,
    image,
    page,
    pagebreak,
    rgb,
    v,
}
