use ecow::EcoString;
use typst_syntax::VirtualPath;

use crate::diag::{HintedStrResult, bail, error};
use crate::foundations::{
    Array, BundlePath, Cast, Content, Datetime, OneOrMultiple, Packed, ShowFn, ShowSet,
    Smart, StyleChain, Styles, Target, Value, cast, elem,
};
use crate::introspection::Locatable;
use crate::text::{Locale, TextElem};

/// Manages metadata and is used to add a document file to a bundle.
///
/// = Metadata <metadata>
/// The document element is the single source of truth for document metadata.
/// With it, you can specify the document's title, authors, date, etc. in one
/// place. Typically, the element is used with a
/// @reference:styling:set-rules[set rule] like this:
///
/// ```example
/// #set document(title: [My doc])
///
/// Title is _not_ rendered, but
/// embedded in PDF metadata.
/// ```
///
/// By default, the metadata is embedded into the output, but not visibly
/// rendered in the document. However, it becomes
/// @reference:context[contextually available] to the full document and can be
/// used by elements and templates. For instance, the built-in @title element
/// automatically picks up the configured document title:
///
/// ```example
/// #set document(title: [My doc])
///
/// #title()
/// Title is now rendered _and_
/// embedded in PDF metadata.
/// ```
///
/// In a similar fashion to the `title` element, you can also access metadata
/// yourself using a @reference:context[context expression].
///
/// ```example
/// // In the document.
/// #set document(
///   keywords: ("Typst", "Metadata")
/// )
///
/// // Somewhere in your template.
/// _Keywords:_
/// #context document.keywords.join(", ")
/// ```
///
/// In single-document export formats, this function is only used with
/// @reference:styling:set-rules[set rules]. Such set rules must only occur at
/// the top level, not inside of any layout container. You can also explicitly
/// create a `document` element, but
/// @document:documents-in-bundle-export[this is only relevant in bundle export].
///
/// == Format-specific considerations <format-specific-considerations>
/// Metadata is embedded into the output to varying extents:
///
/// - PDF export supports the full range of metadata and emits it into the PDF
///   _document information dictionary_ as well as XMP metadata.
///
/// - HTML export only supports the `title`, `description`, `author`, and
///   `keywords` properties. The `date` property is not supported as the HTML
///   standard has no provision for it.
///
/// - SVG and PNG export do not have any metadata support at all.
///
/// = Documents in bundle export <documents-in-bundle-export>
/// In @reference:bundle[bundle export], a document element represents a single
/// file in the bundle output, in one of Typst's other export formats. When
/// creating a document, you must provide an output path and some content. Typst
/// will compile and export the provided content with the appropriate format. By
/// default, the format is inferred from the file extension of the path you
/// specified, but you can also configure the @document.format[`format`]
/// explicitly.
///
/// ```typ
/// #document("index.html", title: [Home])[
///   #title()
///   View #link(<list>)[my famous list].
/// ]
///
/// #document("list.html", title: [My Famous List])[
///   #title()
///   - My
///   - Famous
///   - List
/// ] <list>
/// ```
///
/// == Metadata <metadata>
/// Document elements pick up metadata from top-level `{set document}` rules
/// within them. This means that documents written for single-document export
/// can be used with explicit `document` elements while properly retaining
/// metadata.
///
/// ```typ
/// // Will pick up the title defined in `paper.typ`.
/// #document("paper.pdf", include "paper.typ")
/// ```
///
/// ```typ
/// // paper.typ
/// #set document(title: [My Paper])
/// ...
/// ```
///
/// Note that document set rules within a `document` override explicit arguments
/// passed to the `document` element.
///
/// Moreover, properties configured as explicit arguments to `document` are made
/// contextually available:
///
/// ```typ
/// #document("index.html", title: [My title])[
///   // Both of these will pick up `[My title]`
///   #title()
///   #context document.title
/// ]
/// ```
#[elem(Locatable, ShowSet)]
pub struct DocumentElem {
    /// The path in the bundle at which the exported document will be placed.
    ///
    /// May contain interior slashes, in which case intermediate directories
    /// will be automatically created.
    ///
    /// This property is only supported in the @reference:bundle[bundle] target.
    #[required]
    pub path: BundlePath,

    /// Which format to export in.
    ///
    /// If `{auto}`, Typst attempts to infer the export format from the
    /// @document.path[`path`'s] file extension.
    ///
    /// This property is only supported in the @reference:bundle[bundle] target.
    pub format: Smart<DocumentFormat>,

    /// The document's title. This is rendered as the title of the PDF viewer
    /// window or the browser tab of the page.
    ///
    /// By default, the configured title is not visibly rendered in the
    /// document. You can add the title to the document's contents by using the
    /// @title element. It will automatically pick up the title configured here.
    ///
    /// Adding a title is important for accessibility, as it makes it easier to
    /// navigate to your document and identify it among other open documents.
    /// When exporting to PDF/UA, a title is required.
    ///
    /// While this can be arbitrary content, PDF viewers only support plain text
    /// titles, so the conversion might be lossy.
    pub title: Option<Content>,

    /// The document's authors.
    pub author: OneOrMultiple<EcoString>,

    /// The document's description.
    pub description: Option<Content>,

    /// The document's keywords.
    pub keywords: OneOrMultiple<EcoString>,

    /// The document's creation date.
    ///
    /// If this is `{auto}` (default), Typst uses the current date and time.
    /// Setting it to `{none}` prevents Typst from embedding any creation date
    /// into the PDF metadata.
    ///
    /// The year component must be at least zero in order to be embedded into a
    /// PDF.
    ///
    /// If you want to create byte-by-byte reproducible PDFs, set this to
    /// something other than `{auto}`.
    pub date: Smart<Option<Datetime>>,

    /// The content that makes up the document.
    ///
    /// This property is only supported in the @reference:bundle[bundle] target.
    #[required]
    pub body: Content,
}

impl Packed<DocumentElem> {
    /// Tries to determine the document's format based on the format that was
    /// explicitly defined, or else the extension of the document's path.
    pub fn determine_format(
        &self,
        styles: StyleChain,
    ) -> HintedStrResult<DocumentFormat> {
        self.format
            .get(styles)
            .custom()
            .or_else(|| determine_format_from_path(self.path.as_ref()))
            .ok_or_else(|| {
                error!(
                    "unknown document format";
                    hint: "try specifying the `format` explicitly";
                )
            })
    }
}

/// Derive the document format from the file extension of a path.
fn determine_format_from_path(path: &VirtualPath) -> Option<DocumentFormat> {
    match path.extension()? {
        "pdf" => Some(PagedFormat::Pdf.into()),
        "svg" => Some(PagedFormat::Svg.into()),
        "png" => Some(PagedFormat::Png.into()),
        "html" => Some(DocumentFormat::Html),
        _ => None,
    }
}

pub const DOCUMENT_UNSUPPORTED_RULE: ShowFn<DocumentElem> = |elem, _, _| {
    bail!(
        elem.span(),
        "constructing a document is only supported in the bundle target";
        // TODO: Support for CLI-specific hints would be nice.
        hint: "try enabling the bundle target";
        hint: "or use a `set document(..)` rule to configure metadata";
    )
};

impl ShowSet for Packed<DocumentElem> {
    fn show_set(&self, _: StyleChain) -> Styles {
        // Here, we make explicit document properties contextually available.
        // This is mostly relevant for bundle export as document elements are
        // not directly supported in other targets.
        //
        // Making the properties available like this is inconsistent with normal
        // elements, but consistent with `page` and necessary to make the
        // `title` element work.
        //
        // Nonetheless, it's fairly hacky and the whole thing should probably be
        // revisited at some point. Also see
        // <https://github.com/typst/typst/issues/6721>.
        let mut styles = Styles::new();
        self.format.copy_into(&mut styles);
        self.title.copy_into(&mut styles);
        self.author.copy_into(&mut styles);
        self.description.copy_into(&mut styles);
        self.keywords.copy_into(&mut styles);
        self.date.copy_into(&mut styles);
        styles
    }
}

/// Supported export formats for bundle documents.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum DocumentFormat {
    /// One of the page formats.
    Paged(PagedFormat),
    /// The document format of the web.
    Html,
}

impl DocumentFormat {
    pub fn target(self) -> Target {
        match self {
            Self::Paged(_) => Target::Paged,
            Self::Html => Target::Html,
        }
    }
}

impl From<PagedFormat> for DocumentFormat {
    fn from(format: PagedFormat) -> Self {
        Self::Paged(format)
    }
}

cast! {
    DocumentFormat,
    self => match self {
        Self::Paged(v) => v.into_value(),
        Self::Html => "html".into_value(),
    },
    v: PagedFormat => Self::Paged(v),
    /// The document format of the web.
    "html" => Self::Html,
}

/// Supported paged export formats for bundle documents.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum PagedFormat {
    /// High-fidelity document and graphics format, with focus on exact
    /// reproduction in print.
    Pdf,
    /// Raster format for illustrations and transparent graphics.
    Png,
    /// The vector graphics format of the web.
    Svg,
}

/// A list of authors.
#[derive(Debug, Default, Clone, PartialEq, Hash)]
pub struct Author(Vec<EcoString>);

cast! {
    Author,
    self => self.0.into_value(),
    v: EcoString => Self(vec![v]),
    v: Array => Self(v.into_iter().map(Value::cast).collect::<HintedStrResult<_>>()?),
}

/// A list of keywords.
#[derive(Debug, Default, Clone, PartialEq, Hash)]
pub struct Keywords(Vec<EcoString>);

cast! {
    Keywords,
    self => self.0.into_value(),
    v: EcoString => Self(vec![v]),
    v: Array => Self(v.into_iter().map(Value::cast).collect::<HintedStrResult<_>>()?),
}

/// A document resulting from compilation.
pub trait Document {
    /// Get the document's metadata.
    fn info(&self) -> &DocumentInfo;
}

/// Details about the document.
#[derive(Debug, Default, Clone, PartialEq, Hash)]
pub struct DocumentInfo {
    /// The document's title.
    pub title: Option<EcoString>,
    /// The document's author(s).
    pub author: Vec<EcoString>,
    /// The document's description.
    pub description: Option<EcoString>,
    /// The document's keywords.
    pub keywords: Vec<EcoString>,
    /// The document's creation date.
    pub date: Smart<Option<Datetime>>,
    /// The document's language, set from the first top-level set rule, e.g.
    ///
    /// ```typc
    /// set text(lang: "...", region: "...")
    /// ```
    pub locale: Smart<Locale>,
}

impl DocumentInfo {
    /// Populate this document info with details from the given styles.
    ///
    /// Document set rules are a bit special, so we need to do this manually.
    pub fn populate(&mut self, styles: StyleChain) {
        if styles.has(DocumentElem::title) {
            self.title = styles
                .get_ref(DocumentElem::title)
                .as_ref()
                .map(|content| content.plain_text());
        }
        if styles.has(DocumentElem::author) {
            self.author = styles.get_cloned(DocumentElem::author).0;
        }
        if styles.has(DocumentElem::description) {
            self.description = styles
                .get_ref(DocumentElem::description)
                .as_ref()
                .map(|content| content.plain_text());
        }
        if styles.has(DocumentElem::keywords) {
            self.keywords = styles.get_cloned(DocumentElem::keywords).0;
        }
        if styles.has(DocumentElem::date) {
            self.date = styles.get(DocumentElem::date);
        }
    }

    /// Populate this document info with locale details from the given styles.
    pub fn populate_locale(&mut self, styles: StyleChain) {
        if self.locale.is_custom() {
            return;
        }

        let mut locale: Option<Locale> = None;
        if styles.has(TextElem::lang) {
            locale.get_or_insert_default().lang = styles.get(TextElem::lang);
        }
        if styles.has(TextElem::region) {
            locale.get_or_insert_default().region = styles.get(TextElem::region);
        }
        self.locale = Smart::from(locale);
    }
}
