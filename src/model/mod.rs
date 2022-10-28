//! Layout and computation model.

#[macro_use]
mod styles;
mod collapse;
mod content;
mod eval;
mod layout;
mod property;
mod recipe;
mod show;
#[macro_use]
mod cast;
#[macro_use]
mod array;
#[macro_use]
mod dict;
#[macro_use]
mod str;
#[macro_use]
mod value;
mod args;
mod capture;
mod fold;
mod func;
pub mod methods;
pub mod ops;
mod raw;
mod realize;
mod resolve;
mod scope;
mod vm;

pub use self::str::*;
pub use args::*;
pub use array::*;
pub use capture::*;
pub use cast::*;
pub use collapse::*;
pub use content::*;
pub use dict::*;
pub use eval::*;
pub use fold::*;
pub use func::*;
pub use layout::*;
pub use property::*;
pub use raw::*;
pub use recipe::*;
pub use resolve::*;
pub use scope::*;
pub use show::*;
pub use styles::*;
pub use typst_macros::node;
pub use value::*;
pub use vm::*;

use realize::*;
