use std::num::NonZeroUsize;

use smallvec::{smallvec, SmallVec};

use crate::diag::{SourceResult, StrResult};
use crate::engine::Engine;
use crate::foundations::{
    cast, elem, scope, Array, Content, NativeElement, Show, Smart, StyleChain, Value,
};
use crate::layout::{
    Abs, Align, AlignElem, Axes, Cell, CellGrid, Celled, Fragment, GridLayouter, Layout,
    Length, Regions, Rel, ResolvableCell, Sides, Sizing,
};
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
#[elem(scope, Layout)]
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
    pub align: Celled<Smart<Align>>,

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
    #[default(Sides::splat(Abs::pt(0.0).into()))]
    pub inset: Sides<Option<Rel<Length>>>,

    /// The contents of the grid cells.
    ///
    /// The cells are populated in row-major order.
    #[variadic]
    pub children: Vec<GridCell>,
}

#[scope]
impl GridElem {
    #[elem]
    type GridCell;
}

impl Layout for GridElem {
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
        let grid = CellGrid::new(tracks, gutter, self.children().clone(), styles)
            .resolve_cells(engine, fill, align, inset, styles)?;

        // Prepare grid layout by unifying content and gutter tracks.
        let layouter = GridLayouter::new(&grid, &stroke, regions, styles, self.span());

        // Measure the columns and layout the grid row-by-row.
        Ok(layouter.layout(engine)?.fragment)
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

/// A cell in the grid.
#[elem(name = "cell", title = "Grid Cell", Show)]
pub struct GridCell {
    /// The cell's body.
    #[required]
    body: Content,

    /// The cell's fill override.
    fill: Smart<Option<Paint>>,

    /// The cell's alignment override.
    align: Smart<Align>,

    /// The cell's inset override.
    inset: Smart<Sides<Option<Rel<Length>>>>,
}

cast! {
    GridCell,
    v: Content => v.into(),
}

impl Cell for GridCell {
    fn fill(&self, styles: StyleChain) -> Option<Paint> {
        // The fill must have been resolved by the point it's requested.
        self.fill(styles).unwrap_or(None)
    }
}

impl ResolvableCell for GridCell {
    fn resolve_cell(
        &mut self,
        _x: usize,
        _y: usize,
        fill: &Option<Paint>,
        align: Smart<Align>,
        inset: Sides<Rel<Length>>,
        styles: StyleChain,
    ) {
        self.push_fill(Smart::Custom(self.fill(styles).unwrap_or_else(|| fill.clone())));
        self.push_align(self.align(styles).or(align));
        self.push_inset(Smart::Custom(
            self.inset(styles).unwrap_or_else(|| inset.map(Some)),
        ));
    }
}

impl Show for GridCell {
    fn show(&self, _engine: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        let inset = self.inset(styles).unwrap_or_default().map(Option::unwrap_or_default);

        let mut body = self.body().clone();

        if inset != Sides::default() {
            // Only pad if some inset is not 0pt.
            // Avoids a bug where using .padded() in any way inside Show causes
            // alignment in align(...) to break.
            body = body.padded(inset);
        }

        if let Smart::Custom(alignment) = self.align(styles) {
            body = body.styled(AlignElem::set_alignment(alignment));
        }

        Ok(body)
    }
}

impl Layout for GridCell {
    fn layout(
        &self,
        engine: &mut Engine,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        self.clone().pack().layout(engine, styles, regions)
    }
}

impl From<Content> for GridCell {
    fn from(value: Content) -> Self {
        value
            .to::<Self>()
            .cloned()
            .unwrap_or_else(|| Self::new(value.clone()))
    }
}
