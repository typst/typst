mod layouter;
mod lines;
mod repeated;
mod rowspans;

use typst_library::diag::SourceResult;
use typst_library::engine::Engine;
use typst_library::foundations::{Packed, StyleChain};
use typst_library::introspection::Locator;
use typst_library::layout::raster::{grid_to_raster, table_to_raster, Cell, Repeatable};
use typst_library::layout::{Fragment, GridElem, Regions};
use typst_library::model::TableElem;

pub use self::layouter::GridLayouter;
use self::layouter::RowPiece;
use self::lines::{
    generate_line_segments, hline_stroke_at_column, vline_stroke_at_row, LineSegment,
};
use self::rowspans::{Rowspan, UnbreakableRowGroup};

/// Layout the cell into the given regions.
///
/// The `disambiguator` indicates which instance of this cell this should be
/// layouted as. For normal cells, it is always `0`, but for headers and
/// footers, it indicates the index of the header/footer among all. See the
/// [`Locator`] docs for more details on the concepts behind this.
fn layout_cell(
    cell: &Cell,
    engine: &mut Engine,
    disambiguator: usize,
    styles: StyleChain,
    regions: Regions,
) -> SourceResult<Fragment> {
    let mut locator = cell.locator.relayout();
    if disambiguator > 0 {
        locator = locator.split().next_inner(disambiguator as u128);
    }
    crate::layout_fragment(engine, &cell.body, locator, styles, regions)
}

/// Layout the grid.
#[typst_macros::time(span = elem.span())]
pub fn layout_grid(
    elem: &Packed<GridElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    regions: Regions,
) -> SourceResult<Fragment> {
    let grid = grid_to_raster(elem, engine, locator, styles)?;
    let layouter = GridLayouter::new(&grid, regions, styles, elem.span());

    // Measure the columns and layout the grid row-by-row.
    layouter.layout(engine)
}

/// Layout the table.
#[typst_macros::time(span = elem.span())]
pub fn layout_table(
    elem: &Packed<TableElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    regions: Regions,
) -> SourceResult<Fragment> {
    let grid = table_to_raster(elem, engine, locator, styles)?;

    let layouter = GridLayouter::new(&grid, regions, styles, elem.span());
    layouter.layout(engine)
}
