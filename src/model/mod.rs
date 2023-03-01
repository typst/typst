//! The document model.

#[macro_use]
mod styles;
mod content;
mod realize;
mod typeset;

pub use self::content::*;
pub use self::realize::*;
pub use self::styles::*;
pub use self::typeset::*;

#[doc(hidden)]
pub use once_cell;
pub use typst_macros::{capability, capable, node};
