//! PDF-specific functionality.

mod attach;

pub use self::attach::*;

use crate::foundations::{Deprecation, Element, Module, Scope};

/// Hook up all `pdf` definitions.
pub fn module() -> Module {
    let mut pdf = Scope::deduplicating();
    pdf.start_category(crate::Category::Pdf);
    pdf.define_elem::<AttachElem>();
    pdf.define("embed", Element::of::<AttachElem>()).deprecated(
        // Remember to remove "embed" from `path_completion` when removing this.
        Deprecation::new()
            .with_message("the name `embed` is deprecated, use `attach` instead")
            .with_until("0.15.0"),
    );
    Module::new("pdf", pdf)
}
