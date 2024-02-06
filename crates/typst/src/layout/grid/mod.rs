mod layout;

pub use self::layout::{Cell, CellGrid, Celled, GridLayouter, ResolvableCell};

use std::num::NonZeroUsize;

use ecow::eco_format;
use smallvec::{smallvec, SmallVec};

use crate::diag::{SourceResult, StrResult, Trace, Tracepoint};
use crate::engine::Engine;
use crate::foundations::{
    cast, elem, scope, Array, Content, Fold, Packed, Show, Smart, StyleChain, Value,
};
use crate::layout::{
    AlignElem, Alignment, Axes, Fragment, LayoutMultiple, Length, Regions, Rel, Sides,
    Sizing,
};
use crate::syntax::Span;
use crate::util::NonZeroExt;
use crate::visualize::{Paint, Stroke};

/// Arranges content in a grid.
///
/// The grid element allows you to arrange content in a grid. You can define the
/// number of rows and columns, as well as the size of the gutters between them.
/// There are multiple sizing modes for columns and rows that can be used to
/// create complex layouts.
///
/// The sizing of the grid is determined by the track sizes specified in the
/// arguments. Because each of the sizing parameters accepts the same values, we
/// will explain them just once, here. Each sizing argument accepts an array of
/// individual track sizes. A track size is either:
///
/// - `{auto}`: The track will be sized to fit its contents. It will be at most
///   as large as the remaining space. If there is more than one `{auto}` track
///   which, and together they claim more than the available space, the `{auto}`
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
/// # Styling the grid
/// The grid's appearance can be customized through different parameters, such
/// as `fill` to give all cells a background; `align` to change how cells are
/// aligned; `inset` to optionally add internal padding to each cell; and
/// `stroke` to optionally enable grid lines with a certain stroke.
///
/// If you need to override one of the above options for a single cell, you can
/// use the [`grid.cell`]($grid.cell) element. Alternatively, if you need the
/// appearance options to depend on a cell's position (column and row), you may
/// specify a function to `fill` or `align` of the form
/// `(column, row) => value`. You may also use a show rule on
/// [`grid.cell`]($grid.cell) - see that element's examples or the examples
/// below for more information.
///
/// # Examples
/// The example below demonstrates the different track sizing options.
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
///   image("tiger.jpg", height: 100%),
///   image("tiger.jpg", height: 100%),
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
/// Additionally, you can use [`grid.cell`]($grid.cell) in various ways to
/// not only style each cell based on its position and other fields, but also
/// to determine the cell's preferential position in the table.
///
/// ```example
/// #set page(width: auto)
/// #show grid.cell: it => {
///   if it.y == 0 {
///     // The first row's text must be white and bold.
///     set text(white)
///     strong(it)
///   } else {
///     // For the second row and beyond, we will show the day number for each
///     // cell.
///
///     // In general, a cell's index is given by cell.x + columns * cell.y.
///     // Days start in the second grid row, so we subtract 1 row.
///     // But the first day is day 1, not day 0, so we add 1.
///     let day = it.x + 7 * (it.y - 1) + 1
///     if day <= 31 {
///       // Place the day's number at the top left of the cell.
///       // Only if the day is valid for this month (not 32 or higher).
///       place(top + left, dx: 2pt, dy: 2pt, text(8pt, red.darken(40%))[#day])
///     }
///     it
///   }
/// }
///
/// #grid(
///   fill: (x, y) => if y == 0 { gray.darken(50%) },
///   columns: (30pt,) * 7,
///   rows: (auto, 30pt),
///   // Events will be written at the bottom of each day square.
///   align: bottom,
///   inset: 5pt,
///   stroke: (thickness: 0.5pt, dash: "densely-dotted"),
///
///   [Sun], [Mon], [Tue], [Wed], [Thu], [Fri], [Sat],
///
///   // This event will occur on the first Friday (sixth column).
///   grid.cell(x: 5, fill: yellow.darken(10%))[Call],
///
///   // This event will occur every Monday (second column).
///   // We have to repeat it 5 times so it occurs every week.
///   ..(grid.cell(x: 1, fill: red.lighten(50%))[Meet],) * 5,
///
///   // This event will occur at day 19.
///   grid.cell(x: 4, y: 3, fill: orange.lighten(25%))[Talk],
///
///   // These events will occur at the second week, where available.
///   grid.cell(y: 2, fill: aqua)[Chat],
///   grid.cell(y: 2, fill: aqua)[Walk],
/// )
/// ```
#[elem(scope, LayoutMultiple)]
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

    /// The gaps between rows & columns.
    ///
    /// If there are more gutters than defined sizes, the last gutter is repeated.
    #[external]
    pub gutter: TrackSizings,

    /// The gaps between columns. Takes precedence over `gutter`.
    #[parse(
        let gutter = args.named("gutter")?;
        args.named("column-gutter")?.or_else(|| gutter.clone())
    )]
    #[borrowed]
    pub column_gutter: TrackSizings,

    /// The gaps between rows. Takes precedence over `gutter`.
    #[parse(args.named("row-gutter")?.or_else(|| gutter.clone()))]
    #[borrowed]
    pub row_gutter: TrackSizings,

    /// How to fill the cells.
    ///
    /// This can be a color or a function that returns a color. The function is
    /// passed the cells' column and row index, starting at zero. This can be
    /// used to implement striped grids.
    ///
    /// ```example
    /// #grid(
    ///   fill: (col, row) => if calc.even(col + row) { luma(240) } else { white },
    ///   align: center + horizon,
    ///   columns: 4,
    ///   [X], [O], [X], [O],
    ///   [O], [X], [O], [X],
    ///   [X], [O], [X], [O],
    ///   [O], [X], [O], [X]
    /// )
    /// ```
    #[borrowed]
    pub fill: Celled<Option<Paint>>,

    /// How to align the cells' content.
    ///
    /// This can either be a single alignment, an array of alignments
    /// (corresponding to each column) or a function that returns an alignment.
    /// The function is passed the cells' column and row index, starting at zero.
    /// If set to `{auto}`, the outer alignment is used.
    ///
    /// ```example
    /// #grid(
    ///   columns: 3,
    ///   align: (x, y) => (left, center, right).at(x),
    ///   [Hello], [Hello], [Hello],
    ///   [A], [B], [C],
    /// )
    /// ```
    #[borrowed]
    pub align: Celled<Smart<Alignment>>,

    /// How to [stroke]($stroke) the cells.
    ///
    /// Grids have no strokes by default, which can be changed by setting this
    /// option to the desired stroke.
    ///
    /// _Note:_ Richer stroke customization for individual cells is not yet
    /// implemented, but will be in the future. In the meantime, you can use the
    /// third-party [tablex library](https://github.com/PgBiel/typst-tablex/).
    #[resolve]
    #[fold]
    pub stroke: Option<Stroke>,

    /// How much to pad the cells' content.
    ///
    /// ```example
    /// #grid(
    ///   inset: 10pt,
    ///   fill: (_, row) => (red, blue).at(row),
    ///   [Hello],
    ///   [World],
    /// )
    ///
    /// #grid(
    ///   columns: 2,
    ///   inset: (
    ///     x: 20pt,
    ///     y: 10pt,
    ///   ),
    ///   fill: (col, _) => (red, blue).at(col),
    ///   [Hello],
    ///   [World],
    /// )
    /// ```
    #[fold]
    pub inset: Sides<Option<Rel<Length>>>,

    /// The contents of the grid cells.
    ///
    /// The cells are populated in row-major order.
    #[variadic]
    pub children: Vec<Packed<GridCell>>,
}

#[scope]
impl GridElem {
    #[elem]
    type GridCell;
}

impl LayoutMultiple for Packed<GridElem> {
    #[typst_macros::time(name = "grid", span = self.span())]
    fn layout(
        &self,
        engine: &mut Engine,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let inset = self.inset(styles);
        let align = self.align(styles);
        let columns = self.columns(styles);
        let rows = self.rows(styles);
        let column_gutter = self.column_gutter(styles);
        let row_gutter = self.row_gutter(styles);
        let fill = self.fill(styles);
        let stroke = self.stroke(styles).map(Stroke::unwrap_or_default);

        let tracks = Axes::new(columns.0.as_slice(), rows.0.as_slice());
        let gutter = Axes::new(column_gutter.0.as_slice(), row_gutter.0.as_slice());
        // Use trace to link back to the grid when a specific cell errors
        let tracepoint = || Tracepoint::Call(Some(eco_format!("grid")));
        let grid = CellGrid::resolve(
            tracks,
            gutter,
            self.children(),
            fill,
            align,
            inset,
            engine,
            styles,
            self.span(),
        )
        .trace(engine.world, tracepoint, self.span())?;

        let layouter = GridLayouter::new(&grid, &stroke, regions, styles, self.span());

        // Measure the columns and layout the grid row-by-row.
        layouter.layout(engine)
    }
}

/// Track sizing definitions.
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct TrackSizings(pub SmallVec<[Sizing; 4]>);

cast! {
    TrackSizings,
    self => self.0.into_value(),
    sizing: Sizing => Self(smallvec![sizing]),
    count: NonZeroUsize => Self(smallvec![Sizing::Auto; count.get()]),
    values: Array => Self(values.into_iter().map(Value::cast).collect::<StrResult<_>>()?),
}

/// A cell in the grid. Use this to either override grid properties for a
/// particular cell, or in show rules to apply certain styles to multiple cells
/// at once.
///
/// For example, you can override the fill, alignment or inset for a single
/// cell:
///
/// ```example
/// #grid(
///   columns: 2,
///   fill: red,
///   align: left,
///   inset: 5pt,
///   [ABC], [ABC],
///   grid.cell(fill: blue)[C], [D],
///   grid.cell(align: center)[E], [F],
///   [G], grid.cell(inset: 0pt)[H]
/// )
/// ```
///
/// You may also apply a show rule on `grid.cell` to style all cells at once,
/// which allows you, for example, to apply styles based on a cell's position:
///
/// ```example
/// #show grid.cell: it => {
///   if it.y == 0 {
///     // First row is bold
///     strong(it)
///   } else if it.x == 1 {
///     // Second column is italicized
///     // (except at the first row)
///     emph(it)
///   } else {
///     // Remaining cells aren't changed
///     it
///   }
/// }
///
/// #grid(
///   columns: 3,
///   gutter: 3pt,
///   [Name], [Age], [Info],
///   [John], [52], [Nice],
///   [Mary], [50], [Cool],
///   [Jake], [49], [Epic]
/// )
/// ```
#[elem(name = "cell", title = "Grid Cell", Show)]
pub struct GridCell {
    /// The cell's body.
    #[required]
    body: Content,

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
    /// #grid(
    ///   columns: 4,
    ///   rows: 2.5em,
    ///   fill: (x, y) => if calc.odd(x + y) { blue.lighten(50%) } else { blue.lighten(10%) },
    ///   align: center + horizon,
    ///   inset: 3pt,
    ///   grid.cell(x: 2, y: 2)[3],
    ///   [1], grid.cell(x: 3)[4], [2],
    /// )
    /// ```
    x: Smart<usize>,

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
    /// #grid(
    ///   columns: 2,
    ///   fill: (x, y) => if calc.odd(x + y) { gray.lighten(40%) },
    ///   inset: 1pt,
    ///   [A], grid.cell(y: 1)[B], grid.cell(y: 1)[C], grid.cell(y: 2)[D]
    /// )
    /// ```
    y: Smart<usize>,

    /// The amount of columns spanned by this cell.
    #[default(NonZeroUsize::ONE)]
    colspan: NonZeroUsize,

    /// The cell's fill override.
    fill: Smart<Option<Paint>>,

    /// The cell's alignment override.
    align: Smart<Alignment>,

    /// The cell's inset override.
    inset: Smart<Sides<Option<Rel<Length>>>>,
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
    fn resolve_cell(
        mut self,
        x: usize,
        y: usize,
        fill: &Option<Paint>,
        align: Smart<Alignment>,
        inset: Sides<Option<Rel<Length>>>,
        styles: StyleChain,
    ) -> Cell {
        let cell = &mut *self;
        let colspan = cell.colspan(styles);
        let fill = cell.fill(styles).unwrap_or_else(|| fill.clone());
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
        Cell { body: self.pack(), fill, colspan }
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
pub fn show_grid_cell(
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
        body = body.styled(AlignElem::set_alignment(alignment));
    }

    Ok(body)
}
