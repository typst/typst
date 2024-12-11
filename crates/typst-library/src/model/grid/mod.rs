//! Unified handling for tables and grids.
mod cells;

use std::num::NonZeroUsize;
use std::sync::Arc;

use ecow::eco_format;
use typst_library::diag::{SourceResult, Trace, Tracepoint};
use typst_library::engine::Engine;
use typst_library::foundations::{Fold, Packed, Smart, StyleChain};
use typst_library::introspection::Locator;
use typst_library::layout::{
    Abs, Alignment, Axes, Dir, GridCell, GridChild, GridElem, GridItem, Length,
    OuterHAlignment, OuterVAlignment, Rel, Sides,
};
use typst_library::model::{TableCell, TableChild, TableElem, TableItem};
use typst_library::text::TextElem;
use typst_library::visualize::{Paint, Stroke};
use typst_syntax::Span;

pub use self::cells::{Cell, CellGrid, Footer, Header, Line, LinePosition, Repeatable};
use self::cells::{ResolvableCell, ResolvableGridChild, ResolvableGridItem};

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
        GridChild::Item(item) => {
            ResolvableGridChild::Item(grid_item_to_resolvable(item, styles))
        }
    });
    CellGrid::resolve(
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
    let children = elem.children().iter().map(|child| match child {
        TableChild::Header(header) => ResolvableGridChild::Header {
            repeat: header.repeat(styles),
            span: header.span(),
            items: header.children().iter().map(resolve_item),
        },
        TableChild::Footer(footer) => ResolvableGridChild::Footer {
            repeat: footer.repeat(styles),
            span: footer.span(),
            items: footer.children().iter().map(resolve_item),
        },
        TableChild::Item(item) => {
            ResolvableGridChild::Item(table_item_to_resolvable(item, styles))
        }
    });
    CellGrid::resolve(
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
