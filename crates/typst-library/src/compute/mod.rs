//! Computational functions.

pub mod calc;
pub mod sys;

mod data;
mod foundations;

pub use self::data::*;
pub use self::foundations::*;

use crate::prelude::*;

/// Hook up all compute definitions.
pub(super) fn define(global: &mut Scope) {
    self::foundations::define(global);
    self::data::define(global);
    self::calc::define(global);
    self::sys::define(global);
}
