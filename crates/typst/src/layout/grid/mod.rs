mod cells;
mod layout;
mod lines;
mod repeated;
mod rowspans;

pub use self::cells::{
    Cell, CellGrid, Celled, ResolvableCell, ResolvableGridChild, ResolvableGridItem,
};
pub use self::layout::GridLayouter;
pub use self::lines::LinePosition;

use std::num::NonZeroUsize;
use std::sync::Arc;

use ecow::eco_format;
use smallvec::{smallvec, SmallVec};

use crate::diag::{bail, HintedStrResult, HintedString, SourceResult, Trace, Tracepoint};
use crate::engine::Engine;
use crate::foundations::{
    cast, elem, scope, Array, Content, Fold, NativeElement, Packed, Show, Smart,
    StyleChain, Value,
};
use crate::introspection::Locator;
use crate::layout::{
    Abs, Alignment, Axes, BlockElem, Dir, Fragment, Length, OuterHAlignment,
    OuterVAlignment, Regions, Rel, Sides, Sizing,
};
use crate::model::{TableCell, TableFooter, TableHLine, TableHeader, TableVLine};
use crate::syntax::Span;
use crate::text::TextElem;
use crate::utils::NonZeroExt;
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
/// A grid's sizing is determined by the track sizes specified in the arguments.
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
/// shows how you can use [`grid.cell`]($grid.cell) to make an individual cell
/// span two grid tracks.
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
/// # Styling the grid
/// The grid's appearance can be customized through different parameters. These
/// are the most important ones:
///
/// - [`fill`]($grid.fill) to give all cells a background
/// - [`align`]($grid.align) to change how cells are aligned
/// - [`inset`]($grid.inset) to optionally add internal padding to each cell
/// - [`stroke`]($grid.stroke) to optionally enable grid lines with a certain
///   stroke
///
/// If you need to override one of the above options for a single cell, you can
/// use the [`grid.cell`]($grid.cell) element. Likewise, you can override
/// individual grid lines with the [`grid.hline`]($grid.hline) and
/// [`grid.vline`]($grid.vline) elements.
///
/// Alternatively, if you need the appearance options to depend on a cell's
/// position (column and row), you may specify a function to `fill` or `align`
/// of the form `(column, row) => value`. You may also use a show rule on
/// [`grid.cell`]($grid.cell) - see that element's examples or the examples
/// below for more information.
///
/// Locating most of your styling in set and show rules is recommended, as it
/// keeps the grid's or table's actual usages clean and easy to read. It also
/// allows you to easily change the grid's appearance in one place.
///
/// ## Stroke styling precedence
/// There are three ways to set the stroke of a grid cell: through
/// [`{grid.cell}`'s `stroke` field]($grid.cell.stroke), by using
/// [`{grid.hline}`]($grid.hline) and [`{grid.vline}`]($grid.vline), or by
/// setting the [`{grid}`'s `stroke` field]($grid.stroke). When multiple of
/// these settings are present and conflict, the `hline` and `vline` settings
/// take the highest precedence, followed by the `cell` settings, and finally
/// the `grid` settings.
///
/// Furthermore, strokes of a repeated grid header or footer will take
/// precedence over regular cell strokes.
#[elem(scope, Show)]
pub struct GridElem {
    /// The column sizes.
    ///
    /// Either specify a track size array or provide an integer to create a grid
    /// with that many `{auto}`-sized columns. Note that opposed to rows and
    /// gutters, providing a single track size will only ever create a single
    /// column.
    #[borrowed]
    pub columns: TrackSizings,

    /// The row sizes.
    ///
    /// If there are more cells than fit the defined rows, the last row is
    /// repeated until there are no more cells.
    #[borrowed]
    pub rows: TrackSizings,

    /// The gaps between rows and columns.
    ///
    /// If there are more gutters than defined sizes, the last gutter is
    /// repeated.
    ///
    /// This is a shorthand to set `column-gutter` and `row-gutter` to the same
    /// value.
    #[external]
    pub gutter: TrackSizings,

    /// The gaps between columns.
    #[parse(
        let gutter = args.named("gutter")?;
        args.named("column-gutter")?.or_else(|| gutter.clone())
    )]
    #[borrowed]
    pub column_gutter: TrackSizings,

    /// The gaps between rows.
    #[parse(args.named("row-gutter")?.or_else(|| gutter.clone()))]
    #[borrowed]
    pub row_gutter: TrackSizings,

    /// How to fill the cells.
    ///
    /// This can be a color or a function that returns a color. The function
    /// receives the cells' column and row indices, starting from zero. This can
    /// be used to implement striped grids.
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
    #[borrowed]
    pub fill: Celled<Option<Paint>>,

    /// How to align the cells' content.
    ///
    /// This can either be a single alignment, an array of alignments
    /// (corresponding to each column) or a function that returns an alignment.
    /// The function receives the cells' column and row indices, starting from
    /// zero. If set to `{auto}`, the outer alignment is used.
    ///
    /// You can find an example for this argument at the
    /// [`table.align`]($table.align) parameter.
    #[borrowed]
    pub align: Celled<Smart<Alignment>>,

    /// How to [stroke]($stroke) the cells.
    ///
    /// Grids have no strokes by default, which can be changed by setting this
    /// option to the desired stroke.
    ///
    /// If it is necessary to place lines which can cross spacing between cells
    /// produced by the `gutter` option, or to override the stroke between
    /// multiple specific cells, consider specifying one or more of
    /// [`grid.hline`]($grid.hline) and [`grid.vline`]($grid.vline) alongside
    /// your grid cells.
    ///
    /// ```example
    /// #set page(height: 13em, width: 26em)
    ///
    /// #let cv(..jobs) = grid(
    ///     columns: 2,
    ///     inset: 5pt,
    ///     stroke: (x, y) => if x == 0 and y > 0 {
    ///       (right: (
    ///         paint: luma(180),
    ///         thickness: 1.5pt,
    ///         dash: "dotted"
    ///       ))
    ///     },
    ///     grid.header(grid.cell(colspan: 2)[
    ///       *Professional Experience*
    ///       #box(width: 1fr, line(length: 100%, stroke: luma(180)))
    ///     ]),
    ///     ..{
    ///       let last = none
    ///       for job in jobs.pos() {
    ///         (
    ///           if job.year != last [*#job.year*],
    ///           [
    ///             *#job.company* - #job.role _(#job.timeframe)_ \
    ///             #job.details
    ///           ]
    ///         )
    ///         last = job.year
    ///       }
    ///     }
    ///   )
    ///
    ///   #cv(
    ///     (
    ///       year: 2012,
    ///       company: [Pear Seed & Co.],
    ///       role: [Lead Engineer],
    ///       timeframe: [Jul - Dec],
    ///       details: [
    ///         - Raised engineers from 3x to 10x
    ///         - Did a great job
    ///       ],
    ///     ),
    ///     (
    ///       year: 2012,
    ///       company: [Mega Corp.],
    ///       role: [VP of Sales],
    ///       timeframe: [Mar - Jun],
    ///       details: [- Closed tons of customers],
    ///     ),
    ///     (
    ///       year: 2013,
    ///       company: [Tiny Co.],
    ///       role: [CEO],
    ///       timeframe: [Jan - Dec],
    ///       details: [- Delivered 4x more shareholder value],
    ///     ),
    ///     (
    ///       year: 2014,
    ///       company: [Glorbocorp Ltd],
    ///       role: [CTO],
    ///       timeframe: [Jan - Mar],
    ///       details: [- Drove containerization forward],
    ///     ),
    ///   )
    /// ```
    #[resolve]
    #[fold]
    pub stroke: Celled<Sides<Option<Option<Arc<Stroke>>>>>,

    /// How much to pad the cells' content.
    ///
    /// You can find an example for this argument at the
    /// [`table.inset`]($table.inset) parameter.
    #[fold]
    pub inset: Celled<Sides<Option<Rel<Length>>>>,

    /// The contents of the grid cells, plus any extra grid lines specified
    /// with the [`grid.hline`]($grid.hline) and [`grid.vline`]($grid.vline)
    /// elements.
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

impl Show for Packed<GridElem> {
    fn show(&self, _: &mut Engine, _: StyleChain) -> SourceResult<Content> {
        Ok(BlockElem::multi_layouter(self.clone(), layout_grid)
            .pack()
            .spanned(self.span()))
    }
}

/// Layout the grid.
#[typst_macros::time(span = elem.span())]
fn layout_grid(
    elem: &Packed<GridElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    regions: Regions,
) -> SourceResult<Fragment> {
    let inset = elem.inset(styles);
    let align = elem.align(styles);
    let columns = elem.columns(styles);
    let rows = elem.rows(styles);
    let column_gutter = elem.column_gutter(styles);
    let row_gutter = elem.row_gutter(styles);
    let fill = elem.fill(styles);
    let stroke = elem.stroke(styles);

    let tracks = Axes::new(columns.0.as_slice(), rows.0.as_slice());
    let gutter = Axes::new(column_gutter.0.as_slice(), row_gutter.0.as_slice());
    // Use trace to link back to the grid when a specific cell errors
    let tracepoint = || Tracepoint::Call(Some(eco_format!("grid")));
    let resolve_item = |item: &GridItem| item.to_resolvable(styles);
    let children = elem.children().iter().map(|child| match child {
        GridChild::Header(header) => ResolvableGridChild::Header {
            repeat: header.repeat(styles),
            span: header.span(),
            items: header.children().iter().map(resolve_item),
        },
        GridChild::Footer(footer) => ResolvableGridChild::Footer {
            repeat: footer.repeat(styles),
            span: footer.span(),
            items: footer.children().iter().map(resolve_item),
        },
        GridChild::Item(item) => ResolvableGridChild::Item(item.to_resolvable(styles)),
    });
    let grid = CellGrid::resolve(
        tracks,
        gutter,
        locator,
        children,
        fill,
        align,
        &inset,
        &stroke,
        engine,
        styles,
        elem.span(),
    )
    .trace(engine.world, tracepoint, elem.span())?;

    let layouter = GridLayouter::new(&grid, regions, styles, elem.span());

    // Measure the columns and layout the grid row-by-row.
    layouter.layout(engine)
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

impl GridItem {
    fn to_resolvable(&self, styles: StyleChain) -> ResolvableGridItem<Packed<GridCell>> {
        match self {
            Self::HLine(hline) => ResolvableGridItem::HLine {
                y: hline.y(styles),
                start: hline.start(styles),
                end: hline.end(styles),
                stroke: hline.stroke(styles),
                span: hline.span(),
                position: match hline.position(styles) {
                    OuterVAlignment::Top => LinePosition::Before,
                    OuterVAlignment::Bottom => LinePosition::After,
                },
            },
            Self::VLine(vline) => ResolvableGridItem::VLine {
                x: vline.x(styles),
                start: vline.start(styles),
                end: vline.end(styles),
                stroke: vline.stroke(styles),
                span: vline.span(),
                position: match vline.position(styles) {
                    OuterHAlignment::Left if TextElem::dir_in(styles) == Dir::RTL => {
                        LinePosition::After
                    }
                    OuterHAlignment::Right if TextElem::dir_in(styles) == Dir::RTL => {
                        LinePosition::Before
                    }
                    OuterHAlignment::Start | OuterHAlignment::Left => {
                        LinePosition::Before
                    }
                    OuterHAlignment::End | OuterHAlignment::Right => LinePosition::After,
                },
            },
            Self::Cell(cell) => ResolvableGridItem::Cell(cell.clone()),
        }
    }
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
/// an example, refer to the [`table.header`]($table.header) element and the
/// [`grid.stroke`]($grid.stroke) parameter.
#[elem(name = "header", title = "Grid Header")]
pub struct GridHeader {
    /// Whether this header should be repeated across pages.
    #[default(true)]
    pub repeat: bool,

    /// The cells and lines within the header.
    #[variadic]
    pub children: Vec<GridItem>,
}

/// A repeatable grid footer.
///
/// Just like the [`grid.header`]($grid.header) element, the footer can repeat
/// itself on every page of the table.
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
/// An example for this function can be found at the
/// [`table.hline`]($table.hline) element.
#[elem(name = "hline", title = "Grid Horizontal Line")]
pub struct GridHLine {
    /// The row above which the horizontal line is placed (zero-indexed).
    /// If the `position` field is set to `{bottom}`, the line is placed below
    /// the row with the given index instead (see that field's docs for
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
    #[resolve]
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
    /// The column before which the horizontal line is placed (zero-indexed).
    /// If the `position` field is set to `{end}`, the line is placed after the
    /// column with the given index instead (see that field's docs for
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
    #[resolve]
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
/// Refer to the examples of the [`table.cell`]($table.cell) element to learn
/// more about this.
#[elem(name = "cell", title = "Grid Cell", Show)]
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

    /// The cell's [fill]($grid.fill) override.
    pub fill: Smart<Option<Paint>>,

    /// The cell's [alignment]($grid.align) override.
    pub align: Smart<Alignment>,

    /// The cell's [inset]($grid.inset) override.
    pub inset: Smart<Sides<Option<Rel<Length>>>>,

    /// The cell's [stroke]($grid.stroke) override.
    #[resolve]
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
        Packed::new(GridCell::new(Content::default()))
    }
}

impl ResolvableCell for Packed<GridCell> {
    fn resolve_cell<'a>(
        mut self,
        x: usize,
        y: usize,
        fill: &Option<Paint>,
        align: Smart<Alignment>,
        inset: Sides<Option<Rel<Length>>>,
        stroke: Sides<Option<Option<Arc<Stroke<Abs>>>>>,
        breakable: bool,
        locator: Locator<'a>,
        styles: StyleChain,
    ) -> Cell<'a> {
        let cell = &mut *self;
        let colspan = cell.colspan(styles);
        let rowspan = cell.rowspan(styles);
        let breakable = cell.breakable(styles).unwrap_or(breakable);
        let fill = cell.fill(styles).unwrap_or_else(|| fill.clone());

        let cell_stroke = cell.stroke(styles);
        let stroke_overridden =
            cell_stroke.as_ref().map(|side| matches!(side, Some(Some(_))));

        // Using a typical 'Sides' fold, an unspecified side loses to a
        // specified side. Additionally, when both are specified, an inner
        // None wins over the outer Some, and vice-versa. When both are
        // specified and Some, fold occurs, which, remarkably, leads to an Arc
        // clone.
        //
        // In the end, we flatten because, for layout purposes, an unspecified
        // cell stroke is the same as specifying 'none', so we equate the two
        // concepts.
        let stroke = cell_stroke.fold(stroke).map(Option::flatten);
        cell.push_x(Smart::Custom(x));
        cell.push_y(Smart::Custom(y));
        cell.push_fill(Smart::Custom(fill.clone()));
        cell.push_align(match align {
            Smart::Custom(align) => {
                Smart::Custom(cell.align(styles).map_or(align, |inner| inner.fold(align)))
            }
            // Don't fold if the grid is using outer alignment. Use the
            // cell's alignment instead (which, in the end, will fold with
            // the outer alignment when it is effectively displayed).
            Smart::Auto => cell.align(styles),
        });
        cell.push_inset(Smart::Custom(
            cell.inset(styles).map_or(inset, |inner| inner.fold(inset)),
        ));
        cell.push_stroke(
            // Here we convert the resolved stroke to a regular stroke, however
            // with resolved units (that is, 'em' converted to absolute units).
            // We also convert any stroke unspecified by both the cell and the
            // outer stroke ('None' in the folded stroke) to 'none', that is,
            // all sides are present in the resulting Sides object accessible
            // by show rules on grid cells.
            stroke.as_ref().map(|side| {
                Some(side.as_ref().map(|cell_stroke| {
                    Arc::new((**cell_stroke).clone().map(Length::from))
                }))
            }),
        );
        cell.push_breakable(Smart::Custom(breakable));
        Cell {
            body: self.pack(),
            locator,
            fill,
            colspan,
            rowspan,
            stroke,
            stroke_overridden,
            breakable,
        }
    }

    fn x(&self, styles: StyleChain) -> Smart<usize> {
        (**self).x(styles)
    }

    fn y(&self, styles: StyleChain) -> Smart<usize> {
        (**self).y(styles)
    }

    fn colspan(&self, styles: StyleChain) -> NonZeroUsize {
        (**self).colspan(styles)
    }

    fn rowspan(&self, styles: StyleChain) -> NonZeroUsize {
        (**self).rowspan(styles)
    }

    fn span(&self) -> Span {
        Packed::span(self)
    }
}

impl Show for Packed<GridCell> {
    fn show(&self, _engine: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        show_grid_cell(self.body().clone(), self.inset(styles), self.align(styles))
    }
}

impl From<Content> for GridCell {
    fn from(value: Content) -> Self {
        #[allow(clippy::unwrap_or_default)]
        value.unpack::<Self>().unwrap_or_else(Self::new)
    }
}

/// Function with common code to display a grid cell or table cell.
pub(crate) fn show_grid_cell(
    mut body: Content,
    inset: Smart<Sides<Option<Rel<Length>>>>,
    align: Smart<Alignment>,
) -> SourceResult<Content> {
    let inset = inset.unwrap_or_default().map(Option::unwrap_or_default);

    if inset != Sides::default() {
        // Only pad if some inset is not 0pt.
        // Avoids a bug where using .padded() in any way inside Show causes
        // alignment in align(...) to break.
        body = body.padded(inset);
    }

    if let Smart::Custom(alignment) = align {
        body = body.aligned(alignment);
    }

    Ok(body)
}
