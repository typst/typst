//! PDF-specific functionality.

mod accessibility;
mod attach;

pub use self::accessibility::*;
pub use self::attach::*;

use crate::Feature;
use crate::foundations::{Module, Scope};

/// Hook up all `pdf` definitions.
pub fn module() -> Module {
    let mut pdf = Scope::deduplicating();
    pdf.start_category(crate::Category::Pdf);
    pdf.define_elem::<AttachElem>();
    pdf.define_elem::<ArtifactElem>();

    pdf.define_func::<table_summary>().feature(Feature::A11yExtras);
    pdf.define_func::<header_cell>().feature(Feature::A11yExtras);
    pdf.define_func::<data_cell>().feature(Feature::A11yExtras);

    Module::new("pdf", pdf)
}
