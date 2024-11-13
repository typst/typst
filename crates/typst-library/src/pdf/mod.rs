pub mod embed;

use crate::foundations::Scope;
use crate::foundations::{Category, Module};
use crate::pdf::embed::EmbedElem;
use typst_macros::category;

/// Hook up the pdf module and category.
pub(super) fn define(global: &mut Scope) {
    global.category(PDF);
    global.define_module(module());
}

/// PDF specific functionality.
#[category]
pub static PDF: Category;

/// Hook up the pdf definitions.
pub fn module() -> Module {
    let mut pdf = Scope::deduplicating();
    pdf.define_elem::<EmbedElem>();
    Module::new("pdf", pdf)
}
