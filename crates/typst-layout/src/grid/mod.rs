mod layouter;
mod lines;
mod repeated;
mod rowspans;

pub use self::layouter::GridLayouter;

use typst_library::diag::SourceResult;
use typst_library::engine::Engine;
use typst_library::foundations::{Content, NativeElement, Packed, Smart, StyleChain};
use typst_library::introspection::{Location, Locator, SplitLocator, Tag, TagFlags};
use typst_library::layout::grid::resolve::{Cell, CellSource};
use std::sync::Arc;
use typst_library::layout::{
    Fragment, Frame, FrameItem, FrameParent, GridCell, GridElem, Inherit,
    Point, Regions,
};
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
    let mut tags = None;

    // Generate tags using Cell.source metadata to avoid cloning the full
    // Packed<TableCell/GridCell> from cell.body. We build a lightweight
    // packed cell with only the fields needed for PDF tagging.
    match &cell.source {
        Some(CellSource::Table { cell_x, cell_y, kind }) => {
            let tag_cell = TableCell::new(Content::default())
                .with_x(Smart::Custom(*cell_x))
                .with_y(Smart::Custom(*cell_y))
                .with_colspan(cell.colspan)
                .with_rowspan(cell.rowspan)
                .with_kind(*kind);
            let mut packed = Packed::new(tag_cell).spanned(cell.source_span);
            if is_repeated {
                packed.is_repeated.set(is_repeated);
            }
            tags = Some(generate_tags(packed, &mut locator, engine));
        }
        Some(CellSource::Grid { cell_x, cell_y }) => {
            let tag_cell = GridCell::new(Content::default())
                .with_x(Smart::Custom(*cell_x))
                .with_y(Smart::Custom(*cell_y))
                .with_colspan(cell.colspan)
                .with_rowspan(cell.rowspan);
            let mut packed = Packed::new(tag_cell).spanned(cell.source_span);
            if is_repeated {
                packed.is_repeated.set(is_repeated);
            }
            tags = Some(generate_tags(packed, &mut locator, engine));
        }
        None => {}
    }

    let locator = locator.next(&cell.body.span());
    let fragment = crate::layout_fragment(engine, &cell.body, locator, styles, regions)?;

    // Manually insert tags.
    let mut frames = fragment.into_frames();
    if let Some((elem, loc, key)) = tags
        && let Some((first, remainder)) = frames.split_first_mut()
    {
        let flags = TagFlags { introspectable: true, tagged: true };
        if remainder.is_empty() {
            // Optimization: instead of prepend/push on the existing frame
            // (which triggers Arc::make_mut deep clone when refcount > 1),
            // create a wrapper frame with tags + the original as a group.
            // This avoids cloning the potentially large items Vec.
            if Arc::strong_count(first.items_arc()) > 1 {
                // Wrap the original frame as a Group to avoid deep-cloning
                // the items Vec. Use push(Group) directly instead of
                // push_frame which might inline (triggering clone).
                let size = first.size();
                let kind = first.kind();
                let original = std::mem::replace(first, Frame::new(size, kind));
                first.push(Point::zero(), FrameItem::Tag(Tag::Start(elem, loc, flags)));
                first.push(Point::zero(), FrameItem::Group(
                    typst_library::layout::GroupItem::new(original)
                ));
                first.push(Point::zero(), FrameItem::Tag(Tag::End(loc, key, flags)));
            } else {
                first.prepend(Point::zero(), FrameItem::Tag(Tag::Start(elem, loc, flags)));
                first.push(Point::zero(), FrameItem::Tag(Tag::End(loc, key, flags)));
            }
        } else {
            for frame in frames.iter_mut() {
                frame.set_parent(FrameParent::new(loc, Inherit::Yes));
            }
            frames.first_mut().unwrap().prepend_multiple([
                (Point::zero(), FrameItem::Tag(Tag::Start(elem, loc, flags))),
                (Point::zero(), FrameItem::Tag(Tag::End(loc, key, flags))),
            ]);
        }
    }

    Ok(Fragment::frames(frames))
}

fn generate_tags<T: NativeElement>(
    cell: Packed<T>,
    locator: &mut SplitLocator,
    engine: &mut Engine,
) -> (Content, Location, u128) {
    let key = typst_utils::hash128(&cell);
    let loc = locator.next_location(engine, key, cell.span());
    // Location is stored on the Tag, not on the Content.
    // This avoids triggering make_unique (deep clone).
    (cell.pack(), loc, key)
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
