//! PDF-specific functionality.

mod accessibility;
mod embed;

pub use self::accessibility::*;
pub use self::embed::*;

use crate::foundations::{Module, Scope};

/// Hook up all `pdf` definitions.
pub fn module() -> Module {
    let mut pdf = Scope::deduplicating();
    pdf.start_category(crate::Category::Pdf);
    pdf.define_elem::<EmbedElem>();
    pdf.define_elem::<ArtifactElem>();
    pdf.define_func::<header_cell>();
    pdf.define_func::<data_cell>();
    Module::new("pdf", pdf)
}
