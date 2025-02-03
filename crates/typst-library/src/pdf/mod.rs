//! PDF-specific functionality.

mod embed;

pub use self::embed::*;

use crate::foundations::{category, Category, Module, Scope};

/// PDF-specific functionality.
#[category]
pub static PDF: Category;

/// Hook up the `pdf` module.
pub(super) fn define(global: &mut Scope) {
    global.start_category(PDF);
    global.define("pdf", module());
}

/// Hook up all `pdf` definitions.
pub fn module() -> Module {
    let mut scope = Scope::deduplicating();
    scope.define_elem::<EmbedElem>();
    Module::new("pdf", scope)
}
