mod layouter;
mod lines;
mod repeated;
mod rowspans;

pub use self::layouter::GridLayouter;

use typst_library::diag::{At, SourceResult, assert_internal, internal_error};
use typst_library::engine::Engine;
use typst_library::foundations::{NativeElement, Packed, StyleChain};
use typst_library::introspection::{Locatable, Locator, Tag, Tagged};
use typst_library::layout::grid::resolve::Cell;
use typst_library::layout::{
    Fragment, FrameItem, FrameParent, GridCell, GridElem, Inherit, Regions,
};
use typst_library::model::{TableCell, TableElem};
use typst_utils::display;

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
    let body = if let Some(table_cell) = cell.body.to_packed::<TableCell>() {
        table_cell.clone().unpack().with_is_repeated(is_repeated).pack()
    } else if let Some(grid_cell) = cell.body.to_packed::<GridCell>() {
        grid_cell.clone().unpack().with_is_repeated(is_repeated).pack()
    } else {
        cell.body.clone()
    };

    let fragment = crate::layout_fragment(engine, &body, locator, styles, regions)?;

    // The cell is only laid-out into one region, no further action is needed.
    let mut frames = fragment.into_frames();
    if frames.len() == 1 {
        return Ok(Fragment::frames(frames));
    }

    let first_non_empty = frames.iter().position(|f| !f.is_empty());
    let last_non_empty = frames.iter().rposition(|f| !f.is_empty());
    let Some((first_idx, last_idx)) = first_non_empty.zip(last_non_empty) else {
        return Ok(Fragment::frames(frames));
    };

    // All elements directly passed into grid layout should be `Locatable`,
    // `Tagged`, or empty content. Empty content will always produce empty
    // frames, which is handled above. So this element *should* be `Locatable`
    // or `Tagged` and will generate introspection tags to ensure the logical
    // parenting mechanism can be used to associate parts of the laid-out
    // element with the first frame.
    // Currently the only ones directly used are: `GridCell`, `TableCell`,
    // `ListItemLabel`, `ListItemBody`.
    assert_internal(
        body.can::<dyn Locatable>() || body.can::<dyn Tagged>(),
        display!("cell body is not locatable or tagged: {body:?}"),
    )
    .at(body.span())?;

    // Extract the start and end tag of the element.
    let start = frames[first_idx].remove(0);
    let FrameItem::Tag(start_tag @ Tag::Start(..)) = &start.1 else {
        return Err(internal_error(display!("expected start tag, found {:?}", start.1)))
            .at(body.span())?;
    };
    let last = &mut frames[last_idx];
    let end = last.remove(last.items().len() - 1);
    let FrameItem::Tag(end_tag @ Tag::End(..)) = &end.1 else {
        return Err(internal_error(display!("expected end tag, found {:?}", start.1)))
            .at(body.span())?;
    };
    assert_internal(
        start_tag.location() == end_tag.location(),
        display!(
            "start ({:?}) and end ({:?}) tags don't match",
            start_tag.location(),
            end_tag.location()
        ),
    )
    .at(body.span())?;

    // Set the logical parent of all frames to the cell, which converts them
    // to group frames. Then prepend the start and end tags containing no
    // content. The first frame is also a logical child to guarantee correct
    // ordering in the introspector, since logical children are currently
    // inserted immediately after the start tag of the parent element
    // preceding any content within the parent element's tags.
    for frame in frames[first_idx..=last_idx].iter_mut() {
        frame.set_parent(FrameParent::new(start_tag.location(), Inherit::Yes));
    }
    frames[0].prepend_multiple([start, end]);

    Ok(Fragment::frames(frames))
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
    let grid = elem.grid.as_ref().unwrap();
    GridLayouter::new(grid, regions, locator, styles, elem.span()).layout(engine)
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
    let grid = elem.grid.as_ref().unwrap();
    GridLayouter::new(grid, regions, locator, styles, elem.span()).layout(engine)
}
