use std::num::{NonZeroU32, NonZeroUsize};
use std::ops::Range;
use std::sync::Arc;

use ecow::eco_format;
use typst_library::diag::{
    bail, At, Hint, HintedStrResult, HintedString, SourceResult, Trace, Tracepoint,
};
use typst_library::engine::Engine;
use typst_library::foundations::{Content, Fold, Packed, Smart, StyleChain};
use typst_library::introspection::Locator;
use typst_library::layout::{
    Abs, Alignment, Axes, Celled, GridCell, GridChild, GridElem, GridItem, Length,
    OuterHAlignment, OuterVAlignment, Rel, ResolvedCelled, Sides, Sizing,
};
use typst_library::model::{TableCell, TableChild, TableElem, TableItem};
use typst_library::text::TextElem;
use typst_library::visualize::{Paint, Stroke};
use typst_library::Dir;

use typst_syntax::Span;
use typst_utils::NonZeroExt;

use crate::introspection::SplitLocator;

/// Convert a grid to a cell grid.
#[typst_macros::time(span = elem.span())]
pub fn grid_to_cellgrid<'a>(
    elem: &Packed<GridElem>,
    engine: &mut Engine,
    locator: Locator<'a>,
    styles: StyleChain,
) -> SourceResult<CellGrid<'a>> {
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
    let resolve_item = |item: &GridItem| grid_item_to_resolvable(item, styles);
    let children = elem.children.iter().map(|child| match child {
        GridChild::Header(header) => ResolvableGridChild::Header {
            repeat: header.repeat(styles),
            level: header.level(styles),
            span: header.span(),
            items: header.children.iter().map(resolve_item),
        },
        GridChild::Footer(footer) => ResolvableGridChild::Footer {
            repeat: footer.repeat(styles),
            span: footer.span(),
            items: footer.children.iter().map(resolve_item),
        },
        GridChild::Item(item) => {
            ResolvableGridChild::Item(grid_item_to_resolvable(item, styles))
        }
    });
    resolve_cellgrid(
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
    .trace(engine.world, tracepoint, elem.span())
}

/// Convert a table to a cell grid.
#[typst_macros::time(span = elem.span())]
pub fn table_to_cellgrid<'a>(
    elem: &Packed<TableElem>,
    engine: &mut Engine,
    locator: Locator<'a>,
    styles: StyleChain,
) -> SourceResult<CellGrid<'a>> {
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
    // Use trace to link back to the table when a specific cell errors
    let tracepoint = || Tracepoint::Call(Some(eco_format!("table")));
    let resolve_item = |item: &TableItem| table_item_to_resolvable(item, styles);
    let children = elem.children.iter().map(|child| match child {
        TableChild::Header(header) => ResolvableGridChild::Header {
            repeat: header.repeat(styles),
            level: header.level(styles),
            span: header.span(),
            items: header.children.iter().map(resolve_item),
        },
        TableChild::Footer(footer) => ResolvableGridChild::Footer {
            repeat: footer.repeat(styles),
            span: footer.span(),
            items: footer.children.iter().map(resolve_item),
        },
        TableChild::Item(item) => {
            ResolvableGridChild::Item(table_item_to_resolvable(item, styles))
        }
    });
    resolve_cellgrid(
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
    .trace(engine.world, tracepoint, elem.span())
}

fn grid_item_to_resolvable(
    item: &GridItem,
    styles: StyleChain,
) -> ResolvableGridItem<Packed<GridCell>> {
    match item {
        GridItem::HLine(hline) => ResolvableGridItem::HLine {
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
        GridItem::VLine(vline) => ResolvableGridItem::VLine {
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
                OuterHAlignment::Start | OuterHAlignment::Left => LinePosition::Before,
                OuterHAlignment::End | OuterHAlignment::Right => LinePosition::After,
            },
        },
        GridItem::Cell(cell) => ResolvableGridItem::Cell(cell.clone()),
    }
}

fn table_item_to_resolvable(
    item: &TableItem,
    styles: StyleChain,
) -> ResolvableGridItem<Packed<TableCell>> {
    match item {
        TableItem::HLine(hline) => ResolvableGridItem::HLine {
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
        TableItem::VLine(vline) => ResolvableGridItem::VLine {
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
                OuterHAlignment::Start | OuterHAlignment::Left => LinePosition::Before,
                OuterHAlignment::End | OuterHAlignment::Right => LinePosition::After,
            },
        },
        TableItem::Cell(cell) => ResolvableGridItem::Cell(cell.clone()),
    }
}

impl ResolvableCell for Packed<TableCell> {
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

/// Represents an explicit grid line (horizontal or vertical) specified by the
/// user.
pub struct Line {
    /// The index of the track after this line. This will be the index of the
    /// row a horizontal line is above of, or of the column right after a
    /// vertical line.
    ///
    /// Must be within `0..=tracks.len()` (where `tracks` is either `grid.cols`
    /// or `grid.rows`, ignoring gutter tracks, as appropriate).
    pub index: usize,
    /// The index of the track at which this line starts being drawn.
    /// This is the first column a horizontal line appears in, or the first row
    /// a vertical line appears in.
    ///
    /// Must be within `0..tracks.len()` minus gutter tracks.
    pub start: usize,
    /// The index after the last track through which the line is drawn.
    /// Thus, the line is drawn through tracks `start..end` (note that `end` is
    /// exclusive).
    ///
    /// Must be within `1..=tracks.len()` minus gutter tracks.
    /// `None` indicates the line should go all the way to the end.
    pub end: Option<NonZeroUsize>,
    /// The line's stroke. This is `None` when the line is explicitly used to
    /// override a previously specified line.
    pub stroke: Option<Arc<Stroke<Abs>>>,
    /// The line's position in relation to the track with its index.
    pub position: LinePosition,
}

/// A repeatable grid header. Starts at the first row.
#[derive(Debug)]
pub struct Header {
    /// The first row included in this header.
    pub start: usize,
    /// The index after the last row included in this header.
    pub end: usize,
    /// The header's level.
    ///
    /// Higher level headers repeat together with lower level headers. If a
    /// lower level header stops repeating, all higher level headers do as
    /// well.
    pub level: u32,
}

impl Header {
    /// The header's range of included rows.
    #[inline]
    pub fn range(&self) -> Range<usize> {
        self.start..self.end
    }
}

/// A repeatable grid footer. Stops at the last row.
#[derive(Debug)]
pub struct Footer {
    /// The first row included in this footer.
    pub start: usize,
    /// The index after the last row included in this footer.
    pub end: usize,
    /// The footer's level.
    ///
    /// Used similarly to header level.
    pub level: u32,
}

impl Footer {
    /// The footer's range of included rows.
    #[inline]
    pub fn range(&self) -> Range<usize> {
        self.start..self.end
    }
}

/// A possibly repeatable grid object.
/// It still exists even when not repeatable, but must not have additional
/// considerations by grid layout, other than for consistency (such as making
/// a certain group of rows unbreakable).
pub enum Repeatable<T> {
    Repeated(T),
    NotRepeated(T),
}

impl<T> Repeatable<T> {
    /// Gets the value inside this repeatable, regardless of whether
    /// it repeats.
    #[inline]
    pub fn unwrap(&self) -> &T {
        match self {
            Self::Repeated(repeated) => repeated,
            Self::NotRepeated(not_repeated) => not_repeated,
        }
    }

    /// Gets the value inside this repeatable, regardless of whether
    /// it repeats.
    #[inline]
    pub fn unwrap_mut(&mut self) -> &mut T {
        match self {
            Self::Repeated(repeated) => repeated,
            Self::NotRepeated(not_repeated) => not_repeated,
        }
    }

    /// Returns `Some` if the value is repeated, `None` otherwise.
    #[inline]
    pub fn as_repeated(&self) -> Option<&T> {
        match self {
            Self::Repeated(repeated) => Some(repeated),
            Self::NotRepeated(_) => None,
        }
    }
}

/// Used for cell-like elements which are aware of their final properties in
/// the table, and may have property overrides.
pub trait ResolvableCell {
    /// Resolves the cell's fields, given its coordinates and default grid-wide
    /// fill, align, inset and stroke properties, plus the expected value of
    /// the `breakable` field.
    /// Returns a final Cell.
    #[allow(clippy::too_many_arguments)]
    fn resolve_cell<'a>(
        self,
        x: usize,
        y: usize,
        fill: &Option<Paint>,
        align: Smart<Alignment>,
        inset: Sides<Option<Rel<Length>>>,
        stroke: Sides<Option<Option<Arc<Stroke<Abs>>>>>,
        breakable: bool,
        locator: Locator<'a>,
        styles: StyleChain,
    ) -> Cell<'a>;

    /// Returns this cell's column override.
    fn x(&self, styles: StyleChain) -> Smart<usize>;

    /// Returns this cell's row override.
    fn y(&self, styles: StyleChain) -> Smart<usize>;

    /// The amount of columns spanned by this cell.
    fn colspan(&self, styles: StyleChain) -> NonZeroUsize;

    /// The amount of rows spanned by this cell.
    fn rowspan(&self, styles: StyleChain) -> NonZeroUsize;

    /// The cell's span, for errors.
    fn span(&self) -> Span;
}

/// A grid item, possibly affected by automatic cell positioning. Can be either
/// a line or a cell.
pub enum ResolvableGridItem<T: ResolvableCell> {
    /// A horizontal line in the grid.
    HLine {
        /// The row above which the horizontal line is drawn.
        y: Smart<usize>,
        start: usize,
        end: Option<NonZeroUsize>,
        stroke: Option<Arc<Stroke<Abs>>>,
        /// The span of the corresponding line element.
        span: Span,
        /// The line's position. "before" here means on top of row `y`, while
        /// "after" means below it.
        position: LinePosition,
    },
    /// A vertical line in the grid.
    VLine {
        /// The column before which the vertical line is drawn.
        x: Smart<usize>,
        start: usize,
        end: Option<NonZeroUsize>,
        stroke: Option<Arc<Stroke<Abs>>>,
        /// The span of the corresponding line element.
        span: Span,
        /// The line's position. "before" here means to the left of column `x`,
        /// while "after" means to its right (both considering LTR).
        position: LinePosition,
    },
    /// A cell in the grid.
    Cell(T),
}

/// Represents a cell in CellGrid, to be laid out by GridLayouter.
pub struct Cell<'a> {
    /// The cell's body.
    pub body: Content,
    /// The cell's locator.
    pub locator: Locator<'a>,
    /// The cell's fill.
    pub fill: Option<Paint>,
    /// The amount of columns spanned by the cell.
    pub colspan: NonZeroUsize,
    /// The amount of rows spanned by the cell.
    pub rowspan: NonZeroUsize,
    /// The cell's stroke.
    ///
    /// We use an Arc to avoid unnecessary space usage when all sides are the
    /// same, or when the strokes come from a common source.
    pub stroke: Sides<Option<Arc<Stroke<Abs>>>>,
    /// Which stroke sides were explicitly overridden by the cell, over the
    /// grid's global stroke setting.
    ///
    /// This is used to define whether or not this cell's stroke sides should
    /// have priority over adjacent cells' stroke sides, if those don't
    /// override their own stroke properties (and thus have less priority when
    /// defining with which stroke to draw grid lines around this cell).
    pub stroke_overridden: Sides<bool>,
    /// Whether rows spanned by this cell can be placed in different pages.
    /// By default, a cell spanning only fixed-size rows is unbreakable, while
    /// a cell spanning at least one `auto`-sized row is breakable.
    pub breakable: bool,
}

impl<'a> Cell<'a> {
    /// Create a simple cell given its body and its locator.
    pub fn new(body: Content, locator: Locator<'a>) -> Self {
        Self {
            body,
            locator,
            fill: None,
            colspan: NonZeroUsize::ONE,
            rowspan: NonZeroUsize::ONE,
            stroke: Sides::splat(None),
            stroke_overridden: Sides::splat(false),
            breakable: true,
        }
    }
}

/// Indicates whether the line should be drawn before or after the track with
/// its index. This is mostly only relevant when gutter is used, since, then,
/// the position after a track is not the same as before the next
/// non-gutter track.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum LinePosition {
    /// The line should be drawn before its track (e.g. hline on top of a row).
    Before,
    /// The line should be drawn after its track (e.g. hline below a row).
    After,
}

/// A grid entry.
pub enum Entry<'a> {
    /// An entry which holds a cell.
    Cell(Cell<'a>),
    /// An entry which is merged with another cell.
    Merged {
        /// The index of the cell this entry is merged with.
        parent: usize,
    },
}

impl<'a> Entry<'a> {
    /// Obtains the cell inside this entry, if this is not a merged cell.
    pub fn as_cell(&self) -> Option<&Cell<'a>> {
        match self {
            Self::Cell(cell) => Some(cell),
            Self::Merged { .. } => None,
        }
    }
}

/// Any grid child, which can be either a header or an item.
pub enum ResolvableGridChild<T: ResolvableCell, I> {
    Header { repeat: bool, level: NonZeroU32, span: Span, items: I },
    Footer { repeat: bool, span: Span, items: I },
    Item(ResolvableGridItem<T>),
}

/// A grid of cells, including the columns, rows, and cell data.
pub struct CellGrid<'a> {
    /// The grid cells.
    pub entries: Vec<Entry<'a>>,
    /// The column tracks including gutter tracks.
    pub cols: Vec<Sizing>,
    /// The row tracks including gutter tracks.
    pub rows: Vec<Sizing>,
    /// The vertical lines before each column, or on the end border.
    /// Gutter columns are not included.
    /// Contains up to 'cols_without_gutter.len() + 1' vectors of lines.
    pub vlines: Vec<Vec<Line>>,
    /// The horizontal lines on top of each row, or on the bottom border.
    /// Gutter rows are not included.
    /// Contains up to 'rows_without_gutter.len() + 1' vectors of lines.
    pub hlines: Vec<Vec<Line>>,
    /// The repeatable headers of this grid.
    pub headers: Vec<Repeatable<Header>>,
    /// The repeatable footer of this grid.
    pub footer: Option<Repeatable<Footer>>,
    /// Whether this grid has gutters.
    pub has_gutter: bool,
}

impl<'a> CellGrid<'a> {
    /// Generates the cell grid, given the tracks and cells.
    pub fn new(
        tracks: Axes<&[Sizing]>,
        gutter: Axes<&[Sizing]>,
        cells: impl IntoIterator<Item = Cell<'a>>,
    ) -> Self {
        let entries = cells.into_iter().map(Entry::Cell).collect();
        Self::new_internal(tracks, gutter, vec![], vec![], vec![], None, entries)
    }

    /// Generates the cell grid, given the tracks and resolved entries.
    pub fn new_internal(
        tracks: Axes<&[Sizing]>,
        gutter: Axes<&[Sizing]>,
        vlines: Vec<Vec<Line>>,
        hlines: Vec<Vec<Line>>,
        headers: Vec<Repeatable<Header>>,
        footer: Option<Repeatable<Footer>>,
        entries: Vec<Entry<'a>>,
    ) -> Self {
        let mut cols = vec![];
        let mut rows = vec![];

        // Number of content columns: Always at least one.
        let num_cols = tracks.x.len().max(1);

        // Number of content rows: At least as many as given, but also at least
        // as many as needed to place each item.
        let num_rows = {
            let len = entries.len();
            let given = tracks.y.len();
            let needed = len / num_cols + (len % num_cols).clamp(0, 1);
            given.max(needed)
        };

        let has_gutter = gutter.any(|tracks| !tracks.is_empty());
        let auto = Sizing::Auto;
        let zero = Sizing::Rel(Rel::zero());
        let get_or = |tracks: &[_], idx, default| {
            tracks.get(idx).or(tracks.last()).copied().unwrap_or(default)
        };

        // Collect content and gutter columns.
        for x in 0..num_cols {
            cols.push(get_or(tracks.x, x, auto));
            if has_gutter {
                cols.push(get_or(gutter.x, x, zero));
            }
        }

        // Collect content and gutter rows.
        for y in 0..num_rows {
            rows.push(get_or(tracks.y, y, auto));
            if has_gutter {
                rows.push(get_or(gutter.y, y, zero));
            }
        }

        // Remove superfluous gutter tracks.
        if has_gutter {
            cols.pop();
            rows.pop();
        }

        Self {
            cols,
            rows,
            entries,
            vlines,
            hlines,
            headers,
            footer,
            has_gutter,
        }
    }

    /// Get the grid entry in column `x` and row `y`.
    ///
    /// Returns `None` if it's a gutter cell.
    #[track_caller]
    pub fn entry(&self, x: usize, y: usize) -> Option<&Entry<'a>> {
        assert!(x < self.cols.len());
        assert!(y < self.rows.len());

        if self.has_gutter {
            // Even columns and rows are children, odd ones are gutter.
            if x % 2 == 0 && y % 2 == 0 {
                let c = 1 + self.cols.len() / 2;
                self.entries.get((y / 2) * c + x / 2)
            } else {
                None
            }
        } else {
            let c = self.cols.len();
            self.entries.get(y * c + x)
        }
    }

    /// Get the content of the cell in column `x` and row `y`.
    ///
    /// Returns `None` if it's a gutter cell or merged position.
    #[track_caller]
    pub fn cell(&self, x: usize, y: usize) -> Option<&Cell<'a>> {
        self.entry(x, y).and_then(Entry::as_cell)
    }

    /// Returns the position of the parent cell of the grid entry at the given
    /// position. It is guaranteed to have a non-gutter, non-merged cell at
    /// the returned position, due to how the grid is built.
    /// - If the entry at the given position is a cell, returns the given
    ///   position.
    /// - If it is a merged cell, returns the parent cell's position.
    /// - If it is a gutter cell, returns None.
    #[track_caller]
    pub fn parent_cell_position(&self, x: usize, y: usize) -> Option<Axes<usize>> {
        self.entry(x, y).map(|entry| match entry {
            Entry::Cell(_) => Axes::new(x, y),
            Entry::Merged { parent } => {
                let c = self.non_gutter_column_count();
                let factor = if self.has_gutter { 2 } else { 1 };
                Axes::new(factor * (*parent % c), factor * (*parent / c))
            }
        })
    }

    /// Returns the position of the actual parent cell of a merged position,
    /// even if the given position is gutter, in which case we return the
    /// parent of the nearest adjacent content cell which could possibly span
    /// the given gutter position. If the given position is not a gutter cell,
    /// then this function will return the same as `parent_cell_position` would.
    /// If the given position is a gutter cell, but no cell spans it, returns
    /// `None`.
    ///
    /// This is useful for lines. A line needs to check if a cell next to it
    /// has a stroke override - even at a gutter position there could be a
    /// stroke override, since a cell could be merged with two cells at both
    /// ends of the gutter cell (e.g. to its left and to its right), and thus
    /// that cell would impose a stroke under the gutter. This function allows
    /// getting the position of that cell (which spans the given gutter
    /// position, if it is gutter), if it exists; otherwise returns None (it's
    /// gutter and no cell spans it).
    #[track_caller]
    pub fn effective_parent_cell_position(
        &self,
        x: usize,
        y: usize,
    ) -> Option<Axes<usize>> {
        if self.has_gutter {
            // If (x, y) is a gutter cell, we skip it (skip a gutter column and
            // row) to the nearest adjacent content cell, in the direction
            // which merged cells grow toward (increasing x and increasing y),
            // such that we can verify if that adjacent cell is merged with the
            // gutter cell by checking if its parent would come before (x, y).
            // Otherwise, no cell is merged with this gutter cell, and we
            // return None.
            self.parent_cell_position(x + x % 2, y + y % 2)
                .filter(|&parent| parent.x <= x && parent.y <= y)
        } else {
            self.parent_cell_position(x, y)
        }
    }

    /// Checks if the track with the given index is gutter.
    /// Does not check if the index is a valid track.
    #[inline]
    pub fn is_gutter_track(&self, index: usize) -> bool {
        self.has_gutter && index % 2 == 1
    }

    /// Returns the effective colspan of a cell, considering the gutters it
    /// might span if the grid has gutters.
    #[inline]
    pub fn effective_colspan_of_cell(&self, cell: &Cell) -> usize {
        if self.has_gutter {
            2 * cell.colspan.get() - 1
        } else {
            cell.colspan.get()
        }
    }

    /// Returns the effective rowspan of a cell, considering the gutters it
    /// might span if the grid has gutters.
    #[inline]
    pub fn effective_rowspan_of_cell(&self, cell: &Cell) -> usize {
        if self.has_gutter {
            2 * cell.rowspan.get() - 1
        } else {
            cell.rowspan.get()
        }
    }

    #[inline]
    pub fn non_gutter_column_count(&self) -> usize {
        if self.has_gutter {
            // Calculation: With gutters, we have
            // 'cols = 2 * (non-gutter cols) - 1', since there is a gutter
            // column between each regular column. Therefore,
            // 'floor(cols / 2)' will be equal to
            // 'floor(non-gutter cols - 1/2) = non-gutter-cols - 1',
            // so 'non-gutter cols = 1 + floor(cols / 2)'.
            1 + self.cols.len() / 2
        } else {
            self.cols.len()
        }
    }

    #[inline]
    pub fn has_repeated_headers(&self) -> bool {
        self.headers.iter().any(|h| matches!(h, Repeatable::Repeated(_)))
    }
}

/// Resolves and positions all cells in the grid before creating it.
/// Allows them to keep track of their final properties and positions
/// and adjust their fields accordingly.
/// Cells must implement Clone as they will be owned. Additionally, they
/// must implement Default in order to fill positions in the grid which
/// weren't explicitly specified by the user with empty cells.
#[allow(clippy::too_many_arguments)]
pub fn resolve_cellgrid<'a, 'x, T, C, I>(
    tracks: Axes<&'a [Sizing]>,
    gutter: Axes<&'a [Sizing]>,
    locator: Locator<'x>,
    children: C,
    fill: &'a Celled<Option<Paint>>,
    align: &'a Celled<Smart<Alignment>>,
    inset: &'a Celled<Sides<Option<Rel<Length>>>>,
    stroke: &'a ResolvedCelled<Sides<Option<Option<Arc<Stroke>>>>>,
    engine: &'a mut Engine,
    styles: StyleChain<'a>,
    span: Span,
) -> SourceResult<CellGrid<'x>>
where
    T: ResolvableCell + Default,
    I: Iterator<Item = ResolvableGridItem<T>>,
    C: IntoIterator<Item = ResolvableGridChild<T, I>>,
    C::IntoIter: ExactSizeIterator,
{
    CellGridResolver {
        tracks,
        gutter,
        locator: locator.split(),
        fill,
        align,
        inset,
        stroke,
        engine,
        styles,
        span,
    }
    .resolve(children)
}

struct CellGridResolver<'a, 'b, 'x> {
    tracks: Axes<&'a [Sizing]>,
    gutter: Axes<&'a [Sizing]>,
    locator: SplitLocator<'x>,
    fill: &'a Celled<Option<Paint>>,
    align: &'a Celled<Smart<Alignment>>,
    inset: &'a Celled<Sides<Option<Rel<Length>>>>,
    stroke: &'a ResolvedCelled<Sides<Option<Option<Arc<Stroke>>>>>,
    engine: &'a mut Engine<'b>,
    styles: StyleChain<'a>,
    span: Span,
}

#[derive(Debug, Clone, Copy)]
enum RowGroupKind {
    Header,
    Footer,
}

impl RowGroupKind {
    fn name(self) -> &'static str {
        match self {
            Self::Header => "header",
            Self::Footer => "footer",
        }
    }
}

struct RowGroupData {
    /// The range of rows of cells inside this grid row group. The
    /// first and last rows are guaranteed to have cells (an exception
    /// is made when there is gutter, in which case the group range may
    /// be expanded to include an additional gutter row when there is a
    /// repeatable header or footer). This is `None` until the first
    /// cell of the row group is placed, then it is continually adjusted
    /// to fit the cells inside the row group.
    ///
    /// This stays as `None` for fully empty headers and footers.
    range: Option<Range<usize>>,
    span: Span,
    kind: RowGroupKind,

    /// Whether this header or footer may repeat.
    repeat: bool,

    /// Level of this header or footer.
    repeatable_level: NonZeroU32,

    /// Start of the range of indices of hlines at the top of the row group.
    /// This is always the first index after the last hline before we started
    /// building the row group - any upcoming hlines would appear at least at
    /// this index.
    ///
    /// These hlines were auto-positioned and appeared before any auto-pos
    /// cells, so they will appear at the first possible row (above the
    /// first row spanned by the row group).
    top_hlines_start: usize,

    /// End of the range of indices of hlines at the top of the row group.
    ///
    /// This starts as `None`, meaning that, if we stop the loop before we find
    /// any auto-pos cells, all auto-pos hlines after the last hline (after the
    /// index `top_hlines_start`) should be moved to the top of the row group.
    ///
    /// It becomes `Some(index of last hline at the top)` when an auto-pos cell
    /// is found, as auto-pos hlines after any auto-pos cells appear below
    /// them, not at the top of the row group.
    top_hlines_end: Option<usize>,
}

impl<'x> CellGridResolver<'_, '_, 'x> {
    fn resolve<T, C, I>(mut self, children: C) -> SourceResult<CellGrid<'x>>
    where
        T: ResolvableCell + Default,
        I: Iterator<Item = ResolvableGridItem<T>>,
        C: IntoIterator<Item = ResolvableGridChild<T, I>>,
        C::IntoIter: ExactSizeIterator,
    {
        // Number of content columns: Always at least one.
        let columns = self.tracks.x.len().max(1);

        // Lists of lines.
        // Horizontal lines are only pushed later to be able to check for row
        // validity, since the amount of rows isn't known until all items were
        // analyzed in the for loop below.
        // We keep their spans so we can report errors later.
        // The additional boolean indicates whether the hline had an automatic
        // 'y' index, and is used to change the index of hlines at the top of a
        // header or footer.
        let mut pending_hlines: Vec<(Span, Line, bool)> = vec![];

        // For consistency, only push vertical lines later as well.
        let mut pending_vlines: Vec<(Span, Line)> = vec![];
        let has_gutter = self.gutter.any(|tracks| !tracks.is_empty());

        let mut headers: Vec<Repeatable<Header>> = vec![];

        // Stores where the footer is supposed to end, its span, and the
        // actual footer structure.
        let mut footer: Option<(usize, Span, Footer)> = None;
        let mut repeat_footer = false;

        // We can't just use the cell's index in the 'cells' vector to
        // determine its automatic position, since cells could have arbitrary
        // positions, so the position of a cell in 'cells' can differ from its
        // final position in 'resolved_cells' (see below).
        // Therefore, we use a counter, 'auto_index', to determine the position
        // of the next cell with (x: auto, y: auto). It is only stepped when
        // a cell with (x: auto, y: auto), usually the vast majority, is found.
        //
        // Note that a separate counter ('local_auto_index') is used within
        // headers and footers, as explained above its definition. Outside of
        // those (when the table child being processed is a single cell),
        // 'local_auto_index' will simply be an alias for 'auto_index', which
        // will be updated after that cell is placed, if it is an
        // automatically-positioned cell.
        let mut auto_index: usize = 0;

        // We have to rebuild the grid to account for fixed cell positions.
        //
        // Create at least 'children.len()' positions, since there could be at
        // least 'children.len()' cells (if no explicit lines were specified),
        // even though some of them might be placed in fixed positions and thus
        // cause the grid to expand.
        //
        // Additionally, make sure we allocate up to the next multiple of
        // 'columns', since each row will have 'columns' cells, even if the
        // last few cells weren't explicitly specified by the user.
        let children = children.into_iter();
        let Some(child_count) = children.len().checked_next_multiple_of(columns) else {
            bail!(self.span, "too many cells or lines were given")
        };
        let mut resolved_cells: Vec<Option<Entry>> = Vec::with_capacity(child_count);
        for child in children {
            self.resolve_grid_child(
                columns,
                &mut pending_hlines,
                &mut pending_vlines,
                &mut headers,
                &mut footer,
                &mut repeat_footer,
                &mut auto_index,
                &mut resolved_cells,
                child,
            )?;
        }

        let resolved_cells = self.fixup_cells::<T>(resolved_cells, columns)?;

        let row_amount = resolved_cells.len().div_ceil(columns);
        let (hlines, vlines) = self.collect_lines(
            pending_hlines,
            pending_vlines,
            has_gutter,
            columns,
            row_amount,
        )?;

        let footer = self.finalize_headers_and_footers(
            has_gutter,
            &mut headers,
            footer,
            repeat_footer,
            row_amount,
        )?;

        Ok(CellGrid::new_internal(
            self.tracks,
            self.gutter,
            vlines,
            hlines,
            headers,
            footer,
            resolved_cells,
        ))
    }

    /// Resolve a grid child, which can be a header, a footer (both of which
    /// are row groups, and thus contain multiple grid items inside them), or
    /// a grid item - a cell, an hline or a vline.
    ///
    /// This process consists of placing the child and any sub-items into
    /// appropriate positions in the resolved grid. This is mostly relevant for
    /// items without fixed positions, such that they must be placed after the
    /// previous one, perhaps skipping existing cells along the way.
    #[allow(clippy::too_many_arguments)]
    fn resolve_grid_child<T, I>(
        &mut self,
        columns: usize,
        pending_hlines: &mut Vec<(Span, Line, bool)>,
        pending_vlines: &mut Vec<(Span, Line)>,
        headers: &mut Vec<Repeatable<Header>>,
        footer: &mut Option<(usize, Span, Footer)>,
        repeat_footer: &mut bool,
        auto_index: &mut usize,
        resolved_cells: &mut Vec<Option<Entry<'x>>>,
        child: ResolvableGridChild<T, I>,
    ) -> SourceResult<()>
    where
        T: ResolvableCell + Default,
        I: Iterator<Item = ResolvableGridItem<T>>,
    {
        // Data for the row group in this iteration.
        //
        // Note that cells outside headers and footers are grid children
        // with a single cell inside, and thus not considered row groups,
        // in which case this variable remains 'None'.
        let mut row_group_data: Option<RowGroupData> = None;

        // The normal auto index should only be stepped (upon placing an
        // automatically-positioned cell, to indicate the position of the
        // next) outside of headers or footers, in which case the auto
        // index will be updated with the local auto index. Inside headers
        // and footers, however, cells can only start after the first empty
        // row (as determined by 'first_available_row' below), meaning that
        // the next automatically-positioned cell will be in a different
        // position than it would usually be if it would be in a non-empty
        // row, so we must step a local index inside headers and footers
        // instead, and use a separate counter outside them.
        let mut local_auto_index = *auto_index;

        // The first row in which this table group can fit.
        //
        // Within headers and footers, this will correspond to the first
        // fully empty row available in the grid. This is because headers
        // and footers always occupy entire rows, so they cannot occupy
        // a non-empty row.
        let mut first_available_row = 0;

        let (header_footer_items, simple_item) = match child {
            ResolvableGridChild::Header { repeat, level, span, items, .. } => {
                row_group_data = Some(RowGroupData {
                    range: None,
                    span,
                    kind: RowGroupKind::Header,
                    repeat,
                    repeatable_level: level,
                    top_hlines_start: pending_hlines.len(),
                    top_hlines_end: None,
                });

                first_available_row =
                    find_next_empty_row(resolved_cells, local_auto_index, columns);

                // If any cell in the header is automatically positioned,
                // have it skip to the next empty row. This is to avoid
                // having a header after a partially filled row just add
                // cells to that row instead of starting a new one.
                //
                // Note that the first fully empty row is always after the
                // latest auto-position cell, since each auto-position cell
                // always occupies the first available position after the
                // previous one. Therefore, this will be >= auto_index.
                local_auto_index = first_available_row * columns;

                (Some(items), None)
            }
            ResolvableGridChild::Footer { repeat, span, items, .. } => {
                if footer.is_some() {
                    bail!(span, "cannot have more than one footer");
                }

                row_group_data = Some(RowGroupData {
                    range: None,
                    span,
                    repeat,
                    kind: RowGroupKind::Footer,
                    repeatable_level: NonZeroU32::ONE,
                    top_hlines_start: pending_hlines.len(),
                    top_hlines_end: None,
                });

                first_available_row =
                    find_next_empty_row(resolved_cells, local_auto_index, columns);

                local_auto_index = first_available_row * columns;

                (Some(items), None)
            }
            ResolvableGridChild::Item(item) => (None, Some(item)),
        };

        let items = header_footer_items.into_iter().flatten().chain(simple_item);
        for item in items {
            let cell = match item {
                ResolvableGridItem::HLine { y, start, end, stroke, span, position } => {
                    let has_auto_y = y.is_auto();
                    let y = y.unwrap_or_else(|| {
                        // Avoid placing the hline inside consecutive
                        // rowspans occupying all columns, as it'd just
                        // disappear, at least when there's no column
                        // gutter.
                        skip_auto_index_through_fully_merged_rows(
                            resolved_cells,
                            &mut local_auto_index,
                            columns,
                        );

                        // When no 'y' is specified for the hline, we place
                        // it under the latest automatically positioned
                        // cell.
                        // The current value of the auto index is always
                        // the index of the latest automatically positioned
                        // cell placed plus one (that's what we do in
                        // 'resolve_cell_position'), so we subtract 1 to
                        // get that cell's index, and place the hline below
                        // its row. The exception is when the auto_index is
                        // 0, meaning no automatically positioned cell was
                        // placed yet. In that case, we place the hline at
                        // the top of the table.
                        //
                        // Exceptionally, the hline will be placed before
                        // the minimum auto index if the current auto index
                        // from previous iterations is smaller than the
                        // minimum it should have for the current grid
                        // child. Effectively, this means that a hline at
                        // the start of a header will always appear above
                        // that header's first row. Similarly for footers.
                        local_auto_index
                            .checked_sub(1)
                            .map_or(0, |last_auto_index| last_auto_index / columns + 1)
                    });
                    if end.is_some_and(|end| end.get() < start) {
                        bail!(span, "line cannot end before it starts");
                    }
                    let line = Line { index: y, start, end, stroke, position };

                    // Since the amount of rows is dynamic, delay placing
                    // hlines until after all cells were placed so we can
                    // properly verify if they are valid. Note that we
                    // can't place hlines even if we already know they
                    // would be in a valid row, since it's possible that we
                    // pushed pending hlines in the same row as this one in
                    // previous iterations, and we need to ensure that
                    // hlines from previous iterations are pushed to the
                    // final vector of hlines first - the order of hlines
                    // must be kept, as this matters when determining which
                    // one "wins" in case of conflict. Pushing the current
                    // hline before we push pending hlines later would
                    // change their order!
                    pending_hlines.push((span, line, has_auto_y));
                    continue;
                }
                ResolvableGridItem::VLine { x, start, end, stroke, span, position } => {
                    let x = x.unwrap_or_else(|| {
                        // When no 'x' is specified for the vline, we place
                        // it after the latest automatically positioned
                        // cell.
                        // The current value of the auto index is always
                        // the index of the latest automatically positioned
                        // cell placed plus one (that's what we do in
                        // 'resolve_cell_position'), so we subtract 1 to
                        // get that cell's index, and place the vline after
                        // its column. The exception is when the auto_index
                        // is 0, meaning no automatically positioned cell
                        // was placed yet. In that case, we place the vline
                        // to the left of the table.
                        //
                        // Exceptionally, a vline is also placed to the
                        // left of the table when specified at the start
                        // of a row group, such as a header or footer, that
                        // is, when no automatically-positioned cells have
                        // been specified for that group yet.
                        // For example, this means that a vline at
                        // the beginning of a header will be placed to its
                        // left rather than after the previous
                        // automatically positioned cell. Same for footers.
                        local_auto_index
                            .checked_sub(1)
                            .filter(|_| local_auto_index > first_available_row * columns)
                            .map_or(0, |last_auto_index| last_auto_index % columns + 1)
                    });
                    if end.is_some_and(|end| end.get() < start) {
                        bail!(span, "line cannot end before it starts");
                    }
                    let line = Line { index: x, start, end, stroke, position };

                    // For consistency with hlines, we only push vlines to
                    // the final vector of vlines after processing every
                    // cell.
                    pending_vlines.push((span, line));
                    continue;
                }
                ResolvableGridItem::Cell(cell) => cell,
            };
            let cell_span = cell.span();
            let colspan = cell.colspan(self.styles).get();
            let rowspan = cell.rowspan(self.styles).get();
            // Let's calculate the cell's final position based on its
            // requested position.
            let resolved_index = {
                let cell_x = cell.x(self.styles);
                let cell_y = cell.y(self.styles);
                resolve_cell_position(
                    cell_x,
                    cell_y,
                    colspan,
                    rowspan,
                    headers,
                    footer.as_ref(),
                    resolved_cells,
                    &mut local_auto_index,
                    first_available_row,
                    columns,
                    row_group_data.is_some(),
                )
                .at(cell_span)?
            };
            let x = resolved_index % columns;
            let y = resolved_index / columns;

            if colspan > columns - x {
                bail!(
                    cell_span,
                    "cell's colspan would cause it to exceed the available column(s)";
                    hint: "try placing the cell in another position or reducing its colspan"
                )
            }

            let Some(largest_index) = columns
                .checked_mul(rowspan - 1)
                .and_then(|full_rowspan_offset| {
                    resolved_index.checked_add(full_rowspan_offset)
                })
                .and_then(|last_row_pos| last_row_pos.checked_add(colspan - 1))
            else {
                bail!(
                    cell_span,
                    "cell would span an exceedingly large position";
                    hint: "try reducing the cell's rowspan or colspan"
                )
            };

            // Cell's header or footer must expand to include the cell's
            // occupied positions, if possible.
            if let Some(RowGroupData {
                range: group_range, kind, top_hlines_end, ..
            }) = &mut row_group_data
            {
                *group_range = Some(
                    expand_row_group(
                        resolved_cells,
                        group_range.as_ref(),
                        *kind,
                        first_available_row,
                        y,
                        rowspan,
                        columns,
                    )
                    .at(cell_span)?,
                );

                if top_hlines_end.is_none()
                    && local_auto_index > first_available_row * columns
                {
                    // Auto index was moved, so upcoming auto-pos hlines should
                    // no longer appear at the top.
                    *top_hlines_end = Some(pending_hlines.len());
                }
            }

            // Let's resolve the cell so it can determine its own fields
            // based on its final position.
            let cell = self.resolve_cell(cell, x, y, rowspan, cell_span)?;

            if largest_index >= resolved_cells.len() {
                // Ensure the length of the vector of resolved cells is
                // always a multiple of 'columns' by pushing full rows every
                // time. Here, we add enough absent positions (later
                // converted to empty cells) to ensure the last row in the
                // new vector length is completely filled. This is
                // necessary so that those positions, even if not
                // explicitly used at the end, are eventually susceptible
                // to show rules and receive grid styling, as they will be
                // resolved as empty cells in a second loop below.
                let Some(new_len) = largest_index
                    .checked_add(1)
                    .and_then(|new_len| new_len.checked_next_multiple_of(columns))
                else {
                    bail!(cell_span, "cell position too large")
                };

                // Here, the cell needs to be placed in a position which
                // doesn't exist yet in the grid (out of bounds). We will
                // add enough absent positions for this to be possible.
                // They must be absent as no cells actually occupy them
                // (they can be overridden later); however, if no cells
                // occupy them as we finish building the grid, then such
                // positions will be replaced by empty cells.
                resolved_cells.resize_with(new_len, || None);
            }

            // The vector is large enough to contain the cell, so we can
            // just index it directly to access the position it will be
            // placed in. However, we still need to ensure we won't try to
            // place a cell where there already is one.
            let slot = &mut resolved_cells[resolved_index];
            if slot.is_some() {
                bail!(
                    cell_span,
                    "attempted to place a second cell at column {x}, row {y}";
                    hint: "try specifying your cells in a different order"
                );
            }

            *slot = Some(Entry::Cell(cell));

            // Now, if the cell spans more than one row or column, we fill
            // the spanned positions in the grid with Entry::Merged
            // pointing to the original cell as its parent.
            for rowspan_offset in 0..rowspan {
                let spanned_y = y + rowspan_offset;
                let first_row_index = resolved_index + columns * rowspan_offset;
                for (colspan_offset, slot) in
                    resolved_cells[first_row_index..][..colspan].iter_mut().enumerate()
                {
                    let spanned_x = x + colspan_offset;
                    if spanned_x == x && spanned_y == y {
                        // This is the parent cell.
                        continue;
                    }
                    if slot.is_some() {
                        bail!(
                            cell_span,
                            "cell would span a previously placed cell at column {spanned_x}, row {spanned_y}";
                            hint: "try specifying your cells in a different order or reducing the cell's rowspan or colspan"
                        )
                    }
                    *slot = Some(Entry::Merged { parent: resolved_index });
                }
            }
        }

        if let Some(row_group) = row_group_data {
            let group_range = match row_group.range {
                Some(group_range) => group_range,

                None => {
                    // Empty header/footer: consider the header/footer to be
                    // at the next empty row after the latest auto index.
                    local_auto_index = first_available_row * columns;
                    let group_start = first_available_row;
                    let group_end = group_start + 1;

                    if resolved_cells.len() <= columns * group_start {
                        // Ensure the automatically chosen row actually exists.
                        resolved_cells.resize_with(columns * (group_start + 1), || None);
                    }

                    // Even though this header or footer is fully empty, we add one
                    // default cell to maintain the invariant that each header and
                    // footer has at least one 'Some(...)' cell at its first row
                    // and at least one at its last row (here they are the same
                    // row, of course). This invariant is important to ensure
                    // 'find_next_empty_row' will skip through any existing headers
                    // and footers without having to loop through them each time.
                    // Cells themselves, unfortunately, still have to.
                    assert!(resolved_cells[local_auto_index].is_none());
                    resolved_cells[local_auto_index] =
                        Some(Entry::Cell(self.resolve_cell(
                            T::default(),
                            0,
                            first_available_row,
                            1,
                            Span::detached(),
                        )?));

                    group_start..group_end
                }
            };

            let top_hlines_end = row_group.top_hlines_end.unwrap_or(pending_hlines.len());
            for (_, top_hline, has_auto_y) in pending_hlines
                .get_mut(row_group.top_hlines_start..top_hlines_end)
                .unwrap_or(&mut [])
            {
                if *has_auto_y {
                    // Move this hline to the top of the child, as it was
                    // placed before the first automatically positioned cell
                    // and had an automatic index.
                    top_hline.index = group_range.start;
                }
            }

            match row_group.kind {
                RowGroupKind::Header => {
                    let data = Header {
                        start: group_range.start,

                        // Later on, we have to correct this number in case there
                        // is gutter. But only once all cells have been analyzed
                        // and the header has fully expanded in the fixup loop
                        // below.
                        end: group_range.end,

                        level: row_group.repeatable_level.get(),
                    };

                    headers.push(if row_group.repeat {
                        Repeatable::Repeated(data)
                    } else {
                        Repeatable::NotRepeated(data)
                    });
                }

                RowGroupKind::Footer => {
                    // Only check if the footer is at the end later, once we know
                    // the final amount of rows.
                    *footer = Some((
                        group_range.end,
                        row_group.span,
                        Footer {
                            // Later on, we have to correct this number in case there
                            // is gutter, but only once all cells have been analyzed
                            // and the header's and footer's exact boundaries are
                            // known. That is because the gutter row immediately
                            // before the footer might not be included as part of
                            // the footer if it is contained within the header.
                            start: group_range.start,
                            end: group_range.end,
                            level: 1,
                        },
                    ));

                    *repeat_footer = row_group.repeat;
                }
            }
        } else {
            // The child was a single cell outside headers or footers.
            // Therefore, 'local_auto_index' for this table child was
            // simply an alias for 'auto_index', so we update it as needed.
            *auto_index = local_auto_index;
        }

        Ok(())
    }

    /// Fixup phase (final step in cell grid generation):
    ///
    /// 1. Replace absent entries by resolved empty cells, producing a vector
    ///    of `Entry` from `Option<Entry>`.
    ///
    /// 2. Add enough empty cells to the end of the grid such that it has at
    ///    least the given amount of rows (must be a multiple of `columns`,
    ///    and all rows before the last cell must have cells, empty or not,
    ///    even if the user didn't specify those cells).
    ///
    ///    That is necessary, for example, to ensure even unspecified cells
    ///    can be affected by show rules and grid-wide styling.
    fn fixup_cells<T>(
        &mut self,
        resolved_cells: Vec<Option<Entry<'x>>>,
        columns: usize,
    ) -> SourceResult<Vec<Entry<'x>>>
    where
        T: ResolvableCell + Default,
    {
        let Some(expected_total_cells) = columns.checked_mul(self.tracks.y.len()) else {
            bail!(self.span, "too many rows were specified");
        };
        let missing_cells = expected_total_cells.saturating_sub(resolved_cells.len());

        resolved_cells
            .into_iter()
            .chain(std::iter::repeat_with(|| None).take(missing_cells))
            .enumerate()
            .map(|(i, cell)| {
                if let Some(cell) = cell {
                    Ok(cell)
                } else {
                    let x = i % columns;
                    let y = i / columns;

                    Ok(Entry::Cell(self.resolve_cell(
                        T::default(),
                        x,
                        y,
                        1,
                        Span::detached(),
                    )?))
                }
            })
            .collect::<SourceResult<Vec<Entry>>>()
    }

    /// Takes the list of pending lines and evaluates a final list of hlines
    /// and vlines (in that order in the returned tuple), detecting invalid
    /// line positions in the process.
    ///
    /// For each line type (horizontal and vertical respectively), returns a
    /// vector containing one inner vector for every group of lines with the
    /// same index.
    ///
    /// For example, an hline above the second row (y = 1) is inside the inner
    /// vector at position 1 of the first vector (hlines) returned by this
    /// function.
    #[allow(clippy::type_complexity)]
    fn collect_lines(
        &self,
        pending_hlines: Vec<(Span, Line, bool)>,
        pending_vlines: Vec<(Span, Line)>,
        has_gutter: bool,
        columns: usize,
        row_amount: usize,
    ) -> SourceResult<(Vec<Vec<Line>>, Vec<Vec<Line>>)> {
        let mut hlines: Vec<Vec<Line>> = vec![];
        let mut vlines: Vec<Vec<Line>> = vec![];

        for (line_span, line, _) in pending_hlines {
            let y = line.index;
            if y > row_amount {
                bail!(line_span, "cannot place horizontal line at invalid row {y}");
            }
            if y == row_amount && line.position == LinePosition::After {
                bail!(
                    line_span,
                    "cannot place horizontal line at the 'bottom' position of the bottom border (y = {y})";
                    hint: "set the line's position to 'top' or place it at a smaller 'y' index"
                );
            }
            let line = if line.position == LinePosition::After
                && (!has_gutter || y + 1 == row_amount)
            {
                // Just place the line on top of the next row if
                // there's no gutter and the line should be placed
                // after the one with given index.
                //
                // Note that placing after the last row is also the same as
                // just placing on the grid's bottom border, even with
                // gutter.
                Line {
                    index: y + 1,
                    position: LinePosition::Before,
                    ..line
                }
            } else {
                line
            };
            let y = line.index;

            if hlines.len() <= y {
                hlines.resize_with(y + 1, Vec::new);
            }
            hlines[y].push(line);
        }

        for (line_span, line) in pending_vlines {
            let x = line.index;
            if x > columns {
                bail!(line_span, "cannot place vertical line at invalid column {x}");
            }
            if x == columns && line.position == LinePosition::After {
                bail!(
                    line_span,
                    "cannot place vertical line at the 'end' position of the end border (x = {columns})";
                    hint: "set the line's position to 'start' or place it at a smaller 'x' index"
                );
            }
            let line = if line.position == LinePosition::After
                && (!has_gutter || x + 1 == columns)
            {
                // Just place the line before the next column if
                // there's no gutter and the line should be placed
                // after the one with given index.
                //
                // Note that placing after the last column is also the
                // same as just placing on the grid's end border, even
                // with gutter.
                Line {
                    index: x + 1,
                    position: LinePosition::Before,
                    ..line
                }
            } else {
                line
            };
            let x = line.index;

            if vlines.len() <= x {
                vlines.resize_with(x + 1, Vec::new);
            }
            vlines[x].push(line);
        }

        Ok((hlines, vlines))
    }

    /// Generate the final headers and footers:
    ///
    /// 1. Convert gutter-ignorant to gutter-aware indices if necessary;
    /// 2. Expand the header downwards (or footer upwards) to also include
    ///    an adjacent gutter row to be repeated alongside that header or
    ///    footer, if there is gutter;
    /// 3. Wrap headers and footers in the correct [`Repeatable`] variant.
    #[allow(clippy::type_complexity)]
    fn finalize_headers_and_footers(
        &self,
        has_gutter: bool,
        headers: &mut [Repeatable<Header>],
        footer: Option<(usize, Span, Footer)>,
        repeat_footer: bool,
        row_amount: usize,
    ) -> SourceResult<Option<Repeatable<Footer>>> {
        // Repeat the gutter below a header (hence why we don't
        // subtract 1 from the gutter case).
        // Don't do this if there are no rows under the header.
        if has_gutter {
            for header in &mut *headers {
                let header = header.unwrap_mut();

                // Index of first y is doubled, as each row before it
                // receives a gutter row below.
                header.start *= 2;

                // - 'header.end' is always 'last y + 1'. The header stops
                // before that row.
                // - Therefore, '2 * header.end' will be 2 * (last y + 1),
                // which is the adjusted index of the row before which the
                // header stops, meaning it will still stop right before it
                // even with gutter thanks to the multiplication below.
                // - This means that it will span all rows up to
                // '2 * (last y + 1) - 1 = 2 * last y + 1', which equates
                // to the index of the gutter row right below the header,
                // which is what we want (that gutter spacing should be
                // repeated across pages to maintain uniformity).
                header.end *= 2;

                // If the header occupies the entire grid, ensure we don't
                // include an extra gutter row when it doesn't exist, since
                // the last row of the header is at the very bottom,
                // therefore '2 * last y + 1' is not a valid index.
                let row_amount = (2 * row_amount).saturating_sub(1);
                header.end = header.end.min(row_amount);
            }
        }

        let footer = footer
            .map(|(footer_end, footer_span, mut footer)| {
                if footer_end != row_amount {
                    bail!(footer_span, "footer must end at the last row");
                }

                // TODO: will need a global slice of headers and footers for
                // when we have multiple footers
                let last_header_end =
                    headers.last().map(Repeatable::unwrap).map(|header| header.end);

                if has_gutter {
                    // Convert the footer's start index to post-gutter coordinates.
                    footer.start *= 2;

                    // Include the gutter right before the footer, unless there is
                    // none, or the gutter is already included in the header (no
                    // rows between the header and the footer).
                    if last_header_end != Some(footer.start) {
                        footer.start = footer.start.saturating_sub(1);
                    }

                    // Adapt footer end but DO NOT include the gutter below it,
                    // if it exists. Calculation:
                    // - Starts as 'last y + 1'.
                    // - The result will be
                    // 2 * (last_y + 1) - 1 = 2 * last_y + 1,
                    // which is the new index of the last footer row plus one,
                    // meaning we do exclude any gutter below this way.
                    //
                    // It also keeps us within the total amount of rows, so we
                    // don't need to '.min()' later.
                    footer.end = (2 * footer.end).saturating_sub(1);
                }

                Ok(footer)
            })
            .transpose()?
            .map(|footer| {
                if repeat_footer {
                    Repeatable::Repeated(footer)
                } else {
                    Repeatable::NotRepeated(footer)
                }
            });

        Ok(footer)
    }

    /// Resolves the cell's fields based on grid-wide properties.
    fn resolve_cell<T>(
        &mut self,
        cell: T,
        x: usize,
        y: usize,
        rowspan: usize,
        cell_span: Span,
    ) -> SourceResult<Cell<'x>>
    where
        T: ResolvableCell + Default,
    {
        // Resolve the breakability of a cell. Cells that span at least one
        // auto-sized row or gutter are considered breakable.
        let breakable = {
            let auto = Sizing::Auto;
            let zero = Sizing::Rel(Rel::zero());
            self.tracks
                .y
                .iter()
                .chain(std::iter::repeat(self.tracks.y.last().unwrap_or(&auto)))
                .skip(y)
                .take(rowspan)
                .any(|row| row == &Sizing::Auto)
                || self
                    .gutter
                    .y
                    .iter()
                    .chain(std::iter::repeat(self.gutter.y.last().unwrap_or(&zero)))
                    .skip(y)
                    .take(rowspan - 1)
                    .any(|row_gutter| row_gutter == &Sizing::Auto)
        };

        Ok(cell.resolve_cell(
            x,
            y,
            &self.fill.resolve(self.engine, self.styles, x, y)?,
            self.align.resolve(self.engine, self.styles, x, y)?,
            self.inset.resolve(self.engine, self.styles, x, y)?,
            self.stroke.resolve(self.engine, self.styles, x, y)?,
            breakable,
            self.locator.next(&cell_span),
            self.styles,
        ))
    }
}

/// Given the existing range of a row group (header or footer), tries to expand
/// it to fit the new cell placed inside it. If the newly-expanded row group
/// would conflict with existing cells or other row groups, an error is
/// returned. Otherwise, the new `start..end` range of rows in the row group is
/// returned.
fn expand_row_group(
    resolved_cells: &[Option<Entry<'_>>],
    group_range: Option<&Range<usize>>,
    group_kind: RowGroupKind,
    first_available_row: usize,
    cell_y: usize,
    rowspan: usize,
    columns: usize,
) -> HintedStrResult<Range<usize>> {
    // Ensure each cell in a header or footer is fully contained within it by
    // expanding the header or footer towards this new cell.
    let (new_group_start, new_group_end) = group_range
        .map_or((cell_y, cell_y + rowspan), |r| {
            (r.start.min(cell_y), r.end.max(cell_y + rowspan))
        });

    // This check might be unnecessary with the loop below, but let's keep it
    // here for full correctness.
    //
    // Quickly detect the case:
    // y = 0 => occupied
    // y = 1 => empty
    // y = 2 => header
    // and header tries to expand to y = 0 - invalid, as
    // 'y = 1' is the earliest row it can occupy.
    if new_group_start < first_available_row {
        bail!(
            "cell would cause {} to expand to non-empty row {}",
            group_kind.name(),
            first_available_row.saturating_sub(1);
            hint: "try moving its cells to available rows"
        );
    }

    let new_rows =
        group_range.map_or((new_group_start..new_group_end).chain(0..0), |r| {
            // NOTE: 'r.end' is one row AFTER the row group's last row, so it
            // makes sense to check it if 'new_group_end > r.end', that is, if
            // the row group is going to expand. It is NOT a duplicate check,
            // as we hadn't checked it before (in a previous run, it was
            // 'new_group_end' at the exclusive end of the range)!
            //
            // NOTE: To keep types the same, we have to always return
            // '(range).chain(range)', which justifies chaining an empty
            // range above.
            (new_group_start..r.start).chain(r.end..new_group_end)
        });

    // The check above isn't enough, however, even when the header is expanding
    // upwards, as it might expand upwards towards an occupied row after the
    // first empty row, e.g.
    //
    // y = 0 => occupied
    // y = 1 => empty (first_available_row = 1)
    // y = 2 => occupied
    // y = 3 => header
    //
    // Here, we should bail if the header tries to expand upwards, regardless
    // of the fact that the conflicting row (y = 2) comes after the first
    // available row.
    //
    // Note that expanding upwards is only possible when row-positioned cells
    // are specified, in one of the following cases:
    //
    // 1. We place e.g. 'table.cell(y: 3)' followed by 'table.cell(y: 2)'
    // (earlier row => upwards);
    //
    // 2. We place e.g. 'table.cell(y: 3)' followed by '[a]' (auto-pos cell
    // favors 'first_available_row', so the header tries to expand upwards to
    // place the cell at 'y = 1' and conflicts at 'y = 2') or
    // 'table.cell(x: 1)' (same deal).
    //
    // Of course, we also need to check for downward expansion as usual as
    // there could be a non-empty row below the header, but the upward case is
    // highlighted as it was checked separately before (and also to explain
    // what kind of situation we are preventing with this check).
    //
    // Note that simply checking for non-empty rows like below not only
    // prevents conflicts with top-level cells (outside of headers and
    // footers), but also prevents conflicts with other headers or footers,
    // since we have an invariant that even empty headers and footers must
    // contain at least one 'Some(...)' position in 'resolved_cells'. More
    // precisely, each header and footer has at least one 'Some(...)' cell at
    // 'group_range.start' and at 'group_range.end - 1' - non-empty headers and
    // footers don't span any unnecessary rows. Therefore, we don't have to
    // loop over headers and footers, only check if the new rows are empty.
    for new_y in new_rows {
        if let Some(new_row @ [_non_empty, ..]) = resolved_cells
            .get(new_y * columns..)
            .map(|cells| &cells[..columns.min(cells.len())])
        {
            if new_row.iter().any(Option::is_some) {
                bail!(
                    "cell would cause {} to expand to non-empty row {new_y}",
                    group_kind.name();
                    hint: "try moving its cells to available rows",
                )
            }
        } else {
            // Received 'None' or an empty slice, so we are expanding the
            // header or footer into new rows, which is always valid and cannot
            // conflict with existing cells. (Note that we only resize
            // 'resolved_cells' after this function is called, so, if this
            // header or footer is at the bottom of the table so far, this loop
            // will end quite early, regardless of where this cell was placed
            // or of its rowspan value.)
            break;
        }
    }

    Ok(new_group_start..new_group_end)
}

/// Check if a cell's fixed row would conflict with a header or footer.
fn check_for_conflicting_cell_row(
    headers: &[Repeatable<Header>],
    footer: Option<&(usize, Span, Footer)>,
    cell_y: usize,
    rowspan: usize,
) -> HintedStrResult<()> {
    // TODO: use upcoming headers slice to make this an O(1) check
    // NOTE: y + rowspan >, not >=, header.start, to check if the rowspan
    // enters the header. For example, consider a rowspan of 1: if
    // `y + 1 = header.start` holds, that means `y < header.start`, and it
    // only occupies one row (`y`), so the cell is actually not in
    // conflict.
    if headers.iter().any(|header| {
        cell_y < header.unwrap().end && cell_y + rowspan > header.unwrap().start
    }) {
        bail!(
            "cell would conflict with header spanning the same position";
            hint: "try moving the cell or the header"
        );
    }

    if let Some((_, _, footer)) = footer {
        if cell_y < footer.end && cell_y + rowspan > footer.start {
            bail!(
                "cell would conflict with footer spanning the same position";
                hint: "try reducing the cell's rowspan or moving the footer"
            );
        }
    }

    Ok(())
}

/// Given a cell's requested x and y, the vector with the resolved cell
/// positions, the `auto_index` counter (determines the position of the next
/// `(auto, auto)` cell) and the amount of columns in the grid, returns the
/// final index of this cell in the vector of resolved cells.
///
/// The `first_available_row` parameter is used by headers and footers to
/// indicate the first empty row available. Any rows before those should
/// not be picked by cells with `auto` row positioning, since headers and
/// footers occupy entire rows, and may not conflict with cells outside them.
#[allow(clippy::too_many_arguments)]
fn resolve_cell_position(
    cell_x: Smart<usize>,
    cell_y: Smart<usize>,
    colspan: usize,
    rowspan: usize,
    headers: &[Repeatable<Header>],
    footer: Option<&(usize, Span, Footer)>,
    resolved_cells: &[Option<Entry>],
    auto_index: &mut usize,
    first_available_row: usize,
    columns: usize,
    in_row_group: bool,
) -> HintedStrResult<usize> {
    // Translates a (x, y) position to the equivalent index in the final cell vector.
    // Errors if the position would be too large.
    let cell_index = |x, y: usize| {
        y.checked_mul(columns)
            .and_then(|row_index| row_index.checked_add(x))
            .ok_or_else(|| HintedString::from(eco_format!("cell position too large")))
    };
    match (cell_x, cell_y) {
        // Fully automatic cell positioning. The cell did not
        // request a coordinate.
        (Smart::Auto, Smart::Auto) => {
            // Let's find the first available position starting from the
            // automatic position counter, searching in row-major order.
            // Note that the counter ignores any cells with fixed positions,
            // but automatically-positioned cells will avoid conflicts by
            // simply skipping existing cells, headers and footers.
            let resolved_index = find_next_available_position::<false>(
                headers,
                footer,
                resolved_cells,
                columns,
                *auto_index,
            )?;

            // Ensure the next cell with automatic position will be
            // placed after this one (maybe not immediately after).
            //
            // The calculation below also affects the position of the upcoming
            // automatically-positioned lines, as they are placed below
            // (horizontal lines) or to the right (vertical lines) of the cell
            // that would be placed at 'auto_index'.
            *auto_index = if colspan == columns {
                // The cell occupies all columns, so no cells can be placed
                // after it until all of its rows have been spanned.
                resolved_index + colspan * rowspan
            } else {
                // The next cell will have to be placed at least after its
                // spanned columns.
                resolved_index + colspan
            };

            Ok(resolved_index)
        }
        // Cell has chosen at least its column.
        (Smart::Custom(cell_x), cell_y) => {
            if cell_x >= columns {
                return Err(HintedString::from(eco_format!(
                    "cell could not be placed at invalid column {cell_x}"
                )));
            }
            if let Smart::Custom(cell_y) = cell_y {
                // Cell has chosen its exact position.
                //
                // Ensure it doesn't conflict with an existing header or
                // footer (but only if it isn't already in one, otherwise there
                // will already be a separate check).
                if !in_row_group {
                    check_for_conflicting_cell_row(headers, footer, cell_y, rowspan)?;
                }

                cell_index(cell_x, cell_y)
            } else {
                // Cell has only chosen its column.
                // Let's find the first row which has that column available.
                // If in a header or footer, start searching by the first empty
                // row / the header or footer's first row (specified through
                // 'first_available_row'). Otherwise, start searching at the
                // first row.
                let initial_index = cell_index(cell_x, first_available_row)?;

                // Try each row until either we reach an absent position at the
                // requested column ('Some(None)') or an out of bounds position
                // ('None'), in which case we'd create a new row to place this
                // cell in.
                find_next_available_position::<true>(
                    headers,
                    footer,
                    resolved_cells,
                    columns,
                    initial_index,
                )
            }
        }
        // Cell has only chosen its row, not its column.
        (Smart::Auto, Smart::Custom(cell_y)) => {
            // Ensure it doesn't conflict with an existing header or
            // footer (but only if it isn't already in one, otherwise there
            // will already be a separate check).
            if !in_row_group {
                check_for_conflicting_cell_row(headers, footer, cell_y, rowspan)?;
            }

            // Let's find the first column which has that row available.
            let first_row_pos = cell_index(0, cell_y)?;
            let last_row_pos = first_row_pos
                .checked_add(columns)
                .ok_or_else(|| eco_format!("cell position too large"))?;

            (first_row_pos..last_row_pos)
                .find(|possible_index| {
                    // Much like in the previous cases, we skip any occupied
                    // positions until we either reach an absent position
                    // (`Some(None)`) or an out of bounds position (`None`),
                    // in which case we can just expand the vector enough to
                    // place this cell. In either case, we found an available
                    // position.
                    !matches!(resolved_cells.get(*possible_index), Some(Some(_)))
                })
                .ok_or_else(|| {
                    eco_format!(
                        "cell could not be placed in row {cell_y} because it was full"
                    )
                })
                .hint("try specifying your cells in a different order")
        }
    }
}

/// Finds the first available position after the initial index in the resolved
/// grid of cells. Skips any non-absent positions (positions which already
/// have cells specified by the user) as well as any headers and footers.
#[inline]
fn find_next_available_position<const SKIP_ROWS: bool>(
    headers: &[Repeatable<Header>],
    footer: Option<&(usize, Span, Footer)>,
    resolved_cells: &[Option<Entry<'_>>],
    columns: usize,
    initial_index: usize,
) -> HintedStrResult<usize> {
    let mut resolved_index = initial_index;

    loop {
        if let Some(Some(_)) = resolved_cells.get(resolved_index) {
            // Skip any non-absent cell positions (`Some(None)`) to
            // determine where this cell will be placed. An out of
            // bounds position (thus `None`) is also a valid new
            // position (only requires expanding the vector).
            if SKIP_ROWS {
                // Skip one row at a time (cell chose its column, so we don't
                // change it).
                resolved_index =
                    resolved_index.checked_add(columns).ok_or_else(|| {
                        HintedString::from(eco_format!("cell position too large"))
                    })?;
            } else {
                // Ensure we don't run unnecessary checks in the hot path
                // (for fully automatically-positioned cells). Memory usage
                // would become impractically large before this overflows.
                resolved_index += 1;
            }
        // TODO: consider keeping vector of upcoming headers to make this check
        // non-quadratic (O(cells) instead of O(headers * cells)).
        } else if let Some(header) =
            headers.iter().map(Repeatable::unwrap).find(|header| {
                (header.start * columns..header.end * columns).contains(&resolved_index)
            })
        {
            // Skip header (can't place a cell inside it from outside it).
            resolved_index = header.end * columns;

            if SKIP_ROWS {
                // Ensure the cell's chosen column is kept after the
                // header.
                resolved_index += initial_index % columns;
            }
        } else if let Some((footer_end, _, _)) = footer.filter(|(end, _, footer)| {
            resolved_index >= footer.start * columns && resolved_index < *end * columns
        }) {
            // Skip footer, for the same reason.
            resolved_index = *footer_end * columns;

            if SKIP_ROWS {
                resolved_index += initial_index % columns;
            }
        } else {
            return Ok(resolved_index);
        }
    }
}

/// Computes the `y` of the next available empty row, given the auto index as
/// an initial index for search, since we know that there are no empty rows
/// before automatically-positioned cells, as they are placed sequentially.
fn find_next_empty_row(
    resolved_cells: &[Option<Entry>],
    auto_index: usize,
    columns: usize,
) -> usize {
    let mut resolved_index = auto_index.next_multiple_of(columns);
    while resolved_cells
        .get(resolved_index..resolved_index + columns)
        .is_some_and(|row| row.iter().any(Option::is_some))
    {
        // Skip non-empty rows.
        resolved_index += columns;
    }

    resolved_index / columns
}

/// Fully merged rows under the cell of latest auto index indicate rowspans
/// occupying all columns, so we skip the auto index until the shortest rowspan
/// ends, such that, in the resulting row, we will be able to place an
/// automatically positioned cell - and, in particular, hlines under it. The
/// idea is that an auto hline will be placed after the shortest such rowspan.
/// Otherwise, the hline would just be placed under the first row of those
/// rowspans and disappear (except at the presence of column gutter).
fn skip_auto_index_through_fully_merged_rows(
    resolved_cells: &[Option<Entry>],
    auto_index: &mut usize,
    columns: usize,
) {
    // If the auto index isn't currently at the start of a row, that means
    // there's still at least one auto position left in the row, ignoring
    // cells with manual positions, so we wouldn't have a problem in placing
    // further cells or, in this case, hlines here.
    if *auto_index % columns == 0 {
        while resolved_cells
            .get(*auto_index..*auto_index + columns)
            .is_some_and(|row| {
                row.iter().all(|entry| matches!(entry, Some(Entry::Merged { .. })))
            })
        {
            *auto_index += columns;
        }
    }
}
