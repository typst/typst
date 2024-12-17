//! Exporting Typst documents to PDF.

mod image;
mod krilla;
mod paint;
mod page;
mod util;
mod outline;

use typst_library::diag::SourceResult;
use typst_library::foundations::{Datetime, Smart};
use typst_library::layout::{PageRanges, PagedDocument};

/// Export a document into a PDF file.
///
/// Returns the raw bytes making up the PDF file.
#[typst_macros::time(name = "pdf")]
pub fn pdf(document: &PagedDocument, options: &PdfOptions) -> SourceResult<Vec<u8>> {
    krilla::pdf(document, options)
}

pub use ::krilla::validation::Validator;
pub use ::krilla::version::PdfVersion;

/// Settings for PDF export.
#[derive(Debug, Default)]
pub struct PdfOptions<'a> {
    /// If not `Smart::Auto`, shall be a string that uniquely and stably
    /// identifies the document. It should not change between compilations of
    /// the same document.  **If you cannot provide such a stable identifier,
    /// just pass `Smart::Auto` rather than trying to come up with one.** The
    /// CLI, for example, does not have a well-defined notion of a long-lived
    /// project and as such just passes `Smart::Auto`.
    ///
    /// If an `ident` is given, the hash of it will be used to create a PDF
    /// document identifier (the identifier itself is not leaked). If `ident` is
    /// `Auto`, a hash of the document's title and author is used instead (which
    /// is reasonably unique and stable).
    pub ident: Smart<&'a str>,
    /// If not `None`, shall be the creation date of the document as a UTC
    /// datetime. It will only be used if `set document(date: ..)` is `auto`.
    pub timestamp: Option<Datetime>,
    /// Specifies which ranges of pages should be exported in the PDF. When
    /// `None`, all pages should be exported.
    pub page_ranges: Option<PageRanges>,
    /// The version that should be used to export the PDF.
    pub pdf_version: Option<PdfVersion>,
    /// A standard the PDF should conform to.
    pub validator: Validator,
}
