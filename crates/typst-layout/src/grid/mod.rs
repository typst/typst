mod layouter;
mod lines;
mod repeated;
mod rowspans;

pub use self::layouter::GridLayouter;

use typst_library::diag::SourceResult;
use typst_library::engine::Engine;
use typst_library::foundations::{NativeElement, Packed, StyleChain};
use typst_library::introspection::{Locator, SplitLocator, Tag};
use typst_library::layout::grid::resolve::{Cell, grid_to_cellgrid, table_to_cellgrid};
use typst_library::layout::{Fragment, FrameItem, GridCell, GridElem, Point, Regions};
use typst_library::model::{TableCell, TableElem};

use self::layouter::RowPiece;
use self::lines::{
    LineSegment, generate_line_segments, hline_stroke_at_column, vline_stroke_at_row,
};
use self::rowspans::{Rowspan, UnbreakableRowGroup};

/// Layout the cell into the given regions.
///
/// The `disambiguator` indicates which instance of this cell this should be
/// layouted as. For normal cells, it is always `0`, but for headers and
/// footers, it indicates the index of the header/footer among all. See the
/// [`Locator`] docs for more details on the concepts behind this.
pub fn layout_cell(
    cell: &Cell,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    regions: Regions,
    is_repeated: bool,
) -> SourceResult<Fragment> {
    // HACK: manually generate tags for table and grid cells. Ideally table and
    // grid cells could just be marked as locatable, but the tags are somehow
    // considered significant for layouting. This hack together with a check in
    // the grid layouter makes the test suite pass.
    let mut locator = locator.split();
    let tags = if let Some(table_cell) = cell.body.to_packed::<TableCell>() {
        let mut table_cell = table_cell.clone();
        table_cell.is_repeated.set(is_repeated);
        Some(generate_tags(table_cell, &mut locator, engine))
    } else if let Some(grid_cell) = cell.body.to_packed::<GridCell>() {
        Some(generate_tags(grid_cell.clone(), &mut locator, engine))
    } else {
        None
    };

    let locator = locator.next(&cell.body.span());
    let fragment = crate::layout_fragment(engine, &cell.body, locator, styles, regions)?;

    // Manually insert tags.
    let mut frames = fragment.into_frames();
    if let Some((start, end)) = tags {
        if let Some(first) = frames.first_mut() {
            first.prepend(Point::zero(), FrameItem::Tag(start));
        }
        if let Some(last) = frames.last_mut() {
            last.push(Point::zero(), FrameItem::Tag(end));
        }
    }

    Ok(Fragment::frames(frames))
}

fn generate_tags<T: NativeElement>(
    mut cell: Packed<T>,
    locator: &mut SplitLocator,
    engine: &mut Engine,
) -> (Tag, Tag) {
    let key = typst_utils::hash128(&cell);
    let loc = locator.next_location(engine.introspector, key);
    cell.set_location(loc);
    (Tag::Start(cell.pack()), Tag::End(loc, key))
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
    let grid = grid_to_cellgrid(elem, engine, styles)?;
    GridLayouter::new(&grid, regions, locator, styles, elem.span()).layout(engine)
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
    let grid = table_to_cellgrid(elem, engine, styles)?;
    GridLayouter::new(&grid, regions, locator, styles, elem.span()).layout(engine)
}
