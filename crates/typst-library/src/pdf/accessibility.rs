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

/// Marks content as a PDF artifact.
///
/// Artifacts are parts of the document that are not meant to be read by
/// Assistive Technology (AT), such as screen readers. Typical examples include
/// purely decorative images that do not contribute to the meaning of the
/// document, watermarks, or repeated content such as page numbers.
///
/// Typst will automatically mark certain content, such as page headers,
/// footers, backgrounds, and foregrounds, as artifacts. Likewise, paths and
/// shapes are automatically marked as artifacts, but their content is not.
/// Repetitions of table headers and footers are also marked as artifacts.
///
/// Once something is marked as an artifact, you cannot make any of its
/// contents accessible again. If you need to mark only part of something as an
/// artifact, you may need to use this function multiple times.
///
/// If you are unsure what constitutes an artifact, check the [Accessibility
/// Guide]($guides/accessibility/#artifacts).
///
/// In the future, this function may be moved out of the `pdf` module, making it
/// possible to hide content in HTML export from AT.
// TODO: maybe generalize this and use it to mark html elements with `aria-hidden="true"`?
#[elem(Tagged)]
pub struct ArtifactElem {
    /// The artifact kind.
    ///
    /// This will govern how the PDF reader treats the artifact during reflow
    /// and content extraction (e.g. copy and paste).
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
    /// Not part of the document, but rather the page it is printed on. An
    /// example would be cut marks or color bars.
    Page,
    /// Other artifacts, including purely cosmetic content, backgrounds,
    /// watermarks, and repeated content.
    #[default]
    Other,
}

/// A summary of the purpose and structure of a complex table.
///
/// This will be available for Assistive Technology (AT), such as screen
/// readers, when exporting to PDF, but not for sighted readers of your file.
///
/// This field is intended for instructions that help the user navigate the
/// table using AT. It is not an alternative description, so do not duplicate
/// the contents of the table within. Likewise, do not use this for the core
/// takeaway of the table. Instead, include that in the text around the table
/// or, even better, in a [figure caption]($figure.caption).
///
/// If in doubt whether your table is complex enough to warrant a summary, err
/// on the side of not including one. If you are certain that your table is
/// complex enough, consider whether a sighted user might find it challenging.
/// They might benefit from the instructions you put here, so consider printing
/// them visibly in the document instead.
///
/// The API of this feature is temporary. Hence, calling this function requires
/// enabling the `a11y-extras` feature flag at the moment. Even if this
/// functionality should be available without a feature flag in the future, the
/// summary will remain exclusive to PDF export.
///
/// ```example
/// #figure(
///   pdf.table-summary(
///     // The summary just provides orientation and structural
///     // information for AT users.
///     summary: "The first two columns list the names of each participant. The last column contains cells spanning multiple rows for their assigned group.",
///     table(
///       columns: 3,
///       table.header[First Name][Given Name][Group],
///       [Mike], [Davis], table.cell(rowspan: 3)[Sales],
///       [Anna], [Smith],
///       [John], [Johnson],
///       [Sara], [Wilkins], table.cell(rowspan: 2)[Operations],
///       [Tom], [Brown],
///     ),
///   ),
///   // This is the key takeaway of the table, so we put it in the caption.
///   caption: [The Sales org now has a new member],
/// )
/// ```
#[func]
pub fn table_summary(
    #[named] summary: Option<EcoString>,
    /// The table.
    table: TableElem,
) -> Content {
    table.with_summary(summary).pack()
}

/// Explicitly defines a cell as a header cell.
///
/// Header cells help users of Assistive Technology (AT) understand and navigate
/// complex tables. When your table is correctly marked up with header cells, AT
/// can announce the relevant header information on-demand when entering a cell.
///
/// By default, Typst will automatically mark all cells within [`table.header`]
/// as header cells. They will apply to the columns below them. You can use that
/// function's [`level`]($table.header.level) parameter to make header cells
/// labelled by other header cells.
///
/// The `pdf.header-cell` function allows you to indicate that a cell is a
/// header cell in the following additional situations:
///
/// - You have a **header column** in which each cell applies to its row. In
///   that case, you pass `{"row"}` as an argument to the [`scope`
///   parameter]($pdf.header-cell.scope) to indicate that the header cell
///   applies to the row.
/// - You have a cell in [`table.header`], for example at the very start, that
///   labels both its row and column. In that case, you pass `{"both"}` as an
///   argument to the [`scope`]($pdf.header-cell.scope) parameter.
/// - You have a header cell in a row not containing other header cells. In that
///   case, you can use this function to mark it as a header cell.
///
/// The API of this feature is temporary. Hence, calling this function requires
/// enabling the `a11y-extras` feature flag at the moment. In a future Typst
/// release, this functionality may move out of the `pdf` module so that tables
/// in other export targets can contain the same information.
///
/// ```example
/// >>> #set text(font: "IBM Plex Sans")
/// #show table.cell.where(x: 0): set text(weight: "medium")
/// #show table.cell.where(y: 0): set text(weight: "bold")
///
/// #table(
///   columns: 3,
///   align: (start, end, end),
///
///   table.header(
///     // Top-left cell: Labels both the nutrient rows
///     // and the serving size columns.
///     pdf.header-cell(scope: "both")[Nutrient],
///     [Per 100g],
///     [Per Serving],
///   ),
///
///   // First column cells are row headers
///   pdf.header-cell(scope: "row")[Calories],
///   [250 kcal], [375 kcal],
///   pdf.header-cell(scope: "row")[Protein],
///   [8g], [12g],
///   pdf.header-cell(scope: "row")[Fat],
///   [12g], [18g],
///   pdf.header-cell(scope: "row")[Carbs],
///   [30g], [45g],
/// )
/// ```
#[func]
pub fn header_cell(
    /// The nesting level of this header cell.
    #[named]
    #[default(NonZeroU32::ONE)]
    level: NonZeroU32,
    /// What track of the table this header cell applies to.
    #[named]
    #[default]
    scope: TableHeaderScope,
    /// The table cell.
    ///
    /// This can be content or a call to [`table.cell`].
    cell: TableCell,
) -> Content {
    cell.with_kind(Smart::Custom(TableCellKind::Header(level, scope)))
        .pack()
}

/// Explicitly defines this cell as a data cell.
///
/// Each cell in a table is either a header cell or a data cell. By default, all
/// cells in [`table.header`] are header cells, and all other cells data cells.
///
/// If your header contains a cell that is not a header cell, you can use this
/// function to mark it as a data cell.
///
/// The API of this feature is temporary. Hence, calling this function requires
/// enabling the `a11y-extras` feature flag at the moment. In a future Typst
/// release, this functionality may move out of the `pdf` module so that tables
/// in other export targets can contain the same information.
///
/// ```example
/// #show table.cell.where(x: 0): set text(weight: "bold")
/// #show table.cell.where(x: 1): set text(style: "italic")
/// #show table.cell.where(x: 1, y: 0): set text(style: "normal")
///
/// #table(
///   columns: 3,
///   align: (left, left, center),
///
///   table.header[Objective][Key Result][Status],
///
///   table.header(
///     level: 2,
///     table.cell(colspan: 2)[Improve Customer Satisfaction],
///     // Status is data for this objective, not a header
///     pdf.data-cell[✓ On Track],
///   ),
///   [], [Increase NPS to 50+], [45],
///   [], [Reduce churn to \<5%], [4.2%],
///
///   table.header(
///     level: 2,
///     table.cell(colspan: 2)[Grow Revenue],
///     pdf.data-cell[⚠ At Risk],
///   ),
///   [], [Achieve \$2M ARR], [\$1.8M],
///   [], [Close 50 enterprise deals], [38],
/// )
/// ```
#[func]
pub fn data_cell(
    /// The table cell.
    ///
    /// This can be content or a call to [`table.cell`].
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

/// Which table track a header cell labels.
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
