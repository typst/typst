use std::num::NonZeroUsize;
use std::sync::Arc;

use ecow::eco_format;

use crate::diag::{bail, SourceResult, Trace, Tracepoint};
use crate::engine::Engine;
use crate::foundations::{
    cast, elem, scope, Content, Fold, Packed, Show, Smart, StyleChain,
};
use crate::layout::{
    show_grid_cell, Abs, Alignment, Axes, Cell, CellGrid, Celled, Dir, Fragment,
    GridCell, GridHLine, GridItem, GridLayouter, GridVLine, LayoutMultiple, Length,
    LinePosition, OuterHAlignment, OuterVAlignment, Regions, Rel, ResolvableCell, Sides,
    TrackSizings,
};
use crate::model::Figurable;
use crate::syntax::Span;
use crate::text::{Lang, LocalName, Region, TextElem};
use crate::util::NonZeroExt;
use crate::visualize::{Paint, Stroke};

/// A table of items.
///
/// Tables are used to arrange content in cells. Cells can contain arbitrary
/// content, including multiple paragraphs and are specified in row-major order.
/// Because tables are just grids with different defaults for some cell
/// properties (notably `stroke` and `inset`), refer to the
/// [grid documentation]($grid) for more information on how to size the table
/// tracks and specify the cell appearance properties.
///
/// Note that, to override a particular cell's properties or apply show rules
/// on table cells, you can use the [`table.cell`]($table.cell) element (but
/// not `grid.cell`, which is exclusive to grids). See its documentation for
/// more information.
///
/// To give a table a caption and make it [referenceable]($ref), put it into a
/// [figure]($figure).
///
/// # Example
///
/// The example below demonstrates some of the most common table options.
/// ```example
/// #table(
///   columns: (1fr, auto, auto),
///   inset: 10pt,
///   align: horizon,
///   [], [*Area*], [*Parameters*],
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
/// Much like with grids, you can use [`table.cell`]($table.cell) to customize
/// the appearance and the position of each cell.
///
/// ```example
/// #set page(width: auto)
/// #show table.cell: it => {
///   if it.x == 0 or it.y == 0 {
///     set text(white)
///     strong(it)
///   } else if it.body == [] {
///     // Replace empty cells with 'N/A'
///     pad(rest: it.inset)[_N/A_]
///   } else {
///     it
///   }
/// }
///
/// #table(
///   fill: (x, y) => if x == 0 or y == 0 { gray.darken(50%) },
///   columns: 4,
///   [], [Exam 1], [Exam 2], [Exam 3],
///   ..([John], [Mary], [Jake], [Robert]).map(table.cell.with(x: 0)),
///
///   // Mary got grade A on Exam 3.
///   table.cell(x: 3, y: 2, fill: green)[A],
///
///   // Everyone got grade A on Exam 2.
///   ..(table.cell(x: 2, fill: green)[A],) * 4,
///
///   // Robert got grade B on other exams.
///   ..(table.cell(y: 4, fill: aqua)[B],) * 2,
/// )
/// ```
#[elem(scope, LayoutMultiple, LocalName, Figurable)]
pub struct TableElem {
    /// The column sizes. See the [grid documentation]($grid) for more
    /// information on track sizing.
    #[borrowed]
    pub columns: TrackSizings,

    /// The row sizes. See the [grid documentation]($grid) for more information
    /// on track sizing.
    #[borrowed]
    pub rows: TrackSizings,

    /// The gaps between rows & columns. See the [grid documentation]($grid) for
    /// more information on gutters.
    #[external]
    pub gutter: TrackSizings,

    /// The gaps between columns. Takes precedence over `gutter`. See the
    /// [grid documentation]($grid) for more information on gutters.
    #[borrowed]
    #[parse(
        let gutter = args.named("gutter")?;
        args.named("column-gutter")?.or_else(|| gutter.clone())
    )]
    pub column_gutter: TrackSizings,

    /// The gaps between rows. Takes precedence over `gutter`. See the
    /// [grid documentation]($grid) for more information on gutters.
    #[parse(args.named("row-gutter")?.or_else(|| gutter.clone()))]
    #[borrowed]
    pub row_gutter: TrackSizings,

    /// How to fill the cells.
    ///
    /// This can be a color or a function that returns a color. The function is
    /// passed the cells' column and row index, starting at zero. This can be
    /// used to implement striped tables.
    ///
    /// ```example
    /// #table(
    ///   fill: (col, _) => if calc.odd(col) { luma(240) } else { white },
    ///   align: (col, row) =>
    ///     if row == 0 { center }
    ///     else if col == 0 { left }
    ///     else { right },
    ///   columns: 4,
    ///   [], [*Q1*], [*Q2*], [*Q3*],
    ///   [Revenue:], [1000 €], [2000 €], [3000 €],
    ///   [Expenses:], [500 €], [1000 €], [1500 €],
    ///   [Profit:], [500 €], [1000 €], [1500 €],
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
    /// #table(
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
    /// Strokes can be disabled by setting this to `{none}`.
    ///
    /// If it is necessary to place lines which can cross spacing between cells
    /// produced by the `gutter` option, or to override the stroke between
    /// multiple specific cells, consider specifying one or more of
    /// [`table.hline`]($table.hline) and [`table.vline`]($table.vline) alongside
    /// your table cells.
    ///
    /// See the [grid documentation]($grid) for more information on stroke.
    #[resolve]
    #[fold]
    #[default(Celled::Value(Sides::splat(Some(Some(Arc::new(Stroke::default()))))))]
    pub stroke: Celled<Sides<Option<Option<Arc<Stroke>>>>>,

    /// How much to pad the cells' content.
    ///
    /// ```example
    /// #table(
    ///   inset: 10pt,
    ///   [Hello],
    ///   [World],
    /// )
    ///
    /// #table(
    ///   columns: 2,
    ///   inset: (
    ///     x: 20pt,
    ///     y: 10pt,
    ///   ),
    ///   [Hello],
    ///   [World],
    /// )
    /// ```
    #[fold]
    #[default(Celled::Value(Sides::splat(Some(Abs::pt(5.0).into()))))]
    pub inset: Celled<Sides<Option<Rel<Length>>>>,

    /// The contents of the table cells, plus any extra table lines specified
    /// with the [`table.hline`]($table.hline) and
    /// [`table.vline`]($table.vline) elements.
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
}

impl LayoutMultiple for Packed<TableElem> {
    #[typst_macros::time(name = "table", span = self.span())]
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
        let stroke = self.stroke(styles);

        let tracks = Axes::new(columns.0.as_slice(), rows.0.as_slice());
        let gutter = Axes::new(column_gutter.0.as_slice(), row_gutter.0.as_slice());
        // Use trace to link back to the table when a specific cell errors
        let tracepoint = || Tracepoint::Call(Some(eco_format!("table")));
        let items = self.children().iter().map(|child| match child {
            TableChild::HLine(hline) => GridItem::HLine {
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
            TableChild::VLine(vline) => GridItem::VLine {
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
            TableChild::Cell(cell) => GridItem::Cell(cell.clone()),
        });
        let grid = CellGrid::resolve(
            tracks,
            gutter,
            items,
            fill,
            align,
            &inset,
            &stroke,
            engine,
            styles,
            self.span(),
        )
        .trace(engine.world, tracepoint, self.span())?;

        let layouter = GridLayouter::new(&grid, regions, styles, self.span());
        layouter.layout(engine)
    }
}

impl LocalName for Packed<TableElem> {
    fn local_name(lang: Lang, _: Option<Region>) -> &'static str {
        match lang {
            Lang::ALBANIAN => "Tabel",
            Lang::ARABIC => "جدول",
            Lang::BOKMÅL => "Tabell",
            Lang::CATALAN => "Taula",
            Lang::CHINESE => "表",
            Lang::CZECH => "Tabulka",
            Lang::DANISH => "Tabel",
            Lang::DUTCH => "Tabel",
            Lang::ESTONIAN => "Tabel",
            Lang::FILIPINO => "Talaan",
            Lang::FINNISH => "Taulukko",
            Lang::FRENCH => "Tableau",
            Lang::GERMAN => "Tabelle",
            Lang::GREEK => "Πίνακας",
            Lang::HUNGARIAN => "Táblázat",
            Lang::ITALIAN => "Tabella",
            Lang::NYNORSK => "Tabell",
            Lang::POLISH => "Tabela",
            Lang::PORTUGUESE => "Tabela",
            Lang::ROMANIAN => "Tabelul",
            Lang::RUSSIAN => "Таблица",
            Lang::SERBIAN => "Табела",
            Lang::SLOVENIAN => "Tabela",
            Lang::SPANISH => "Tabla",
            Lang::SWEDISH => "Tabell",
            Lang::TURKISH => "Tablo",
            Lang::UKRAINIAN => "Таблиця",
            Lang::VIETNAMESE => "Bảng",
            Lang::JAPANESE => "表",
            Lang::ENGLISH | _ => "Table",
        }
    }
}

impl Figurable for Packed<TableElem> {}

/// Any child of a table element.
#[derive(Debug, PartialEq, Clone, Hash)]
pub enum TableChild {
    HLine(Packed<TableHLine>),
    VLine(Packed<TableVLine>),
    Cell(Packed<TableCell>),
}

cast! {
    TableChild,
    self => match self {
        Self::HLine(hline) => hline.into_value(),
        Self::VLine(vline) => vline.into_value(),
        Self::Cell(cell) => cell.into_value(),
    },
    v: Content => {
        if v.is::<GridCell>() {
            bail!(
                "cannot use `grid.cell` as a table cell; use `table.cell` instead"
            );
        }
        if v.is::<GridHLine>() {
            bail!(
                "cannot use `grid.hline` as a table line; use `table.hline` instead"
            );
        }
        if v.is::<GridVLine>() {
            bail!(
                "cannot use `grid.vline` as a table line; use `table.vline` instead"
            );
        }
        v.into()
    }
}

impl From<Content> for TableChild {
    fn from(value: Content) -> Self {
        value
            .into_packed::<TableHLine>()
            .map(TableChild::HLine)
            .or_else(|value| value.into_packed::<TableVLine>().map(TableChild::VLine))
            .or_else(|value| value.into_packed::<TableCell>().map(TableChild::Cell))
            .unwrap_or_else(|value| {
                let span = value.span();
                TableChild::Cell(Packed::new(TableCell::new(value)).spanned(span))
            })
    }
}

/// A horizontal line in the table. See the docs for
/// [`grid.hline`]($grid.hline) for more information regarding how to use this
/// element's fields.
///
/// Overrides any per-cell stroke, including stroke specified through the
/// table's `stroke` field. Can cross spacing between cells created through
/// the table's `column-gutter` option.
#[elem(name = "hline", title = "Table Horizontal Line")]
pub struct TableHLine {
    /// The row above which the horizontal line is placed (zero-indexed).
    /// Functions identically to the `y` field in [`grid.hline`]($grid.hline).
    pub y: Smart<usize>,

    /// The column at which the horizontal line starts (zero-indexed, inclusive).
    pub start: usize,

    /// The column before which the horizontal line ends (zero-indexed,
    /// exclusive).
    pub end: Option<NonZeroUsize>,

    /// The line's stroke.
    ///
    /// Specifying `{none}` interrupts previous hlines placed across this
    /// line's range, but does not affect per-cell stroke or vlines.
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

/// A vertical line in the table. See the docs for [`grid.vline`]($grid.vline)
/// for more information regarding how to use this element's fields.
///
/// Overrides any per-cell stroke, including stroke specified through the
/// table's `stroke` field. Can cross spacing between cells created through
/// the table's `row-gutter` option.
#[elem(name = "vline", title = "Table Vertical Line")]
pub struct TableVLine {
    /// The column before which the horizontal line is placed (zero-indexed).
    /// Functions identically to the `x` field in [`grid.vline`]($grid.vline).
    pub x: Smart<usize>,

    /// The row at which the vertical line starts (zero-indexed, inclusive).
    pub start: usize,

    /// The row on top of which the vertical line ends (zero-indexed,
    /// exclusive).
    pub end: Option<NonZeroUsize>,

    /// The line's stroke.
    ///
    /// Specifying `{none}` interrupts previous vlines placed across this
    /// line's range, but does not affect per-cell stroke or hlines.
    #[resolve]
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

/// A cell in the table. Use this to either override table properties for a
/// particular cell, or in show rules to apply certain styles to multiple cells
/// at once.
///
/// For example, you can override the fill, alignment or inset for a single
/// cell:
///
/// ```example
/// #table(
///   columns: 2,
///   fill: green,
///   align: right,
///   [*Name*], [*Data*],
///   table.cell(fill: blue)[J.], [Organizer],
///   table.cell(align: center)[K.], [Leader],
///   [M.], table.cell(inset: 0pt)[Player]
/// )
/// ```
///
/// You may also apply a show rule on `table.cell` to style all cells at once,
/// which allows you, for example, to apply styles based on a cell's position:
///
/// ```example
/// #show table.cell: it => {
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
/// #table(
///   columns: 3,
///   gutter: 3pt,
///   [Name], [Age], [Info],
///   [John], [52], [Nice],
///   [Mary], [50], [Cool],
///   [Jake], [49], [Epic]
/// )
/// ```
#[elem(name = "cell", title = "Table Cell", Show)]
pub struct TableCell {
    /// The cell's body.
    #[required]
    pub body: Content,

    /// The cell's column (zero-indexed).
    /// Functions identically to the `x` field in [`grid.cell`]($grid.cell).
    pub x: Smart<usize>,

    /// The cell's row (zero-indexed).
    /// Functions identically to the `y` field in [`grid.cell`]($grid.cell).
    pub y: Smart<usize>,

    /// The cell's fill override.
    pub fill: Smart<Option<Paint>>,

    /// The amount of columns spanned by this cell.
    #[default(NonZeroUsize::ONE)]
    pub colspan: NonZeroUsize,

    /// The cell's alignment override.
    pub align: Smart<Alignment>,

    /// The cell's inset override.
    pub inset: Smart<Sides<Option<Rel<Length>>>>,

    /// The cell's stroke override.
    #[resolve]
    #[fold]
    pub stroke: Sides<Option<Option<Arc<Stroke>>>>,
}

cast! {
    TableCell,
    v: Content => v.into(),
}

impl Default for Packed<TableCell> {
    fn default() -> Self {
        Packed::new(TableCell::new(Content::default()))
    }
}

impl ResolvableCell for Packed<TableCell> {
    fn resolve_cell(
        mut self,
        x: usize,
        y: usize,
        fill: &Option<Paint>,
        align: Smart<Alignment>,
        inset: Sides<Option<Rel<Length>>>,
        stroke: Sides<Option<Option<Arc<Stroke<Abs>>>>>,
        styles: StyleChain,
    ) -> Cell {
        let cell = &mut *self;
        let colspan = cell.colspan(styles);
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
            // Don't fold if the table is using outer alignment. Use the
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
            // by show rules on table cells.
            stroke.as_ref().map(|side| {
                Some(side.as_ref().map(|cell_stroke| {
                    Arc::new((**cell_stroke).clone().map(Length::from))
                }))
            }),
        );
        Cell {
            body: self.pack(),
            fill,
            colspan,
            stroke,
            stroke_overridden,
        }
    }

    fn x(&self, styles: StyleChain) -> Smart<usize> {
        (**self).x(styles)
    }

    fn y(&self, styles: StyleChain) -> Smart<usize> {
        (**self).y(styles)
    }

    fn colspan(&self, styles: StyleChain) -> std::num::NonZeroUsize {
        (**self).colspan(styles)
    }

    fn span(&self) -> Span {
        Packed::span(self)
    }
}

impl Show for Packed<TableCell> {
    fn show(&self, _engine: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        show_grid_cell(self.body().clone(), self.inset(styles), self.align(styles))
    }
}

impl From<Content> for TableCell {
    fn from(value: Content) -> Self {
        #[allow(clippy::unwrap_or_default)]
        value.unpack::<Self>().unwrap_or_else(Self::new)
    }
}
