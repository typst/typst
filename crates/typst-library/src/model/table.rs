use std::num::{NonZeroU32, NonZeroUsize};
use std::sync::Arc;

use typst_utils::NonZeroExt;

use crate::diag::{HintedStrResult, HintedString, bail};
use crate::foundations::{Content, Packed, Smart, cast, elem, scope};
use crate::layout::{
    Abs, Alignment, Celled, GridCell, GridFooter, GridHLine, GridHeader, GridVLine,
    Length, OuterHAlignment, OuterVAlignment, Rel, Sides, TrackSizings,
};
use crate::model::Figurable;
use crate::text::LocalName;
use crate::visualize::{Paint, Stroke};

/// A table of items.
///
/// Tables are used to arrange content in cells. Cells can contain arbitrary
/// content, including multiple paragraphs and are specified in row-major order.
/// For a hands-on explanation of all the ways you can use and customize tables
/// in Typst, check out the [table guide]($guides/table-guide).
///
/// Because tables are just grids with different defaults for some cell
/// properties (notably `stroke` and `inset`), refer to the [grid
/// documentation]($grid/#track-size) for more information on how to size the
/// table tracks and specify the cell appearance properties.
///
/// If you are unsure whether you should be using a table or a grid, consider
/// whether the content you are arranging semantically belongs together as a set
/// of related data points or similar or whether you are just want to enhance
/// your presentation by arranging unrelated content in a grid. In the former
/// case, a table is the right choice, while in the latter case, a grid is more
/// appropriate. Furthermore, Typst will annotate its output in the future such
/// that screenreaders will announce content in `table` as tabular while a
/// grid's content will be announced no different than multiple content blocks
/// in the document flow.
///
/// Note that, to override a particular cell's properties or apply show rules on
/// table cells, you can use the [`table.cell`] element. See its documentation
/// for more information.
///
/// Although the `table` and the `grid` share most properties, set and show
/// rules on one of them do not affect the other. Locating most of your styling
/// in set and show rules is recommended, as it keeps the table's actual usages
/// clean and easy to read. It also allows you to easily change the appearance
/// of all tables in one place.
///
/// To give a table a caption and make it [referenceable]($ref), put it into a
/// [figure].
///
/// # Example
///
/// The example below demonstrates some of the most common table options.
/// ```example
/// #table(
///   columns: (1fr, auto, auto),
///   inset: 10pt,
///   align: horizon,
///   table.header(
///     [], [*Volume*], [*Parameters*],
///   ),
///   image("cylinder.svg"),
///   $ pi h (D^2 - d^2) / 4 $,
///   [
///     $h$: height \
///     $D$: outer radius \
///     $d$: inner radius
///   ],
///   image("tetrahedron.svg"),
///   $ sqrt(2) / 12 a^3 $,
///   [$a$: edge length]
/// )
/// ```
///
/// Much like with grids, you can use [`table.cell`] to customize the appearance
/// and the position of each cell.
///
/// ```example
/// >>> #set page(width: auto)
/// >>> #set text(font: "IBM Plex Sans")
/// >>> #let gray = rgb("#565565")
/// >>>
/// #set table(
///   stroke: none,
///   gutter: 0.2em,
///   fill: (x, y) =>
///     if x == 0 or y == 0 { gray },
///   inset: (right: 1.5em),
/// )
///
/// #show table.cell: it => {
///   if it.x == 0 or it.y == 0 {
///     set text(white)
///     strong(it)
///   } else if it.body == [] {
///     // Replace empty cells with 'N/A'
///     pad(..it.inset)[_N/A_]
///   } else {
///     it
///   }
/// }
///
/// #let a = table.cell(
///   fill: green.lighten(60%),
/// )[A]
/// #let b = table.cell(
///   fill: aqua.lighten(60%),
/// )[B]
///
/// #table(
///   columns: 4,
///   [], [Exam 1], [Exam 2], [Exam 3],
///
///   [John], [], a, [],
///   [Mary], [], a, a,
///   [Robert], b, a, b,
/// )
/// ```
#[elem(scope, LocalName, Figurable)]
pub struct TableElem {
    /// The column sizes. See the [grid documentation]($grid/#track-size) for
    /// more information on track sizing.
    pub columns: TrackSizings,

    /// The row sizes. See the [grid documentation]($grid/#track-size) for more
    /// information on track sizing.
    pub rows: TrackSizings,

    /// The gaps between rows and columns. This is a shorthand for setting
    /// `column-gutter` and `row-gutter` to the same value. See the [grid
    /// documentation]($grid.gutter) for more information on gutters.
    #[external]
    pub gutter: TrackSizings,

    /// The gaps between columns. Takes precedence over `gutter`. See the
    /// [grid documentation]($grid.gutter) for more information on gutters.
    #[parse(
        let gutter = args.named("gutter")?;
        args.named("column-gutter")?.or_else(|| gutter.clone())
    )]
    pub column_gutter: TrackSizings,

    /// The gaps between rows. Takes precedence over `gutter`. See the
    /// [grid documentation]($grid.gutter) for more information on gutters.
    #[parse(args.named("row-gutter")?.or_else(|| gutter.clone()))]
    pub row_gutter: TrackSizings,

    /// How much to pad the cells' content.
    ///
    /// To specify the same inset for all cells, use a single length for all
    /// sides, or a dictionary of lengths for individual sides. See the
    /// [box's documentation]($box.inset) for more details.
    ///
    /// To specify a varying inset for different cells, you can:
    /// - use a single, uniform inset for all cells
    /// - use an array of insets for each column
    /// - use a function that maps a cell's X/Y position (both starting from
    ///   zero) to its inset
    ///
    /// See the [grid documentation]($grid/#styling) for more details.
    ///
    /// ```example
    /// #table(
    ///   columns: 2,
    ///   inset: 10pt,
    ///   [Hello],
    ///   [World],
    /// )
    ///
    /// #table(
    ///   columns: 2,
    ///   inset: (x: 20pt, y: 10pt),
    ///   [Hello],
    ///   [World],
    /// )
    /// ```
    #[fold]
    #[default(Celled::Value(Sides::splat(Some(Abs::pt(5.0).into()))))]
    pub inset: Celled<Sides<Option<Rel<Length>>>>,

    /// How to align the cells' content.
    ///
    /// If set to `{auto}`, the outer alignment is used.
    ///
    /// You can specify the alignment in any of the following fashions:
    /// - use a single alignment for all cells
    /// - use an array of alignments corresponding to each column
    /// - use a function that maps a cell's X/Y position (both starting from
    ///   zero) to its alignment
    ///
    /// See the [table guide]($guides/table-guide/#alignment) for details.
    ///
    /// ```example
    /// #table(
    ///   columns: 3,
    ///   align: (left, center, right),
    ///   [Hello], [Hello], [Hello],
    ///   [A], [B], [C],
    /// )
    /// ```
    pub align: Celled<Smart<Alignment>>,

    /// How to fill the cells.
    ///
    /// This can be:
    /// - a single fill for all cells
    /// - an array of fill corresponding to each column
    /// - a function that maps a cell's position to its fill
    ///
    /// Most notably, arrays and functions are useful for creating striped
    /// tables. See the [table guide]($guides/table-guide/#fills) for more
    /// details.
    ///
    /// ```example
    /// #table(
    ///   fill: (x, _) =>
    ///     if calc.odd(x) { luma(240) }
    ///     else { white },
    ///   align: (x, y) =>
    ///     if y == 0 { center }
    ///     else if x == 0 { left }
    ///     else { right },
    ///   columns: 4,
    ///   [], [*Q1*], [*Q2*], [*Q3*],
    ///   [Revenue:], [1000 €], [2000 €], [3000 €],
    ///   [Expenses:], [500 €], [1000 €], [1500 €],
    ///   [Profit:], [500 €], [1000 €], [1500 €],
    /// )
    /// ```
    pub fill: Celled<Option<Paint>>,

    /// How to [stroke] the cells.
    ///
    /// Strokes can be disabled by setting this to `{none}`.
    ///
    /// If it is necessary to place lines which can cross spacing between cells
    /// produced by the [`gutter`]($table.gutter) option, or to override the
    /// stroke between multiple specific cells, consider specifying one or more
    /// of [`table.hline`] and [`table.vline`] alongside your table cells.
    ///
    /// To specify the same stroke for all cells, use a single [stroke] for all
    /// sides, or a dictionary of [strokes]($stroke) for individual sides. See
    /// the [rectangle's documentation]($rect.stroke) for more details.
    ///
    /// To specify varying strokes for different cells, you can:
    /// - use a single stroke for all cells
    /// - use an array of strokes corresponding to each column
    /// - use a function that maps a cell's position to its stroke
    ///
    /// See the [table guide]($guides/table-guide/#strokes) for more details.
    #[fold]
    #[default(Celled::Value(Sides::splat(Some(Some(Arc::new(Stroke::default()))))))]
    pub stroke: Celled<Sides<Option<Option<Arc<Stroke>>>>>,

    /// The contents of the table cells, plus any extra table lines specified
    /// with the [`table.hline`] and [`table.vline`] elements.
    #[variadic]
    pub children: Vec<TableChild>,
}

#[scope]
impl TableElem {
    #[elem]
    type TableCell;

    #[elem]
    type TableHLine;

    #[elem]
    type TableVLine;

    #[elem]
    type TableHeader;

    #[elem]
    type TableFooter;
}

impl LocalName for Packed<TableElem> {
    const KEY: &'static str = "table";
}

impl Figurable for Packed<TableElem> {}

/// Any child of a table element.
#[derive(Debug, PartialEq, Clone, Hash)]
pub enum TableChild {
    Header(Packed<TableHeader>),
    Footer(Packed<TableFooter>),
    Item(TableItem),
}

cast! {
    TableChild,
    self => match self {
        Self::Header(header) => header.into_value(),
        Self::Footer(footer) => footer.into_value(),
        Self::Item(item) => item.into_value(),
    },
    v: Content => {
        v.try_into()?
    },
}

impl TryFrom<Content> for TableChild {
    type Error = HintedString;

    fn try_from(value: Content) -> HintedStrResult<Self> {
        if value.is::<GridHeader>() {
            bail!(
                "cannot use `grid.header` as a table header";
                hint: "use `table.header` instead"
            )
        }
        if value.is::<GridFooter>() {
            bail!(
                "cannot use `grid.footer` as a table footer";
                hint: "use `table.footer` instead"
            )
        }

        value
            .into_packed::<TableHeader>()
            .map(Self::Header)
            .or_else(|value| value.into_packed::<TableFooter>().map(Self::Footer))
            .or_else(|value| TableItem::try_from(value).map(Self::Item))
    }
}

/// A table item, which is the basic unit of table specification.
#[derive(Debug, PartialEq, Clone, Hash)]
pub enum TableItem {
    HLine(Packed<TableHLine>),
    VLine(Packed<TableVLine>),
    Cell(Packed<TableCell>),
}

cast! {
    TableItem,
    self => match self {
        Self::HLine(hline) => hline.into_value(),
        Self::VLine(vline) => vline.into_value(),
        Self::Cell(cell) => cell.into_value(),
    },
    v: Content => {
        v.try_into()?
    },
}

impl TryFrom<Content> for TableItem {
    type Error = HintedString;

    fn try_from(value: Content) -> HintedStrResult<Self> {
        if value.is::<GridHeader>() {
            bail!("cannot place a grid header within another header or footer");
        }
        if value.is::<TableHeader>() {
            bail!("cannot place a table header within another header or footer");
        }
        if value.is::<GridFooter>() {
            bail!("cannot place a grid footer within another footer or header");
        }
        if value.is::<TableFooter>() {
            bail!("cannot place a table footer within another footer or header");
        }
        if value.is::<GridCell>() {
            bail!(
                "cannot use `grid.cell` as a table cell";
                hint: "use `table.cell` instead"
            );
        }
        if value.is::<GridHLine>() {
            bail!(
                "cannot use `grid.hline` as a table line";
                hint: "use `table.hline` instead"
            );
        }
        if value.is::<GridVLine>() {
            bail!(
                "cannot use `grid.vline` as a table line";
                hint: "use `table.vline` instead"
            );
        }

        Ok(value
            .into_packed::<TableHLine>()
            .map(Self::HLine)
            .or_else(|value| value.into_packed::<TableVLine>().map(Self::VLine))
            .or_else(|value| value.into_packed::<TableCell>().map(Self::Cell))
            .unwrap_or_else(|value| {
                let span = value.span();
                Self::Cell(Packed::new(TableCell::new(value)).spanned(span))
            }))
    }
}

/// A repeatable table header.
///
/// You should wrap your tables' heading rows in this function even if you do not
/// plan to wrap your table across pages because Typst will use this function to
/// attach accessibility metadata to tables in the future and ensure universal
/// access to your document.
///
/// You can use the `repeat` parameter to control whether your table's header
/// will be repeated across pages.
///
/// ```example
/// #set page(height: 11.5em)
/// #set table(
///   fill: (x, y) =>
///     if x == 0 or y == 0 {
///       gray.lighten(40%)
///     },
///   align: right,
/// )
///
/// #show table.cell.where(x: 0): strong
/// #show table.cell.where(y: 0): strong
///
/// #table(
///   columns: 4,
///   table.header(
///     [], [Blue chip],
///     [Fresh IPO], [Penny st'k],
///   ),
///   table.cell(
///     rowspan: 6,
///     align: horizon,
///     rotate(-90deg, reflow: true)[
///       *USD / day*
///     ],
///   ),
///   [0.20], [104], [5],
///   [3.17], [108], [4],
///   [1.59], [84],  [1],
///   [0.26], [98],  [15],
///   [0.01], [195], [4],
///   [7.34], [57],  [2],
/// )
/// ```
#[elem(name = "header", title = "Table Header")]
pub struct TableHeader {
    /// Whether this header should be repeated across pages.
    #[default(true)]
    pub repeat: bool,

    /// The level of the header. Must not be zero.
    ///
    /// This allows repeating multiple headers at once. Headers with different
    /// levels can repeat together, as long as they have ascending levels.
    ///
    /// Notably, when a header with a lower level starts repeating, all higher
    /// or equal level headers stop repeating (they are "replaced" by the new
    /// header).
    #[default(NonZeroU32::ONE)]
    pub level: NonZeroU32,

    /// The cells and lines within the header.
    #[variadic]
    pub children: Vec<TableItem>,
}

/// A repeatable table footer.
///
/// Just like the [`table.header`] element, the footer can repeat itself on
/// every page of the table. This is useful for improving legibility by adding
/// the column labels in both the header and footer of a large table, totals, or
/// other information that should be visible on every page.
///
/// No other table cells may be placed after the footer.
#[elem(name = "footer", title = "Table Footer")]
pub struct TableFooter {
    /// Whether this footer should be repeated across pages.
    #[default(true)]
    pub repeat: bool,

    /// The cells and lines within the footer.
    #[variadic]
    pub children: Vec<TableItem>,
}

/// A horizontal line in the table.
///
/// Overrides any per-cell stroke, including stroke specified through the
/// table's `stroke` field. Can cross spacing between cells created through the
/// table's [`column-gutter`]($table.column-gutter) option.
///
/// Use this function instead of the table's `stroke` field if you want to
/// manually place a horizontal line at a specific position in a single table.
/// Consider using [table's `stroke`]($table.stroke) field or [`table.cell`'s
/// `stroke`]($table.cell.stroke) field instead if the line you want to place is
/// part of all your tables' designs.
///
/// ```example
/// #set table.hline(stroke: .6pt)
///
/// #table(
///   stroke: none,
///   columns: (auto, 1fr),
///   [09:00], [Badge pick up],
///   [09:45], [Opening Keynote],
///   [10:30], [Talk: Typst's Future],
///   [11:15], [Session: Good PRs],
///   table.hline(start: 1),
///   [Noon], [_Lunch break_],
///   table.hline(start: 1),
///   [14:00], [Talk: Tracked Layout],
///   [15:00], [Talk: Automations],
///   [16:00], [Workshop: Tables],
///   table.hline(),
///   [19:00], [Day 1 Attendee Mixer],
/// )
/// ```
#[elem(name = "hline", title = "Table Horizontal Line")]
pub struct TableHLine {
    /// The row above which the horizontal line is placed (zero-indexed).
    /// Functions identically to the `y` field in [`grid.hline`]($grid.hline.y).
    pub y: Smart<usize>,

    /// The column at which the horizontal line starts (zero-indexed, inclusive).
    pub start: usize,

    /// The column before which the horizontal line ends (zero-indexed,
    /// exclusive).
    pub end: Option<NonZeroUsize>,

    /// The line's stroke.
    ///
    /// Specifying `{none}` removes any lines previously placed across this
    /// line's range, including hlines or per-cell stroke below it.
    #[fold]
    #[default(Some(Arc::new(Stroke::default())))]
    pub stroke: Option<Arc<Stroke>>,

    /// The position at which the line is placed, given its row (`y`) - either
    /// `{top}` to draw above it or `{bottom}` to draw below it.
    ///
    /// This setting is only relevant when row gutter is enabled (and
    /// shouldn't be used otherwise - prefer just increasing the `y` field by
    /// one instead), since then the position below a row becomes different
    /// from the position above the next row due to the spacing between both.
    #[default(OuterVAlignment::Top)]
    pub position: OuterVAlignment,
}

/// A vertical line in the table. See the docs for [`grid.vline`] for more
/// information regarding how to use this element's fields.
///
/// Overrides any per-cell stroke, including stroke specified through the
/// table's `stroke` field. Can cross spacing between cells created through the
/// table's [`row-gutter`]($table.row-gutter) option.
///
/// Similar to [`table.hline`], use this function if you want to manually place
/// a vertical line at a specific position in a single table and use the
/// [table's `stroke`]($table.stroke) field or [`table.cell`'s
/// `stroke`]($table.cell.stroke) field instead if the line you want to place is
/// part of all your tables' designs.
#[elem(name = "vline", title = "Table Vertical Line")]
pub struct TableVLine {
    /// The column before which the vertical line is placed (zero-indexed).
    /// Functions identically to the `x` field in [`grid.vline`].
    pub x: Smart<usize>,

    /// The row at which the vertical line starts (zero-indexed, inclusive).
    pub start: usize,

    /// The row on top of which the vertical line ends (zero-indexed,
    /// exclusive).
    pub end: Option<NonZeroUsize>,

    /// The line's stroke.
    ///
    /// Specifying `{none}` removes any lines previously placed across this
    /// line's range, including vlines or per-cell stroke below it.
    #[fold]
    #[default(Some(Arc::new(Stroke::default())))]
    pub stroke: Option<Arc<Stroke>>,

    /// The position at which the line is placed, given its column (`x`) -
    /// either `{start}` to draw before it or `{end}` to draw after it.
    ///
    /// The values `{left}` and `{right}` are also accepted, but discouraged as
    /// they cause your table to be inconsistent between left-to-right and
    /// right-to-left documents.
    ///
    /// This setting is only relevant when column gutter is enabled (and
    /// shouldn't be used otherwise - prefer just increasing the `x` field by
    /// one instead), since then the position after a column becomes different
    /// from the position before the next column due to the spacing between
    /// both.
    #[default(OuterHAlignment::Start)]
    pub position: OuterHAlignment,
}

/// A cell in the table. Use this to position a cell manually or to apply
/// styling. To do the latter, you can either use the function to override the
/// properties for a particular cell, or use it in show rules to apply certain
/// styles to multiple cells at once.
///
/// Perhaps the most important use case of `{table.cell}` is to make a cell span
/// multiple columns and/or rows with the `colspan` and `rowspan` fields.
///
/// ```example
/// >>> #set page(width: auto)
/// #show table.cell.where(y: 0): strong
/// #set table(
///   stroke: (x, y) => if y == 0 {
///     (bottom: 0.7pt + black)
///   },
///   align: (x, y) => (
///     if x > 0 { center }
///     else { left }
///   )
/// )
///
/// #table(
///   columns: 3,
///   table.header(
///     [Substance],
///     [Subcritical °C],
///     [Supercritical °C],
///   ),
///   [Hydrochloric Acid],
///   [12.0], [92.1],
///   [Sodium Myreth Sulfate],
///   [16.6], [104],
///   [Potassium Hydroxide],
///   table.cell(colspan: 2)[24.7],
/// )
/// ```
///
/// For example, you can override the fill, alignment or inset for a single
/// cell:
///
/// ```example
/// >>> #set page(width: auto)
/// // You can also import those.
/// #import table: cell, header
///
/// #table(
///   columns: 2,
///   align: center,
///   header(
///     [*Trip progress*],
///     [*Itinerary*],
///   ),
///   cell(
///     align: right,
///     fill: fuchsia.lighten(80%),
///     [🚗],
///   ),
///   [Get in, folks!],
///   [🚗], [Eat curbside hotdog],
///   cell(align: left)[🌴🚗],
///   cell(
///     inset: 0.06em,
///     text(1.62em)[🏝️🌅🌊],
///   ),
/// )
/// ```
///
/// You may also apply a show rule on `table.cell` to style all cells at once.
/// Combined with selectors, this allows you to apply styles based on a cell's
/// position:
///
/// ```example
/// #show table.cell.where(x: 0): strong
///
/// #table(
///   columns: 3,
///   gutter: 3pt,
///   [Name], [Age], [Strength],
///   [Hannes], [36], [Grace],
///   [Irma], [50], [Resourcefulness],
///   [Vikram], [49], [Perseverance],
/// )
/// ```
#[elem(name = "cell", title = "Table Cell")]
pub struct TableCell {
    /// The cell's body.
    #[required]
    pub body: Content,

    /// The cell's column (zero-indexed).
    /// Functions identically to the `x` field in [`grid.cell`].
    pub x: Smart<usize>,

    /// The cell's row (zero-indexed).
    /// Functions identically to the `y` field in [`grid.cell`].
    pub y: Smart<usize>,

    /// The amount of columns spanned by this cell.
    #[default(NonZeroUsize::ONE)]
    pub colspan: NonZeroUsize,

    /// The amount of rows spanned by this cell.
    #[default(NonZeroUsize::ONE)]
    pub rowspan: NonZeroUsize,

    /// The cell's [inset]($table.inset) override.
    pub inset: Smart<Sides<Option<Rel<Length>>>>,

    /// The cell's [alignment]($table.align) override.
    pub align: Smart<Alignment>,

    /// The cell's [fill]($table.fill) override.
    pub fill: Smart<Option<Paint>>,

    /// The cell's [stroke]($table.stroke) override.
    #[fold]
    pub stroke: Sides<Option<Option<Arc<Stroke>>>>,

    /// Whether rows spanned by this cell can be placed in different pages.
    /// When equal to `{auto}`, a cell spanning only fixed-size rows is
    /// unbreakable, while a cell spanning at least one `{auto}`-sized row is
    /// breakable.
    pub breakable: Smart<bool>,
}

cast! {
    TableCell,
    v: Content => v.into(),
}

impl Default for Packed<TableCell> {
    fn default() -> Self {
        Packed::new(
            // Explicitly set colspan and rowspan to ensure they won't be
            // overridden by set rules (default cells are created after
            // colspans and rowspans are processed in the resolver)
            TableCell::new(Content::default())
                .with_colspan(NonZeroUsize::ONE)
                .with_rowspan(NonZeroUsize::ONE),
        )
    }
}

impl From<Content> for TableCell {
    fn from(value: Content) -> Self {
        #[allow(clippy::unwrap_or_default)]
        value.unpack::<Self>().unwrap_or_else(Self::new)
    }
}
