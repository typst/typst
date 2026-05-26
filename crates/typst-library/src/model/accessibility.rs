use typst_macros::{Cast, elem};

use crate::diag::SourceResult;
use crate::diag::bail;
use crate::engine::Engine;
use crate::foundations::{Args, Construct, Content, NativeElement};
use crate::introspection::Tagged;

/// Marks content as a PDF artifact.
///
/// Artifacts are parts of the document that are not meant to be read by
/// Assistive Technology (AT), such as screen readers. Typical examples include
/// purely decorative images that do not contribute to the meaning of the
/// document, watermarks, or repeated content such as page numbers.
///
/// Typst will automatically mark certain content, such as page headers,
/// footers, backgrounds, and foregrounds, as artifacts. Likewise, paths and
/// shapes are automatically marked as artifacts, but their content is not. Line
/// numbers created using @par.line are automatically marked as artifacts, as
/// are repetitions of table headers and footers.
///
/// Once something is marked as an artifact, you cannot make any of its contents
/// accessible again. If you need to mark only part of something as an artifact,
/// you may need to use this function multiple times.
///
/// If you are unsure what constitutes an artifact, check the
/// @guides:accessibility:artifacts[Accessibility Guide].
///
/// In the future, this function may be moved out of the `pdf` module, making it
/// possible to hide content in HTML export from AT.
// TODO: maybe generalize this and use it to mark html elements with `aria-hidden="true"`?
#[elem(Tagged)]
pub struct ArtifactElem {
    /// The artifact kind.
    ///
    /// You can improve accessibility by using the most specific artifact kind
    /// available. Your choice will govern how the PDF reader treats the
    /// artifact during reflow and content extraction (e.g. copy and paste).
    ///
    /// Artifact types have been introduced in various different PDF
    /// specifications. Depending on which PDF version you target, Typst will
    /// select the most appropriate artifact type using your selection here.
    #[default(ArtifactKind::Other)]
    pub kind: ArtifactKind,

    /// The content that is an artifact.
    #[required]
    pub body: Content,
}

/// The type of artifact.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum ArtifactKind {
    /// Repeats on the top of each page.
    Header,
    /// Repeats at the bottom of each page.
    Footer,
    /// Text or graphics in the back- or foreground of all pages.
    Watermark,
    /// Page numbers. Note that if your page numbers are contained in a footer
    /// or header instead, the whole header or footer should an artifact of the
    /// appropriate type.
    PageNumber,
    /// Line or paragraph numbers.
    LineNumber,
    /// Placeholders for areas in which there was content in another rendition
    /// of the document which has since been removed.
    Redaction,
    /// Bates numbering. Note that if your Bates numbering is contained in a
    /// footer or header instead, the whole header or footer should an artifact
    /// of the appropriate type.
    Bates,
    /// Not part of the document, but rather the page it is printed on. An
    /// example would be cut marks or color bars.
    Page,
    /// Artifacts arising from paginating the document not covered by other
    /// artifact types. This category generally applies if this artifact would
    /// not appear in your document if it was a website instead. If your
    /// artifact is covered by other categories, prefer them over this.
    PaginationOther,
    /// Purely cosmetric content or typographical flourishes not contributing to
    /// the document's content.
    Layout,
    /// Background of a page or a graphical element. This artifact kind was
    /// added in PDF 1.7. However, due to requirements in the PDF 1.7
    /// specification that later specifications lifted, Typst only uses this
    /// artifact type in PDF 2.0. If you use it in a PDF 1.7 or earlier, Typst
    /// will use the `{"other"}` type instead.
    Background,
    /// Other artifacts.
    #[default]
    Other,
}

/// Used to delimit content for tagged PDF.
#[elem(Construct, Tagged)]
pub struct PdfMarkerTag {
    #[internal]
    #[required]
    pub kind: PdfMarkerTagKind,
    #[required]
    pub body: Content,
}

impl Construct for PdfMarkerTag {
    fn construct(_: &mut Engine, args: &mut Args) -> SourceResult<Content> {
        bail!(args.span, "cannot be constructed manually");
    }
}

macro_rules! pdf_marker_tag {
    ($(#[doc = $doc:expr] $variant:ident$(($($name:ident: $ty:ty)+))?,)+) => {
        #[derive(Debug, Clone, Eq, PartialEq, Hash)]
        pub enum PdfMarkerTagKind {
            $(
                #[doc = $doc]
                $variant $(($($ty),+))?
            ),+
        }

        impl PdfMarkerTag {
            $(
                #[doc = $doc]
                #[allow(non_snake_case)]
                pub fn $variant($($($name: $ty,)+)? body: Content) -> Content {
                    let span = body.span();
                    Self {
                        kind: PdfMarkerTagKind::$variant $(($($name),+))?,
                        body,
                    }.pack().spanned(span)
                }
            )+
        }
    }
}

pdf_marker_tag! {
    /// `TOC`.
    OutlineBody,
    /// `L` bibliography list.
    Bibliography(numbered: bool),
    /// `LBody` wrapping `BibEntry`.
    BibEntry,
    /// `Lbl` (marker) of the list item.
    ListItemLabel,
    /// `LBody` of the list item.
    ListItemBody,
    /// `Lbl` of the term item.
    TermsItemLabel,
    /// `LBody` the term item including the label.
    TermsItemBody,
    /// A generic `Lbl`.
    Label,
}
