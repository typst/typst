//! Exporting Typst documents to PDF.

mod attach;
mod convert;
mod format;
mod image;
mod link;
mod metadata;
mod outline;
mod page;
mod paint;
mod shape;
mod tags;
mod text;
mod util;

pub use self::format::*;
pub use self::metadata::{Timestamp, Timezone};

use std::fmt::Debug;
use std::hash::Hash;

use comemo::Tracked;
use ecow::EcoString;
use krilla::configure::{PdfVersion, Validators};
use typst_layout::PagedDocument;
use typst_library::diag::{SourceResult, bail};
use typst_library::format::{Complete, Fields, Partial};
use typst_library::foundations::Smart;
use typst_library::introspection::Location;
use typst_library::model::LateLinkResolver;

/// Export a document into a PDF file.
///
/// Returns the raw bytes making up the PDF file.
#[typst_macros::time(name = "pdf")]
pub fn pdf(document: &PagedDocument, options: &PdfOptions) -> SourceResult<Vec<u8>> {
    convert::convert(document, options, &[], None)
}

/// Export a document into a PDF file as part of a bundle.
///
/// Takes additional `anchor` locations that will be serialized as named
/// destinations. This enables other documents in the bundle to link into the
/// resulting PDF. Also takes a `link_resolver` for resolving cross-document
/// links.
#[typst_macros::time(name = "pdf in bundle")]
pub fn pdf_in_bundle(
    document: &PagedDocument,
    options: &PdfOptions,
    anchors: &[(Location, EcoString)],
    link_resolver: Tracked<LateLinkResolver>,
) -> SourceResult<Vec<u8>> {
    convert::convert(document, options, anchors, Some(link_resolver))
}

/// Settings for PDF export.
#[derive(Debug, Default, PartialEq, Eq, Hash)]
pub struct PdfOptions<F: Fields = Partial> {
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
    pub ident: Smart<String>,
    /// Configures the `/Creator` metadata in the resulting PDF. When set to
    /// `Smart::Auto`, defaults to `Typst $version`.
    pub creator: Smart<Option<String>>,
    /// If not `None`, shall be the creation timestamp of the document. It will
    /// only be used if `set document(date: ..)` is `auto`.
    pub timestamp: Option<Timestamp>,
    /// Format options that override the defaults set by the document.
    pub format: PdfFormatOptions<F>,
}

impl PdfOptions {
    pub fn resolve(&self, doc: &PdfFormatOptions) -> SourceResult<PdfOptions<Complete>> {
        let format = self.format.resolve(doc);

        if format.pages.v.is_some() {
            if format.tagged.v == Smart::Custom(true) {
                let span = format.tagged.span.or(format.pages.span);
                bail!(span, "cannot enable tagged PDF and export a page range");
            } else if format.tagged.v.is_auto() {
                // TODO: Accept a warning sink of some sort.
                // warnings.push(
                //     SourceDiagnostic::warning(
                //         Span::detached(),
                //         "exporting a page range disables PDF tagging",
                //     )
                //     .with_hints([
                //         "the resulting PDF will be inaccessible".into(),
                //         "set `pdf(tagged: false)` to silence this warning".into(),
                //     ]),
                // );
            }
        }

        const ACCESSIBLE: [(PdfStandard, &str); 4] = [
            (PdfStandard::A_1a, "PDF/A-1a"),
            (PdfStandard::A_2a, "PDF/A-2a"),
            (PdfStandard::A_3a, "PDF/A-3a"),
            (PdfStandard::UA_1, "PDF/UA-1"),
        ];
        for (standard, name) in ACCESSIBLE {
            if format.standard.v.standards().any(|s| s == standard) {
                if format.tagged.v == Smart::Custom(false) {
                    let span = format.standard.span.or(format.tagged.span);
                    bail!(
                        span,
                        "cannot set `pdf(tagged: false)` when exporting a {name} document"
                    );
                } else if format.pages.v.is_some() {
                    let span = format.standard.span.or(format.pages.span);
                    bail!(
                        span,
                        "cannot set `pdf(tagged: false)` when exporting a {name} document";
                        hint: "using `pdf(pages: ...)` implies pdf(tagged: false)";
                    );
                }
            }
        }

        Ok(PdfOptions {
            ident: self.ident.clone(),
            creator: self.creator.clone(),
            timestamp: self.timestamp,
            format,
        })
    }
}

impl PdfOptions<Complete> {
    pub(crate) fn version(&self) -> PdfVersion {
        self.format.standard.v.config.version()
    }

    /// Returns whether to produce a tagged PDF document.
    /// By default unless explicitly disabled or when only exporting a page
    /// range, tagging is enabled.
    pub(crate) fn tagged(&self) -> bool {
        self.format.pages.v.is_none() && self.format.tagged.v.unwrap_or(true)
    }

    /// Returns the accessibility validator. Returns `Some` for PDF/UA-1, and in
    /// the future maybe PDF/UA-2.
    pub(crate) fn validators(&self) -> Validators {
        self.format.standard.v.config.validators()
    }
}
