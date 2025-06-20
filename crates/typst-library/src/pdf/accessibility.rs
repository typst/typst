use ecow::EcoString;
use typst_macros::{cast, elem};

use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{Content, Packed, Show, StyleChain};
use crate::introspection::Locatable;

// TODO: docs
#[elem(Locatable, Show)]
pub struct PdfTagElem {
    #[default(PdfTagKind::NonStruct)]
    pub kind: PdfTagKind,

    /// An alternate description.
    pub alt: Option<EcoString>,
    /// Exact replacement for this structure element and its children.
    pub actual_text: Option<EcoString>,
    /// The expanded form of an abbreviation/acronym.
    pub expansion: Option<EcoString>,

    /// The content to underline.
    #[required]
    pub body: Content,
}

impl Show for Packed<PdfTagElem> {
    #[typst_macros::time(name = "pdf.tag", span = self.span())]
    fn show(&self, _: &mut Engine, _: StyleChain) -> SourceResult<Content> {
        Ok(self.body.clone())
    }
}

// TODO: docs
/// PDF structure elements
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PdfTagKind {
    // grouping elements
    /// (Part)
    Part,
    /// (Article)
    Art,
    /// (Section)
    Sect,
    /// (Division)
    Div,
    /// (Block quotation)
    BlockQuote,
    /// (Caption)
    Caption,
    /// (Table of contents)
    TOC,
    /// (Table of contents item)
    TOCI,
    /// (Index)
    Index,
    /// (Nonstructural element)
    NonStruct,
    /// (Private element)
    Private,

    // paragraph like elements
    /// (Heading)
    H { title: Option<EcoString> },
    /// (Heading level 1)
    H1 { title: Option<EcoString> },
    /// (Heading level 2)
    H2 { title: Option<EcoString> },
    /// (Heading level 3)
    H4 { title: Option<EcoString> },
    /// (Heading level 4)
    H3 { title: Option<EcoString> },
    /// (Heading level 5)
    H5 { title: Option<EcoString> },
    /// (Heading level 6)
    H6 { title: Option<EcoString> },
    /// (Paragraph)
    P,

    // list elements
    /// (List)
    L { numbering: ListNumbering },
    /// (List item)
    LI,
    /// (Label)
    Lbl,
    /// (List body)
    LBody,

    // table elements
    /// (Table)
    Table,
    /// (Table row)
    TR,
    /// (Table header)
    TH { scope: TableHeaderScope },
    /// (Table data cell)
    TD,
    /// (Table header row group)
    THead,
    /// (Table body row group)
    TBody,
    /// (Table footer row group)
    TFoot,

    // inline elements
    /// (Span)
    Span,
    /// (Quotation)
    Quote,
    /// (Note)
    Note,
    /// (Reference)
    Reference,
    /// (Bibliography Entry)
    BibEntry,
    /// (Code)
    Code,
    /// (Link)
    Link,
    /// (Annotation)
    Annot,

    /// (Ruby)
    Ruby,
    /// (Ruby base text)
    RB,
    /// (Ruby annotation text)
    RT,
    /// (Ruby punctuation)
    RP,

    /// (Warichu)
    Warichu,
    /// (Warichu text)
    WT,
    /// (Warichu punctuation)
    WP,

    /// (Figure)
    Figure,
    /// (Formula)
    Formula,
    /// (Form)
    Form,
}

cast! {
    PdfTagKind,
    self => match self {
        PdfTagKind::Part => "part".into_value(),
        _ => todo!(),
    },
    "part" => Self::Part,
    // TODO
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ListNumbering {
    /// No numbering.
    None,
    /// Solid circular bullets.
    Disc,
    /// Open circular bullets.
    Circle,
    /// Solid square bullets.
    Square,
    /// Decimal numbers.
    Decimal,
    /// Lowercase Roman numerals.
    LowerRoman,
    /// Uppercase Roman numerals.
    UpperRoman,
    /// Lowercase letters.
    LowerAlpha,
    /// Uppercase letters.
    UpperAlpha,
}

/// The scope of a table header cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TableHeaderScope {
    /// The header cell refers to the row.
    Row,
    /// The header cell refers to the column.
    Column,
    /// The header cell refers to both the row and the column.
    Both,
}

/// Mark content as a PDF artifact.
/// TODO: maybe generalize this and use it to mark html elements with `aria-hidden="true"`?
#[elem(Locatable, Show)]
pub struct ArtifactElem {
    #[default(ArtifactKind::Other)]
    pub kind: ArtifactKind,

    /// The content to underline.
    #[required]
    pub body: Content,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ArtifactKind {
    /// Page header artifacts.
    Header,
    /// Page footer artifacts.
    Footer,
    /// Other page artifacts.
    Page,
    /// Other artifacts.
    #[default]
    Other,
}

cast! {
    ArtifactKind,
    self => match self {
        ArtifactKind::Header => "header".into_value(),
        ArtifactKind::Footer => "footer".into_value(),
        ArtifactKind::Page => "page".into_value(),
        ArtifactKind::Other => "other".into_value(),
    },
    "header" => Self::Header,
    "footer" => Self::Footer,
    "page" => Self::Page,
    "other" => Self::Other,
}

impl Show for Packed<ArtifactElem> {
    #[typst_macros::time(name = "pdf.artifact", span = self.span())]
    fn show(&self, _: &mut Engine, _: StyleChain) -> SourceResult<Content> {
        Ok(self.body.clone())
    }
}
