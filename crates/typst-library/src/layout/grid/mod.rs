pub mod resolve;

use std::num::{NonZeroU32, NonZeroUsize};
use std::sync::Arc;

use comemo::Track;
use smallvec::{SmallVec, smallvec};
use typst_utils::NonZeroExt;

use crate::diag::{At, HintedStrResult, HintedString, SourceResult, bail};
use crate::engine::Engine;
use crate::foundations::{
    Array, CastInfo, Content, Context, Fold, FromValue, Func, IntoValue, Packed, Reflect,
    Resolve, Smart, StyleChain, Value, cast, elem, scope,
};
use crate::layout::{
    Alignment, Length, OuterHAlignment, OuterVAlignment, Rel, Sides, Sizing,
};
use crate::model::{TableCell, TableFooter, TableHLine, TableHeader, TableVLine};
use crate::visualize::{Paint, Stroke};

/// Arranges content in a grid.
///
/// The grid element allows you to arrange content in a grid. You can define the
/// number of rows and columns, as well as the size of the gutters between them.
/// There are multiple sizing modes for columns and rows that can be used to
/// create complex layouts.
///
/// While the grid and table elements work very similarly, they are intended for
/// different use cases and carry different semantics. The grid element is
/// intended for presentational and layout purposes, while the
/// [`{table}`]($table) element is intended for, in broad terms, presenting
/// multiple related data points. In the future, Typst will annotate its output
/// such that screenreaders will announce content in `table` as tabular while a
/// grid's content will be announced no different than multiple content blocks
/// in the document flow. Set and show rules on one of these elements do not
/// affect the other.
///
/// # Sizing the tracks { #track-size }
///
/// A grid's sizing is determined by the track sizes specified in the arguments.
/// There are multiple sizing parameters: [`columns`]($grid.columns),
/// [`rows`]($grid.rows) and [`gutter`]($grid.gutter).
/// Because each of the sizing parameters accepts the same values, we will
/// explain them just once, here. Each sizing argument accepts an array of
/// individual track sizes. A track size is either:
///
/// - `{auto}`: The track will be sized to fit its contents. It will be at most
///   as large as the remaining space. If there is more than one `{auto}` track
///   width, and together they claim more than the available space, the `{auto}`
///   tracks will fairly distribute the available space among themselves.
///
/// - A fixed or relative length (e.g. `{10pt}` or `{20% - 1cm}`): The track
///   will be exactly of this size.
///
/// - A fractional length (e.g. `{1fr}`): Once all other tracks have been sized,
///   the remaining space will be divided among the fractional tracks according
///   to their fractions. For example, if there are two fractional tracks, each
///   with a fraction of `{1fr}`, they will each take up half of the remaining
///   space.
///
/// To specify a single track, the array can be omitted in favor of a single
/// value. To specify multiple `{auto}` tracks, enter the number of tracks
/// instead of an array. For example, `columns:` `{3}` is equivalent to
/// `columns:` `{(auto, auto, auto)}`.
///
/// # Examples
/// The example below demonstrates the different track sizing options. It also
/// shows how you can use [`grid.cell`] to make an individual cell span two grid
/// tracks.
///
/// ```example
/// // We use `rect` to emphasize the
/// // area of cells.
/// #set rect(
///   inset: 8pt,
///   fill: rgb("e4e5ea"),
///   width: 100%,
/// )
///
/// #grid(
///   columns: (60pt, 1fr, 2fr),
///   rows: (auto, 60pt),
///   gutter: 3pt,
///   rect[Fixed width, auto height],
///   rect[1/3 of the remains],
///   rect[2/3 of the remains],
///   rect(height: 100%)[Fixed height],
///   grid.cell(
///     colspan: 2,
///     image("tiger.jpg", width: 100%),
///   ),
/// )
/// ```
///
/// You can also [spread]($arguments/#spreading) an array of strings or content
/// into a grid to populate its cells.
///
/// ```example
/// #grid(
///   columns: 5,
///   gutter: 5pt,
///   ..range(25).map(str)
/// )
/// ```
///
/// # Styling the grid { #styling }
/// The grid and table elements work similarly. For a hands-on explanation,
/// refer to the [table guide]($guides/table-guide/#fills); for a quick overview,
/// continue reading.
///
/// The grid's appearance can be customized through different parameters. These
/// are the most important ones:
///
/// - [`align`]($grid.align) to change how cells are aligned
/// - [`inset`]($grid.inset) to optionally add internal padding to cells
/// - [`fill`]($grid.fill) to give cells a background
/// - [`stroke`]($grid.stroke) to optionally enable grid lines with a certain
///   stroke
///
/// To meet different needs, there are various ways to set them.
///
/// If you need to override the above options for individual cells, you can use
/// the [`grid.cell`] element. Likewise, you can override individual grid lines
/// with the [`grid.hline`] and [`grid.vline`] elements.
///
/// To configure an overall style for a grid, you may instead specify the option
/// in any of the following fashions:
///
/// - As a single value that applies to all cells.
/// - As an array of values corresponding to each column. The array will be
///   cycled if there are more columns than the array has items.
/// - As a function in the form of `(x, y) => value`. It receives the cell's
///   column and row indices (both starting from zero) and should return the
///   value to apply to that cell.
///
/// ```example
/// #grid(
///   columns: 5,
///
///   // By a single value
///   align: center,
///   // By a single but more complicated value
///   inset: (x: 2pt, y: 3pt),
///   // By an array of values (cycling)
///   fill: (rgb("#239dad50"), none),
///   // By a function that returns a value
///   stroke: (x, y) => if calc.rem(x + y, 3) == 0 { 0.5pt },
///
///   ..range(5 * 3).map(n => numbering("A", n + 1))
/// )
/// ```
///
/// On top of that, you may [apply styling rules]($styling) to [`grid`] and
/// [`grid.cell`]. Especially, the [`x`]($grid.cell.x) and [`y`]($grid.cell.y)
/// fields of `grid.cell` can be used in a [`where`]($function.where) selector,
/// making it possible to style cells at specific columns or rows, or individual
/// positions.
///
/// ## Stroke styling precedence
/// As explained above, there are three ways to set the stroke of a grid cell:
/// through [`{grid.cell}`'s `stroke` field]($grid.cell.stroke), by using
/// [`{grid.hline}`]($grid.hline) and [`{grid.vline}`]($grid.vline), or by
/// setting the [`{grid}`'s `stroke` field]($grid.stroke). When multiple of
/// these settings are present and conflict, the `hline` and `vline` settings
/// take the highest precedence, followed by the `cell` settings, and finally
/// the `grid` settings.
///
/// Furthermore, strokes of a repeated grid header or footer will take
/// precedence over regular cell strokes.
#[elem(scope)]
pub struct GridElem {
    /// The column sizes.
    ///
    /// Either specify a track size array or provide an integer to create a grid
    /// with that many `{auto}`-sized columns. Note that opposed to rows and
    /// gutters, providing a single track size will only ever create a single
    /// column.
    ///
    /// See the [track size section](#track-size) above for more details.
    pub columns: TrackSizings,

    /// The row sizes.
    ///
    /// If there are more cells than fit the defined rows, the last row is
    /// repeated until there are no more cells.
    ///
    /// See the [track size section](#track-size) above for more details.
    pub rows: TrackSizings,

    /// The gaps between rows and columns. This is a shorthand to set
    /// [`column-gutter`]($grid.column-gutter) and [`row-gutter`]($grid.row-gutter)
    /// to the same value.
    ///
    /// If there are more gutters than defined sizes, the last gutter is
    /// repeated.
    ///
    /// See the [track size section](#track-size) above for more details.
    #[external]
    pub gutter: TrackSizings,

    /// The gaps between columns.
    #[parse(
        let gutter = args.named("gutter")?;
        args.named("column-gutter")?.or_else(|| gutter.clone())
    )]
    pub column_gutter: TrackSizings,

    /// The gaps between rows.
    #[parse(args.named("row-gutter")?.or_else(|| gutter.clone()))]
    pub row_gutter: TrackSizings,

    /// How much to pad the cells' content.
    ///
    /// To specify a uniform inset for all cells, you can use a single length
    /// for all sides, or a dictionary of lengths for individual sides. See the
    /// [box's documentation]($box.inset) for more details.
    ///
    /// To specify varying inset for different cells, you can:
    /// - use a single inset for all cells
    /// - use an array of insets corresponding to each column
    /// - use a function that maps a cell's position to its inset
    ///
    /// See the [styling section](#styling) above for more details.
    ///
    /// In addition, you can find an example at the [`table.inset`] parameter.
    #[fold]
    pub inset: Celled<Sides<Option<Rel<Length>>>>,

    /// How to align the cells' content.
    ///
    /// If set to `{auto}`, the outer alignment is used.
    ///
    /// You can specify the alignment in any of the following fashions:
    /// - use a single alignment for all cells
    /// - use an array of alignments corresponding to each column
    /// - use a function that maps a cell's position to its alignment
    ///
    /// See the [styling section](#styling) above for details.
    ///
    /// In addition, you can find an example at the [`table.align`] parameter.
    pub align: Celled<Smart<Alignment>>,

    /// How to fill the cells.
    ///
    /// This can be:
    /// - a single color for all cells
    /// - an array of colors corresponding to each column
    /// - a function that maps a cell's position to its color
    ///
    /// Most notably, arrays and functions are useful for creating striped grids.
    /// See the [styling section](#styling) above for more details.
    ///
    /// ```example
    /// #grid(
    ///   fill: (x, y) =>
    ///     if calc.even(x + y) { luma(230) }
    ///     else { white },
    ///   align: center + horizon,
    ///   columns: 4,
    ///   inset: 2pt,
    ///   [X], [O], [X], [O],
    ///   [O], [X], [O], [X],
    ///   [X], [O], [X], [O],
    ///   [O], [X], [O], [X],
    /// )
    /// ```
    pub fill: Celled<Option<Paint>>,

    /// How to [stroke]($stroke) the cells.
    ///
    /// Grids have no strokes by default, which can be changed by setting this
    /// option to the desired stroke.
    ///
    /// If it is necessary to place lines which can cross spacing between cells
    /// produced by the [`gutter`]($grid.gutter) option, or to override the
    /// stroke between multiple specific cells, consider specifying one or more
    /// of [`grid.hline`] and [`grid.vline`] alongside your grid cells.
    ///
    /// To specify the same stroke for all cells, you can use a single [stroke]
    /// for all sides, or a dictionary of [strokes]($stroke) for individual
    /// sides. See the [rectangle's documentation]($rect.stroke) for more
    /// details.
    ///
    /// To specify varying strokes for different cells, you can:
    /// - use a single stroke for all cells
    /// - use an array of strokes corresponding to each column
    /// - use a function that maps a cell's position to its stroke
    ///
    /// See the [styling section](#styling) above for more details.
    ///
    /// ```example
    /// #set page(width: 420pt)
    /// #set text(number-type: "old-style")
    /// #show grid.cell.where(y: 0): set text(size: 1.3em)
    ///
    /// #grid(
    ///   columns: (1fr, 2fr, 2fr),
    ///   row-gutter: 1.5em,
    ///   inset: (left: 0.5em),
    ///   stroke: (x, y) => if x > 0 { (left: 0.5pt + gray) },
    ///   align: horizon,
    ///
    ///   [Winter \ 2007 \ Season],
    ///   [Aaron Copland \ *The Tender Land* \ January 2007],
    ///   [Eric Satie \ *Gymnopedie 1, 2* \ February 2007],
    ///
    ///   [],
    ///   [Jan 12 \ *Middlebury College \ Center for the Arts* \ 20:00],
    ///   [Feb 2 \ *Johnson State College Dibden Center for the Arts* \ 19:30],
    ///
    ///   [],
    ///   [Skip a week \ #text(0.8em)[_Prepare your exams!_]],
    ///   [Feb 9 \ *Castleton State College \ Fine Arts Center* \ 19:30],
    ///
    ///   [],
    ///   [Jan 26, 27 \ *Lyndon State College Alexander Twilight Theater* \ 20:00],
    ///   [
    ///     Feb 17 --- #smallcaps[Anniversary] \
    ///     *Middlebury College \ Center for the Arts* \
    ///     19:00 #text(0.7em)[(for a special guest)]
    ///   ],
    /// )
    /// ```
    ///
    /// ```example
    /// #set page(height: 13em, width: 26em)
    ///
    /// #let cv(..jobs) = grid(
    ///   columns: 2,
    ///   inset: 5pt,
    ///   stroke: (x, y) => if x == 0 and y > 0 {
    ///     (right: (
    ///       paint: luma(180),
    ///       thickness: 1.5pt,
    ///       dash: "dotted"
    ///     ))
    ///   },
    ///   grid.header(grid.cell(colspan: 2)[
    ///     *Professional Experience*
    ///     #box(width: 1fr, line(length: 100%, stroke: luma(180)))
    ///   ]),
    ///   ..{
    ///     let last = none
    ///     for job in jobs.pos() {
    ///       (
    ///         if job.year != last [*#job.year*],
    ///         [
    ///           *#job.company* - #job.role _(#job.timeframe)_ \
    ///           #job.details
    ///         ]
    ///       )
    ///       last = job.year
    ///     }
    ///   }
    /// )
    ///
    /// #cv(
    ///   (
    ///     year: 2012,
    ///     company: [Pear Seed & Co.],
    ///     role: [Lead Engineer],
    ///     timeframe: [Jul - Dec],
    ///     details: [
    ///       - Raised engineers from 3x to 10x
    ///       - Did a great job
    ///     ],
    ///   ),
    ///   (
    ///     year: 2012,
    ///     company: [Mega Corp.],
    ///     role: [VP of Sales],
    ///     timeframe: [Mar - Jun],
    ///     details: [- Closed tons of customers],
    ///   ),
    ///   (
    ///     year: 2013,
    ///     company: [Tiny Co.],
    ///     role: [CEO],
    ///     timeframe: [Jan - Dec],
    ///     details: [- Delivered 4x more shareholder value],
    ///   ),
    ///   (
    ///     year: 2014,
    ///     company: [Glorbocorp Ltd],
    ///     role: [CTO],
    ///     timeframe: [Jan - Mar],
    ///     details: [- Drove containerization forward],
    ///   ),
    /// )
    /// ```
    #[fold]
    pub stroke: Celled<Sides<Option<Option<Arc<Stroke>>>>>,

    /// The contents of the grid cells, plus any extra grid lines specified with
    /// the [`grid.hline`] and [`grid.vline`] elements.
    ///
    /// The cells are populated in row-major order.
    #[variadic]
    pub children: Vec<GridChild>,
}

#[scope]
impl GridElem {
    #[elem]
    type GridCell;

    #[elem]
    type GridHLine;

    #[elem]
    type GridVLine;

    #[elem]
    type GridHeader;

    #[elem]
    type GridFooter;
}

/// Track sizing definitions.
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct TrackSizings(pub SmallVec<[Sizing; 4]>);

cast! {
    TrackSizings,
    self => self.0.into_value(),
    sizing: Sizing => Self(smallvec![sizing]),
    count: NonZeroUsize => Self(smallvec![Sizing::Auto; count.get()]),
    values: Array => Self(values.into_iter().map(Value::cast).collect::<HintedStrResult<_>>()?),
}

/// Any child of a grid element.
#[derive(Debug, PartialEq, Clone, Hash)]
pub enum GridChild {
    Header(Packed<GridHeader>),
    Footer(Packed<GridFooter>),
    Item(GridItem),
}

cast! {
    GridChild,
    self => match self {
        Self::Header(header) => header.into_value(),
        Self::Footer(footer) => footer.into_value(),
        Self::Item(item) => item.into_value(),
    },
    v: Content => {
        v.try_into()?
    },
}

impl TryFrom<Content> for GridChild {
    type Error = HintedString;
    fn try_from(value: Content) -> HintedStrResult<Self> {
        if value.is::<TableHeader>() {
            bail!(
                "cannot use `table.header` as a grid header";
                hint: "use `grid.header` instead"
            )
        }
        if value.is::<TableFooter>() {
            bail!(
                "cannot use `table.footer` as a grid footer";
                hint: "use `grid.footer` instead"
            )
        }

        value
            .into_packed::<GridHeader>()
            .map(Self::Header)
            .or_else(|value| value.into_packed::<GridFooter>().map(Self::Footer))
            .or_else(|value| GridItem::try_from(value).map(Self::Item))
    }
}

/// A grid item, which is the basic unit of grid specification.
#[derive(Debug, PartialEq, Clone, Hash)]
pub enum GridItem {
    HLine(Packed<GridHLine>),
    VLine(Packed<GridVLine>),
    Cell(Packed<GridCell>),
}

cast! {
    GridItem,
    self => match self {
        Self::HLine(hline) => hline.into_value(),
        Self::VLine(vline) => vline.into_value(),
        Self::Cell(cell) => cell.into_value(),
    },
    v: Content => {
        v.try_into()?
    }
}

impl TryFrom<Content> for GridItem {
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
        if value.is::<TableCell>() {
            bail!(
                "cannot use `table.cell` as a grid cell";
                hint: "use `grid.cell` instead"
            );
        }
        if value.is::<TableHLine>() {
            bail!(
                "cannot use `table.hline` as a grid line";
                hint: "use `grid.hline` instead"
            );
        }
        if value.is::<TableVLine>() {
            bail!(
                "cannot use `table.vline` as a grid line";
                hint: "use `grid.vline` instead"
            );
        }

        Ok(value
            .into_packed::<GridHLine>()
            .map(Self::HLine)
            .or_else(|value| value.into_packed::<GridVLine>().map(Self::VLine))
            .or_else(|value| value.into_packed::<GridCell>().map(Self::Cell))
            .unwrap_or_else(|value| {
                let span = value.span();
                Self::Cell(Packed::new(GridCell::new(value)).spanned(span))
            }))
    }
}

/// A repeatable grid header.
///
/// If `repeat` is set to `true`, the header will be repeated across pages. For
/// an example, refer to the [`table.header`] element and the [`grid.stroke`]
/// parameter.
#[elem(name = "header", title = "Grid Header")]
pub struct GridHeader {
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
    pub children: Vec<GridItem>,
}

/// A repeatable grid footer.
///
/// Just like the [`grid.header`] element, the footer can repeat itself on every
/// page of the table.
///
/// No other grid cells may be placed after the footer.
#[elem(name = "footer", title = "Grid Footer")]
pub struct GridFooter {
    /// Whether this footer should be repeated across pages.
    #[default(true)]
    pub repeat: bool,

    /// The cells and lines within the footer.
    #[variadic]
    pub children: Vec<GridItem>,
}

/// A horizontal line in the grid.
///
/// Overrides any per-cell stroke, including stroke specified through the grid's
/// `stroke` field. Can cross spacing between cells created through the grid's
/// `column-gutter` option.
///
/// An example for this function can be found at the [`table.hline`] element.
#[elem(name = "hline", title = "Grid Horizontal Line")]
pub struct GridHLine {
    /// The row above which the horizontal line is placed (zero-indexed).
    /// If the `position` field is set to `{bottom}`, the line is placed below
    /// the row with the given index instead (see [`grid.hline.position`] for
    /// details).
    ///
    /// Specifying `{auto}` causes the line to be placed at the row below the
    /// last automatically positioned cell (that is, cell without coordinate
    /// overrides) before the line among the grid's children. If there is no
    /// such cell before the line, it is placed at the top of the grid (row 0).
    /// Note that specifying for this option exactly the total amount of rows
    /// in the grid causes this horizontal line to override the bottom border
    /// of the grid, while a value of 0 overrides the top border.
    pub y: Smart<usize>,

    /// The column at which the horizontal line starts (zero-indexed, inclusive).
    pub start: usize,

    /// The column before which the horizontal line ends (zero-indexed,
    /// exclusive).
    /// Therefore, the horizontal line will be drawn up to and across column
    /// `end - 1`.
    ///
    /// A value equal to `{none}` or to the amount of columns causes it to
    /// extend all the way towards the end of the grid.
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

/// A vertical line in the grid.
///
/// Overrides any per-cell stroke, including stroke specified through the
/// grid's `stroke` field. Can cross spacing between cells created through
/// the grid's `row-gutter` option.
#[elem(name = "vline", title = "Grid Vertical Line")]
pub struct GridVLine {
    /// The column before which the vertical line is placed (zero-indexed).
    /// If the `position` field is set to `{end}`, the line is placed after the
    /// column with the given index instead (see [`grid.vline.position`] for
    /// details).
    ///
    /// Specifying `{auto}` causes the line to be placed at the column after
    /// the last automatically positioned cell (that is, cell without
    /// coordinate overrides) before the line among the grid's children. If
    /// there is no such cell before the line, it is placed before the grid's
    /// first column (column 0).
    /// Note that specifying for this option exactly the total amount of
    /// columns in the grid causes this vertical line to override the end
    /// border of the grid (right in LTR, left in RTL), while a value of 0
    /// overrides the start border (left in LTR, right in RTL).
    pub x: Smart<usize>,

    /// The row at which the vertical line starts (zero-indexed, inclusive).
    pub start: usize,

    /// The row on top of which the vertical line ends (zero-indexed,
    /// exclusive).
    /// Therefore, the vertical line will be drawn up to and across row
    /// `end - 1`.
    ///
    /// A value equal to `{none}` or to the amount of rows causes it to extend
    /// all the way towards the bottom of the grid.
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
    /// they cause your grid to be inconsistent between left-to-right and
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

/// A cell in the grid. You can use this function in the argument list of a grid
/// to override grid style properties for an individual cell or manually
/// positioning it within the grid. You can also use this function in show rules
/// to apply certain styles to multiple cells at once.
///
/// For example, you can override the position and stroke for a single cell:
///
/// ```example
/// >>> #set page(width: auto)
/// >>> #set text(15pt, font: "Noto Sans Symbols 2", bottom-edge: -.2em)
/// <<< #set text(15pt, font: "Noto Sans Symbols 2")
/// #show regex("[♚-♟︎]"): set text(fill: rgb("21212A"))
/// #show regex("[♔-♙]"): set text(fill: rgb("111015"))
///
/// #grid(
///   fill: (x, y) => rgb(
///     if calc.odd(x + y) { "7F8396" }
///     else { "EFF0F3" }
///   ),
///   columns: (1em,) * 8,
///   rows: 1em,
///   align: center + horizon,
///
///   [♖], [♘], [♗], [♕], [♔], [♗], [♘], [♖],
///   [♙], [♙], [♙], [♙], [],  [♙], [♙], [♙],
///   grid.cell(
///     x: 4, y: 3,
///     stroke: blue.transparentize(60%)
///   )[♙],
///
///   ..(grid.cell(y: 6)[♟],) * 8,
///   ..([♜], [♞], [♝], [♛], [♚], [♝], [♞], [♜])
///     .map(grid.cell.with(y: 7)),
/// )
/// ```
///
/// You may also apply a show rule on `grid.cell` to style all cells at once,
/// which allows you, for example, to apply styles based on a cell's position.
/// Refer to the examples of the [`table.cell`] element to learn more about
/// this.
#[elem(name = "cell", title = "Grid Cell")]
pub struct GridCell {
    /// The cell's body.
    #[required]
    pub body: Content,

    /// The cell's column (zero-indexed).
    /// This field may be used in show rules to style a cell depending on its
    /// column.
    ///
    /// You may override this field to pick in which column the cell must
    /// be placed. If no row (`y`) is chosen, the cell will be placed in the
    /// first row (starting at row 0) with that column available (or a new row
    /// if none). If both `x` and `y` are chosen, however, the cell will be
    /// placed in that exact position. An error is raised if that position is
    /// not available (thus, it is usually wise to specify cells with a custom
    /// position before cells with automatic positions).
    ///
    /// ```example
    /// #let circ(c) = circle(
    ///     fill: c, width: 5mm
    /// )
    ///
    /// #grid(
    ///   columns: 4,
    ///   rows: 7mm,
    ///   stroke: .5pt + blue,
    ///   align: center + horizon,
    ///   inset: 1mm,
    ///
    ///   grid.cell(x: 2, y: 2, circ(aqua)),
    ///   circ(yellow),
    ///   grid.cell(x: 3, circ(green)),
    ///   circ(black),
    /// )
    /// ```
    pub x: Smart<usize>,

    /// The cell's row (zero-indexed).
    /// This field may be used in show rules to style a cell depending on its
    /// row.
    ///
    /// You may override this field to pick in which row the cell must be
    /// placed. If no column (`x`) is chosen, the cell will be placed in the
    /// first column (starting at column 0) available in the chosen row. If all
    /// columns in the chosen row are already occupied, an error is raised.
    ///
    /// ```example
    /// #let tri(c) = polygon.regular(
    ///   fill: c,
    ///   size: 5mm,
    ///   vertices: 3,
    /// )
    ///
    /// #grid(
    ///   columns: 2,
    ///   stroke: blue,
    ///   inset: 1mm,
    ///
    ///   tri(black),
    ///   grid.cell(y: 1, tri(teal)),
    ///   grid.cell(y: 1, tri(red)),
    ///   grid.cell(y: 2, tri(orange))
    /// )
    /// ```
    pub y: Smart<usize>,

    /// The amount of columns spanned by this cell.
    #[default(NonZeroUsize::ONE)]
    pub colspan: NonZeroUsize,

    /// The amount of rows spanned by this cell.
    #[default(NonZeroUsize::ONE)]
    pub rowspan: NonZeroUsize,

    /// The cell's [inset]($grid.inset) override.
    pub inset: Smart<Sides<Option<Rel<Length>>>>,

    /// The cell's [alignment]($grid.align) override.
    pub align: Smart<Alignment>,

    /// The cell's [fill]($grid.fill) override.
    pub fill: Smart<Option<Paint>>,

    /// The cell's [stroke]($grid.stroke) override.
    #[fold]
    pub stroke: Sides<Option<Option<Arc<Stroke>>>>,

    /// Whether rows spanned by this cell can be placed in different pages.
    /// When equal to `{auto}`, a cell spanning only fixed-size rows is
    /// unbreakable, while a cell spanning at least one `{auto}`-sized row is
    /// breakable.
    pub breakable: Smart<bool>,
}

cast! {
    GridCell,
    v: Content => v.into(),
}

impl Default for Packed<GridCell> {
    fn default() -> Self {
        Packed::new(
            // Explicitly set colspan and rowspan to ensure they won't be
            // overridden by set rules (default cells are created after
            // colspans and rowspans are processed in the resolver)
            GridCell::new(Content::default())
                .with_colspan(NonZeroUsize::ONE)
                .with_rowspan(NonZeroUsize::ONE),
        )
    }
}

impl From<Content> for GridCell {
    fn from(value: Content) -> Self {
        #[allow(clippy::unwrap_or_default)]
        value.unpack::<Self>().unwrap_or_else(Self::new)
    }
}

/// A value that can be configured per cell.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Celled<T> {
    /// A bare value, the same for all cells.
    Value(T),
    /// A closure mapping from cell coordinates to a value.
    Func(Func),
    /// An array of values corresponding to each column. The array will be
    /// cycled if there are more columns than the array has items.
    Array(Vec<T>),
}

impl<T: Default + Clone + FromValue> Celled<T> {
    /// Resolve the value based on the cell position.
    pub fn resolve(
        &self,
        engine: &mut Engine,
        styles: StyleChain,
        x: usize,
        y: usize,
    ) -> SourceResult<T> {
        Ok(match self {
            Self::Value(value) => value.clone(),
            Self::Func(func) => func
                .call(engine, Context::new(None, Some(styles)).track(), [x, y])?
                .cast()
                .at(func.span())?,
            Self::Array(array) => x
                .checked_rem(array.len())
                .and_then(|i| array.get(i))
                .cloned()
                .unwrap_or_default(),
        })
    }
}

impl<T: Default> Default for Celled<T> {
    fn default() -> Self {
        Self::Value(T::default())
    }
}

impl<T: Reflect> Reflect for Celled<T> {
    fn input() -> CastInfo {
        T::input() + Array::input() + Func::input()
    }

    fn output() -> CastInfo {
        T::output() + Array::output() + Func::output()
    }

    fn castable(value: &Value) -> bool {
        Array::castable(value) || Func::castable(value) || T::castable(value)
    }
}

impl<T: IntoValue> IntoValue for Celled<T> {
    fn into_value(self) -> Value {
        match self {
            Self::Value(value) => value.into_value(),
            Self::Func(func) => func.into_value(),
            Self::Array(arr) => arr.into_value(),
        }
    }
}

impl<T: FromValue> FromValue for Celled<T> {
    fn from_value(value: Value) -> HintedStrResult<Self> {
        match value {
            Value::Func(v) => Ok(Self::Func(v)),
            Value::Array(array) => Ok(Self::Array(
                array.into_iter().map(T::from_value).collect::<HintedStrResult<_>>()?,
            )),
            v if T::castable(&v) => Ok(Self::Value(T::from_value(v)?)),
            v => Err(Self::error(&v)),
        }
    }
}

impl<T: Fold> Fold for Celled<T> {
    fn fold(self, outer: Self) -> Self {
        match (self, outer) {
            (Self::Value(inner), Self::Value(outer)) => Self::Value(inner.fold(outer)),
            (self_, _) => self_,
        }
    }
}

impl<T: Resolve> Resolve for Celled<T> {
    type Output = ResolvedCelled<T>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        match self {
            Self::Value(value) => ResolvedCelled(Celled::Value(value.resolve(styles))),
            Self::Func(func) => ResolvedCelled(Celled::Func(func)),
            Self::Array(values) => ResolvedCelled(Celled::Array(
                values.into_iter().map(|value| value.resolve(styles)).collect(),
            )),
        }
    }
}

/// The result of resolving a Celled's value according to styles.
/// Holds resolved values which depend on each grid cell's position.
/// When it is a closure, however, it is only resolved when the closure is
/// called.
#[derive(Default, Clone)]
pub struct ResolvedCelled<T: Resolve>(Celled<T::Output>);

impl<T> ResolvedCelled<T>
where
    T: FromValue + Resolve,
    <T as Resolve>::Output: Default + Clone,
{
    /// Resolve the value based on the cell position.
    pub fn resolve(
        &self,
        engine: &mut Engine,
        styles: StyleChain,
        x: usize,
        y: usize,
    ) -> SourceResult<T::Output> {
        Ok(match &self.0 {
            Celled::Value(value) => value.clone(),
            Celled::Func(func) => func
                .call(engine, Context::new(None, Some(styles)).track(), [x, y])?
                .cast::<T>()
                .at(func.span())?
                .resolve(styles),
            Celled::Array(array) => x
                .checked_rem(array.len())
                .and_then(|i| array.get(i))
                .cloned()
                .unwrap_or_default(),
        })
    }
}
