use std::num::NonZeroU32;

use ecow::EcoString;
use typst_macros::{Cast, elem, func};
use typst_utils::NonZeroExt;

use crate::diag::SourceResult;
use crate::diag::bail;
use crate::engine::Engine;
use crate::foundations::{Args, Construct, Content, NativeElement, Smart};
use crate::introspection::Tagged;
use crate::model::{TableCell, TableElem};

/// Mark content as a PDF artifact.
// TODO: maybe generalize this and use it to mark html elements with `aria-hidden="true"`?
#[elem(Tagged)]
pub struct ArtifactElem {
    /// The artifact kind.
    #[default(ArtifactKind::Other)]
    pub kind: ArtifactKind,

    /// The content that is an artifact.
    #[required]
    pub body: Content,
}

/// The type of artifact.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Cast)]
pub enum ArtifactKind {
    /// Page header artifacts.
    Header,
    /// Page footer artifacts.
    Footer,
    /// Page artifacts, such as cut marks or color bars.
    Page,
    /// Other artifacts.
    #[default]
    Other,
}

/// Add a summary of the table's purpose and structure.
///
/// This will be available for assistive technologies (AT), such as screen
/// readers.
///
/// This function exists as a temporary solution and will either be removed or
/// replaced by another mechanism in a future release.
#[func]
pub fn table_summary(
    #[named] summary: Option<EcoString>,
    /// The table.
    table: TableElem,
) -> Content {
    table.with_summary(summary).pack()
}

/// Explicitly define this cell as a PDF header cell (`TH`).
///
/// This function exists as a temporary solution and will be replaced by another
/// mechanism in a future release.
#[func]
pub fn header_cell(
    #[named]
    #[default(NonZeroU32::ONE)]
    level: NonZeroU32,
    #[named]
    #[default]
    scope: TableHeaderScope,
    /// The table cell.
    cell: TableCell,
) -> Content {
    cell.with_kind(Smart::Custom(TableCellKind::Header(level, scope)))
        .pack()
}

/// Explicitly define this cell as a PDF data cell (`TD`).
///
/// This function exists as a temporary solution and will be replaced by another
/// mechanism in a future release.
#[func]
pub fn data_cell(
    /// The table cell.
    cell: TableCell,
) -> Content {
    cell.with_kind(Smart::Custom(TableCellKind::Data)).pack()
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub enum TableCellKind {
    Header(NonZeroU32, TableHeaderScope),
    Footer,
    #[default]
    Data,
}

/// The scope of a table header cell.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum TableHeaderScope {
    /// The header cell refers to both the row and the column.
    Both,
    /// The header cell refers to the column.
    #[default]
    Column,
    /// The header cell refers to the row.
    Row,
}

impl TableHeaderScope {
    pub fn refers_to_column(&self) -> bool {
        match self {
            TableHeaderScope::Both => true,
            TableHeaderScope::Column => true,
            TableHeaderScope::Row => false,
        }
    }

    pub fn refers_to_row(&self) -> bool {
        match self {
            TableHeaderScope::Both => true,
            TableHeaderScope::Column => false,
            TableHeaderScope::Row => true,
        }
    }
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
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
