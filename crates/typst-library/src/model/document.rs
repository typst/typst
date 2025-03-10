use ecow::EcoString;

use crate::diag::{bail, HintedStrResult, SourceResult};
use crate::engine::Engine;
use crate::foundations::{
    cast, elem, Args, Array, Construct, Content, Datetime, Fields, OneOrMultiple, Smart,
    StyleChain, Styles, Value,
};

/// The root element of a document and its metadata.
///
/// All documents are automatically wrapped in a `document` element. You cannot
/// create a document element yourself. This function is only used with
/// [set rules]($styling/#set-rules) to specify document metadata. Such a set
/// rule must not occur inside of any layout container.
///
/// ```example
/// #set document(title: [Hello])
///
/// This has no visible output, but
/// embeds metadata into the PDF!
/// ```
///
/// Note that metadata set with this function is not rendered within the
/// document. Instead, it is embedded in the compiled PDF file.
#[elem(Construct)]
pub struct DocumentElem {
    /// The document's title. This is often rendered as the title of the
    /// PDF viewer window.
    ///
    /// While this can be arbitrary content, PDF viewers only support plain text
    /// titles, so the conversion might be lossy.
    #[ghost]
    pub title: Option<Content>,

    /// The document's authors.
    #[ghost]
    pub author: OneOrMultiple<EcoString>,

    /// The document's description.
    #[ghost]
    pub description: Option<Content>,

    /// The document's keywords.
    #[ghost]
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
    #[ghost]
    pub date: Smart<Option<Datetime>>,
}

impl Construct for DocumentElem {
    fn construct(_: &mut Engine, args: &mut Args) -> SourceResult<Content> {
        bail!(args.span, "can only be used in set rules")
    }
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
}

impl DocumentInfo {
    /// Populate this document info with details from the given styles.
    ///
    /// Document set rules are a bit special, so we need to do this manually.
    pub fn populate(&mut self, styles: &Styles) {
        let chain = StyleChain::new(styles);
        let has = |field| styles.has::<DocumentElem>(field as _);
        if has(<DocumentElem as Fields>::Enum::Title) {
            self.title =
                DocumentElem::title_in(chain).map(|content| content.plain_text());
        }
        if has(<DocumentElem as Fields>::Enum::Author) {
            self.author = DocumentElem::author_in(chain).0;
        }
        if has(<DocumentElem as Fields>::Enum::Description) {
            self.description =
                DocumentElem::description_in(chain).map(|content| content.plain_text());
        }
        if has(<DocumentElem as Fields>::Enum::Keywords) {
            self.keywords = DocumentElem::keywords_in(chain).0;
        }
        if has(<DocumentElem as Fields>::Enum::Date) {
            self.date = DocumentElem::date_in(chain);
        }
    }
}
