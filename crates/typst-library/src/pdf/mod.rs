//! PDF-specific functionality.

mod accessibility;
mod attach;

pub use self::accessibility::*;
pub use self::attach::*;

use crate::foundations::{Module, Scope};
use crate::{Feature, Features};

/// Hook up all `pdf` definitions.
pub fn module(features: &Features) -> Module {
    let mut pdf = Scope::deduplicating();
    pdf.start_category(crate::Category::Pdf);
    pdf.define_elem::<AttachElem>();
    pdf.define_elem::<ArtifactElem>();
    if features.is_enabled(Feature::A11yExtras) {
        pdf.define_func::<table_summary>();
        pdf.define_func::<header_cell>();
        pdf.define_func::<data_cell>();
    }
    Module::new("pdf", pdf)
}
