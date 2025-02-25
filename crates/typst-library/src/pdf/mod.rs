//! PDF-specific functionality.

mod embed;

pub use self::embed::*;

use crate::foundations::{Module, Scope};

/// Hook up all `pdf` definitions.
pub fn module() -> Module {
    let mut pdf = Scope::deduplicating();
    pdf.start_category(crate::Category::Pdf);
    pdf.define_elem::<EmbedElem>();
    Module::new("pdf", pdf)
}
