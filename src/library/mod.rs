//! The standard library.

mod insert;
mod layout;
mod style;

pub use insert::*;
pub use layout::*;
pub use style::*;

use crate::eval::{Scope, ValueFunc};

macro_rules! std {
    ($($func:expr $(=> $name:expr)?),* $(,)?) => {
        /// The scope containing all standard library functions.
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
