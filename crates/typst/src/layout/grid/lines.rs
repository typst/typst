use std::{num::NonZeroUsize, sync::Arc};

use crate::foundations::Fold;
use crate::layout::{Abs, Axes};
use crate::visualize::Stroke;

use super::layout::CellGrid;

#[cfg(test)]
use super::layout::{Entry, RowPiece};

/// Represents an explicit grid line (horizontal or vertical) specified by the
/// user.
pub struct Line {
    /// The index of the track after this line. This will be the index of the
    /// row a horizontal line is above of, or of the column right after a
    /// vertical line.
    /// Must be within `0..=tracks.len()` (where `tracks` is either `grid.cols`
    /// or `grid.rows`, ignoring gutter tracks, as appropriate).
    pub index: usize,
    /// The index of the track at which this line starts being drawn.
    /// This is the first column a horizontal line appears in, or the first row
    /// a vertical line appears in.
    /// Must be within `0..tracks.len()` minus gutter tracks.
    pub start: usize,
    /// The index after the last track through which the line is drawn.
    /// Thus, the line is drawn through tracks `start..end` (note that `end` is
    /// exclusive).
    /// Must be within `1..=tracks.len()` minus gutter tracks.
    /// `None` indicates the line should go all the way to the end.
    pub end: Option<NonZeroUsize>,
    /// The line's stroke. This is `None` when the line is explicitly used to
    /// override a previously specified line.
    pub stroke: Option<Arc<Stroke<Abs>>>,
    /// The line's position in relation to the track with its index.
    pub position: LinePosition,
}

/// Indicates whether the line should be drawn before or after the track with
/// its index. This is mostly only relevant when gutter is used, since, then,
/// the position after a track is not the same as before the next
/// non-gutter track.
#[derive(PartialEq, Eq)]
pub enum LinePosition {
    /// The line should be drawn before its track (e.g. hline on top of a row).
    Before,
    /// The line should be drawn after its track (e.g. hline below a row).
    After,
}

/// Generates the segments of lines that should be drawn alongside a certain
/// axis in the grid, going through the given tracks (orthogonal to the lines).
/// Each returned segment contains its stroke, its offset from the start, and
/// its length.
/// Accepts, as parameters, the index of the lines that should be produced
/// (for example, the column at which vertical lines will be drawn); a list of
/// user-specified lines with the same index (the `lines` parameter); whether
/// the given index corresponds to the maximum index for the line's axis; and a
/// function which returns the final stroke that should be used for each track
/// the line goes through (its parameters are the grid, the index of the line
/// to be drawn, the number of the track to draw at and the stroke of the user
/// hline/vline override at this index to fold with, if any).
/// Contiguous segments with the same stroke are joined together automatically.
/// The function should return 'None' for positions at which the line would
/// otherwise cross a merged cell (for example, a vline could cross a colspan),
/// in which case a new segment should be drawn after the merged cell(s), even
/// if it would have the same stroke as the previous one.
/// Note that we assume that the tracks are sorted according to ascending
/// number, and they must be iterable over pairs of (number, size). For
/// vertical lines, for instance, 'tracks' would describe the rows in the
/// current region, as pairs (row index, row height).
pub(super) fn generate_line_segments<'grid, F, I>(
    grid: &'grid CellGrid,
    tracks: I,
    index: usize,
    lines: &'grid [Line],
    is_max_index: bool,
    line_stroke_at_track: F,
) -> impl IntoIterator<Item = (Arc<Stroke<Abs>>, Abs, Abs)> + 'grid
where
    F: Fn(&CellGrid, usize, usize, Option<Arc<Stroke<Abs>>>) -> Option<Arc<Stroke<Abs>>>
        + 'grid,
    I: IntoIterator<Item = (usize, Abs)>,
    I::IntoIter: 'grid,
{
    // The segment currently being drawn.
    // It is extended for each consecutive track through which the line would
    // be drawn with the same stroke.
    // Starts as None to force us to create a new segment as soon as we find
    // the first track through which we should draw.
    let mut current_segment: Option<(Arc<Stroke<Abs>>, Abs, Abs)> = None;
    // How far from the start (before the first track) have we gone so far.
    // Used to determine the positions at which to draw each segment.
    let mut offset = Abs::zero();
    // How much to multiply line indices by to account for gutter.
    let gutter_factor = if grid.has_gutter { 2 } else { 1 };
    // Which line position to look for in the given list of lines.
    // If the index represents a gutter track, this means the list of lines
    // parameter will actually correspond to the list of lines in the previous
    // index, so we must look for lines positioned after the previous index,
    // and not before, to determine which lines should be placed in gutter.
    // Note that the maximum index is always an odd number when there's gutter,
    // so we must check for it to ensure we don't give it the same treatment as
    // a line before a gutter track.
    let expected_line_position = if grid.has_gutter && index % 2 == 1 && !is_max_index {
        LinePosition::After
    } else {
        LinePosition::Before
    };

    // Create an iterator which will go through each track, from start to
    // finish, to create line segments and extend them until they are
    // interrupted. Each track will be mapped to the finished line segment
    // they interrupted; if they didn't interrupt any, they are filtered out.
    // When going through each track, we check if the current segment would be
    // interrupted, either because, at this track, we hit a merged cell over
    // which we shouldn't draw, or because the line would have a different
    // stroke at this point (so we have to start a new segment). If so, the
    // current segment is yielded and the variable is set to None (meaning we
    // have to create a new one later) or to the new segment (if we're starting
    // to draw a segment with a different stroke than before).
    // Otherwise (if the current segment should span the current track), it is
    // simply extended (or a new one is created, if it is 'None'), and no value
    // is yielded for the current track, since the segment isn't yet complete
    // (the next tracks might extend it further before it is interrupted and
    // yielded). That is, we yield each segment only when it is interrupted,
    // since then we will know its final length for sure.
    // We chain an extra 'None' track to ensure the final segment is always
    // interrupted and yielded, if it wasn't interrupted earlier.
    tracks.into_iter().map(Some).chain(std::iter::once(None)).filter_map(
        move |track_data| {
            if let Some((track, size)) = track_data {
                // Get the expected line stroke at this track by folding the
                // strokes of each user-specified line (with priority to the
                // user-specified line specified last).
                let stroke = lines
                    .iter()
                    .filter(|line| {
                        line.position == expected_line_position
                            && line
                                .end
                                .map(|end| {
                                    // Subtract 1 from end index so we stop at the last
                                    // cell before it (don't cross one extra gutter).
                                    let end = if grid.has_gutter {
                                        2 * end.get() - 1
                                    } else {
                                        end.get()
                                    };
                                    (gutter_factor * line.start..end).contains(&track)
                                })
                                .unwrap_or_else(|| track >= gutter_factor * line.start)
                    })
                    .map(|line| line.stroke.as_ref().cloned())
                    .fold(None, |acc, line_stroke| line_stroke.fold(acc));

                // The function shall determine if it is appropriate to draw
                // the line at this position or not (i.e. whether or not it
                // would cross a merged cell), and, if so, the final stroke it
                // should have (because cells near this position could have
                // stroke overrides, which have priority and should be folded
                // with the stroke obtained above).
                // The variable 'interrupted_segment' will contain the segment
                // to yield for this track, which will be the current segment
                // if it was interrupted, or 'None' (don't yield yet)
                // otherwise.
                let interrupted_segment = if let Some(stroke) =
                    line_stroke_at_track(grid, index, track, stroke)
                {
                    // We should draw at this position. Let's check if we were
                    // already drawing in the previous position.
                    if let Some(current_segment) = &mut current_segment {
                        // We are currently building a segment. Let's check if
                        // we should extend it to this track as well.
                        if current_segment.0 == stroke {
                            // Extend the current segment so it covers at least
                            // this track as well, since we should use the same
                            // stroke as in the previous one when a line goes
                            // through this track.
                            current_segment.2 += size;
                            // No need to yield the current segment, we might not
                            // be done extending its length yet.
                            None
                        } else {
                            // We got a different stroke now, so create a new
                            // segment with the new stroke and spanning the
                            // current track. Yield the old segment, as it was
                            // interrupted and is thus complete.
                            Some(std::mem::replace(
                                current_segment,
                                (stroke, offset, size),
                            ))
                        }
                    } else {
                        // We should draw here, but there is no segment
                        // currently being drawn, either because the last
                        // position had a merged cell, had a stroke
                        // of 'None', or because this is the first track.
                        // Create a new segment to draw. We start spanning this
                        // track.
                        current_segment = Some((stroke, offset, size));
                        // Nothing to yield for this track. The new segment
                        // might still be extended in the next track.
                        None
                    }
                } else {
                    // We shouldn't draw here (stroke of None), so we yield the
                    // current segment, as it was interrupted.
                    current_segment.take()
                };
                offset += size;
                interrupted_segment
            } else {
                // Reached the end of all tracks, so we interrupt and finish
                // the current segment.
                current_segment.take()
            }
        },
    )
}

/// Returns the correct stroke with which to draw a vline right before column
/// 'x' when going through row 'y', given the stroke of the user-specified line
/// at this position, if any.
/// If the vline would go through a colspan, returns None (shouldn't be drawn).
/// If the one (when at the border) or two (otherwise) cells to the left and
/// right of the vline have right and left stroke overrides, respectively,
/// then the cells' stroke overrides are folded together with the vline's
/// stroke (with priority to the vline's stroke, followed by the right cell's
/// stroke, and, finally, the left cell's) and returned. If, however, the cells
/// around the vline at this row do not have any stroke overrides, then the
/// vline's own stroke is directly returned.
pub(super) fn vline_stroke_at_row(
    grid: &CellGrid,
    x: usize,
    y: usize,
    stroke: Option<Arc<Stroke<Abs>>>,
) -> Option<Arc<Stroke<Abs>>> {
    if x != 0 && x != grid.cols.len() {
        // When the vline isn't at the border, we need to check if a colspan would
        // be present between columns 'x' and 'x-1' at row 'y', and thus overlap
        // with the line.
        // To do so, we analyze the cell right after this vline. If it is merged
        // with a cell before this line (parent_x < x) which is at this row or
        // above it (parent_y <= y), this means it would overlap with the vline,
        // so the vline must not be drawn at this row.
        let first_adjacent_cell = if grid.has_gutter {
            // Skip the gutters, if x or y represent gutter tracks.
            // We would then analyze the cell one column after (if at a gutter
            // column), and/or one row below (if at a gutter row), in order to
            // check if it would be merged with a cell before the vline.
            (x + x % 2, y + y % 2)
        } else {
            (x, y)
        };
        let Axes { x: parent_x, y: parent_y } = grid
            .parent_cell_position(first_adjacent_cell.0, first_adjacent_cell.1)
            .unwrap();

        if parent_x < x && parent_y <= y {
            // There is a colspan cell going through this vline's position,
            // so don't draw it here.
            return None;
        }
    }

    let left_cell_stroke = x
        .checked_sub(1)
        .and_then(|left_x| grid.parent_cell(left_x, y))
        .and_then(|left_cell| left_cell.stroke.right.as_ref());
    let right_cell_stroke = if x < grid.cols.len() {
        grid.parent_cell(x, y)
            .and_then(|right_cell| right_cell.stroke.left.as_ref())
    } else {
        None
    };

    let cell_stroke = match (left_cell_stroke.cloned(), right_cell_stroke.cloned()) {
        (Some(left_cell_stroke), Some(right_cell_stroke)) => {
            // When both cells specify a stroke for this line segment, fold
            // both strokes, with priority to the right cell's left stroke.
            Some(right_cell_stroke.fold(left_cell_stroke))
        }
        // When one of the cells doesn't specify a stroke, the other cell's
        // stroke should be used.
        (left_cell_stroke, right_cell_stroke) => left_cell_stroke.or(right_cell_stroke),
    };

    // Fold the line stroke and folded cell strokes, if possible.
    // Give priority to the explicit line stroke.
    // Otherwise, use whichever of the two isn't 'none' or unspecified.
    match (cell_stroke, stroke) {
        (Some(cell_stroke), Some(stroke)) => Some(stroke.fold(cell_stroke)),
        (cell_stroke, stroke) => cell_stroke.or(stroke),
    }
}

/// Returns the correct stroke with which to draw a hline on top of row 'y'
/// when going through column 'x', given the stroke of the user-specified line
/// at this position, if any.
/// If the one (when at the border) or two (otherwise) cells above and below
/// the hline have bottom and top stroke overrides, respectively, then the
/// cells' stroke overrides are folded together with the hline's stroke (with
/// priority to hline's stroke, followed by the bottom cell's stroke, and,
/// finally, the top cell's) and returned. If, however, the cells around the
/// hline at this column do not have any stroke overrides, then the hline's own
/// stroke is directly returned.
pub(super) fn hline_stroke_at_column(
    grid: &CellGrid,
    y: usize,
    x: usize,
    stroke: Option<Arc<Stroke<Abs>>>,
) -> Option<Arc<Stroke<Abs>>> {
    // There are no rowspans yet, so no need to add a check here. The line will
    // always be drawn, if it has a stroke.
    let cell_x = if grid.has_gutter {
        // Skip the gutter column this hline is in.
        // This is because positions above and below it, even if gutter, could
        // be part of a colspan, so we have to check the following cell.
        // However, this is only valid if we're not in a gutter row.
        x + x % 2
    } else {
        x
    };
    let top_cell_stroke = y
        .checked_sub(1)
        .and_then(|top_y| {
            // Let's find the parent cell of the position above us, in order
            // to take its bottom stroke, even when we're below gutter.
            grid.parent_cell_position(cell_x, top_y)
        })
        .filter(|Axes { x: parent_x, .. }| {
            // Only use the stroke of the cell above us but one column to the
            // right if it is merged with a cell before this line's column.
            // If the position above us is a simple non-merged cell, or the
            // parent of a colspan, this will also evaluate to true.
            parent_x <= &x
        })
        .and_then(|Axes { x: parent_x, y: parent_y }| {
            let top_cell = grid.cell(parent_x, parent_y).unwrap();
            top_cell.stroke.bottom.as_ref()
        });
    let bottom_cell_stroke = if y < grid.rows.len() {
        // Let's find the parent cell of the position below us, in order
        // to take its top stroke, even when we're above gutter.
        grid.parent_cell_position(cell_x, y)
            .filter(|Axes { x: parent_x, .. }| {
                // Only use the stroke of the cell below us but one column to the
                // right if it is merged with a cell before this line's column.
                // If the position below us is a simple non-merged cell, or the
                // parent of a colspan, this will also evaluate to true.
                parent_x <= &x
            })
            .and_then(|Axes { x: parent_x, y: parent_y }| {
                let bottom_cell = grid.cell(parent_x, parent_y).unwrap();
                bottom_cell.stroke.top.as_ref()
            })
    } else {
        // No cell below the bottom border.
        None
    };

    let cell_stroke = match (top_cell_stroke.cloned(), bottom_cell_stroke.cloned()) {
        (Some(top_cell_stroke), Some(bottom_cell_stroke)) => {
            // When both cells specify a stroke for this line segment, fold
            // both strokes, with priority to the bottom cell's top stroke.
            Some(bottom_cell_stroke.fold(top_cell_stroke))
        }
        // When one of the cells doesn't specify a stroke, the other cell's
        // stroke should be used.
        (top_cell_stroke, bottom_cell_stroke) => top_cell_stroke.or(bottom_cell_stroke),
    };

    // Fold the line stroke and folded cell strokes, if possible.
    // Give priority to the explicit line stroke.
    // Otherwise, use whichever of the two isn't 'none' or unspecified.
    match (cell_stroke, stroke) {
        (Some(cell_stroke), Some(stroke)) => Some(stroke.fold(cell_stroke)),
        (cell_stroke, stroke) => cell_stroke.or(stroke),
    }
}

#[cfg(test)]
mod test {
    use crate::foundations::Content;
    use crate::layout::{Cell, Sides, Sizing};
    use crate::util::NonZeroExt;

    use super::*;

    fn sample_cell() -> Cell {
        Cell {
            body: Content::default(),
            fill: None,
            colspan: NonZeroUsize::ONE,
            stroke: Sides::splat(Some(Arc::new(Stroke::default()))),
        }
    }

    fn cell_with_colspan(colspan: usize) -> Cell {
        Cell {
            body: Content::default(),
            fill: None,
            colspan: NonZeroUsize::try_from(colspan).unwrap(),
            stroke: Sides::splat(Some(Arc::new(Stroke::default()))),
        }
    }

    fn sample_grid(gutters: bool) -> CellGrid {
        const COLS: usize = 4;
        const ROWS: usize = 6;
        let entries = vec![
            // row 0
            Entry::Cell(sample_cell()),
            Entry::Cell(sample_cell()),
            Entry::Cell(cell_with_colspan(2)),
            Entry::Merged { parent: 2 },
            // row 1
            Entry::Cell(sample_cell()),
            Entry::Cell(cell_with_colspan(3)),
            Entry::Merged { parent: 5 },
            Entry::Merged { parent: 5 },
            // row 2
            Entry::Merged { parent: 4 },
            Entry::Cell(sample_cell()),
            Entry::Cell(cell_with_colspan(2)),
            Entry::Merged { parent: 10 },
            // row 3
            Entry::Cell(sample_cell()),
            Entry::Cell(cell_with_colspan(3)),
            Entry::Merged { parent: 13 },
            Entry::Merged { parent: 13 },
            // row 4
            Entry::Cell(sample_cell()),
            Entry::Merged { parent: 13 },
            Entry::Merged { parent: 13 },
            Entry::Merged { parent: 13 },
            // row 5
            Entry::Cell(sample_cell()),
            Entry::Cell(sample_cell()),
            Entry::Cell(cell_with_colspan(2)),
            Entry::Merged { parent: 22 },
        ];
        CellGrid::new_internal(
            Axes::with_x(&[Sizing::Auto; COLS]),
            if gutters {
                Axes::new(&[Sizing::Auto; COLS - 1], &[Sizing::Auto; ROWS - 1])
            } else {
                Axes::default()
            },
            vec![],
            vec![],
            entries,
        )
    }

    #[test]
    fn test_vline_splitting_without_gutter() {
        let stroke = Arc::new(Stroke::default());
        let grid = sample_grid(false);
        let rows = &[
            RowPiece { height: Abs::pt(1.0), y: 0 },
            RowPiece { height: Abs::pt(2.0), y: 1 },
            RowPiece { height: Abs::pt(4.0), y: 2 },
            RowPiece { height: Abs::pt(8.0), y: 3 },
            RowPiece { height: Abs::pt(16.0), y: 4 },
            RowPiece { height: Abs::pt(32.0), y: 5 },
        ];
        let expected_vline_splits = &[
            vec![(stroke.clone(), Abs::pt(0.), Abs::pt(1. + 2. + 4. + 8. + 16. + 32.))],
            vec![(stroke.clone(), Abs::pt(0.), Abs::pt(1. + 2. + 4. + 8. + 16. + 32.))],
            // interrupted a few times by colspans
            vec![
                (stroke.clone(), Abs::pt(0.), Abs::pt(1.)),
                (stroke.clone(), Abs::pt(1. + 2.), Abs::pt(4.)),
                (stroke.clone(), Abs::pt(1. + 2. + 4. + 8. + 16.), Abs::pt(32.)),
            ],
            // interrupted every time by colspans
            vec![],
            vec![(stroke, Abs::pt(0.), Abs::pt(1. + 2. + 4. + 8. + 16. + 32.))],
        ];
        for (x, expected_splits) in expected_vline_splits.iter().enumerate() {
            let tracks = rows.iter().map(|row| (row.y, row.height));
            assert_eq!(
                expected_splits,
                &generate_line_segments(
                    &grid,
                    tracks,
                    x,
                    &[],
                    x == grid.cols.len(),
                    vline_stroke_at_row
                )
                .into_iter()
                .collect::<Vec<_>>(),
            );
        }
    }

    #[test]
    fn test_vline_splitting_with_gutter_and_per_cell_stroke() {
        let stroke = Arc::new(Stroke::default());
        let grid = sample_grid(true);
        let rows = &[
            RowPiece { height: Abs::pt(1.0), y: 0 },
            RowPiece { height: Abs::pt(2.0), y: 1 },
            RowPiece { height: Abs::pt(4.0), y: 2 },
            RowPiece { height: Abs::pt(8.0), y: 3 },
            RowPiece { height: Abs::pt(16.0), y: 4 },
            RowPiece { height: Abs::pt(32.0), y: 5 },
            RowPiece { height: Abs::pt(64.0), y: 6 },
            RowPiece { height: Abs::pt(128.0), y: 7 },
            RowPiece { height: Abs::pt(256.0), y: 8 },
            RowPiece { height: Abs::pt(512.0), y: 9 },
            RowPiece { height: Abs::pt(1024.0), y: 10 },
        ];
        // Stroke is per-cell so we skip gutter
        let expected_vline_splits = &[
            // left border
            vec![
                (stroke.clone(), Abs::pt(0.), Abs::pt(1.)),
                (stroke.clone(), Abs::pt(1. + 2.), Abs::pt(4.)),
                (stroke.clone(), Abs::pt(1. + 2. + 4. + 8.), Abs::pt(16.)),
                (stroke.clone(), Abs::pt(1. + 2. + 4. + 8. + 16. + 32.), Abs::pt(64.)),
                (
                    stroke.clone(),
                    Abs::pt(1. + 2. + 4. + 8. + 16. + 32. + 64. + 128.),
                    Abs::pt(256.),
                ),
                (
                    stroke.clone(),
                    Abs::pt(1. + 2. + 4. + 8. + 16. + 32. + 64. + 128. + 256. + 512.),
                    Abs::pt(1024.),
                ),
            ],
            // gutter line below
            vec![
                (stroke.clone(), Abs::pt(0.), Abs::pt(1.)),
                (stroke.clone(), Abs::pt(1. + 2.), Abs::pt(4.)),
                (stroke.clone(), Abs::pt(1. + 2. + 4. + 8.), Abs::pt(16.)),
                (stroke.clone(), Abs::pt(1. + 2. + 4. + 8. + 16. + 32.), Abs::pt(64.)),
                (
                    stroke.clone(),
                    Abs::pt(1. + 2. + 4. + 8. + 16. + 32. + 64. + 128.),
                    Abs::pt(256.),
                ),
                (
                    stroke.clone(),
                    Abs::pt(1. + 2. + 4. + 8. + 16. + 32. + 64. + 128. + 256. + 512.),
                    Abs::pt(1024.),
                ),
            ],
            vec![
                (stroke.clone(), Abs::pt(0.), Abs::pt(1.)),
                (stroke.clone(), Abs::pt(1. + 2.), Abs::pt(4.)),
                (stroke.clone(), Abs::pt(1. + 2. + 4. + 8.), Abs::pt(16.)),
                (stroke.clone(), Abs::pt(1. + 2. + 4. + 8. + 16. + 32.), Abs::pt(64.)),
                (
                    stroke.clone(),
                    Abs::pt(1. + 2. + 4. + 8. + 16. + 32. + 64. + 128.),
                    Abs::pt(256.),
                ),
                (
                    stroke.clone(),
                    Abs::pt(1. + 2. + 4. + 8. + 16. + 32. + 64. + 128. + 256. + 512.),
                    Abs::pt(1024.),
                ),
            ],
            // gutter line below
            // the two lines below are interrupted multiple times by colspans
            vec![
                (stroke.clone(), Abs::pt(0.), Abs::pt(1.)),
                (stroke.clone(), Abs::pt(1. + 2. + 4. + 8.), Abs::pt(16.)),
                (
                    stroke.clone(),
                    Abs::pt(1. + 2. + 4. + 8. + 16. + 32. + 64. + 128. + 256. + 512.),
                    Abs::pt(1024.),
                ),
            ],
            vec![
                (stroke.clone(), Abs::pt(0.), Abs::pt(1.)),
                (stroke.clone(), Abs::pt(1. + 2. + 4. + 8.), Abs::pt(16.)),
                (
                    stroke.clone(),
                    Abs::pt(1. + 2. + 4. + 8. + 16. + 32. + 64. + 128. + 256. + 512.),
                    Abs::pt(1024.),
                ),
            ],
            // gutter line below
            // the two lines below can only cross certain gutter rows, because
            // all non-gutter cells in the following column are merged with
            // cells from the previous column.
            vec![],
            vec![],
            // right border
            vec![
                (stroke.clone(), Abs::pt(0.), Abs::pt(1.)),
                (stroke.clone(), Abs::pt(1. + 2.), Abs::pt(4.)),
                (stroke.clone(), Abs::pt(1. + 2. + 4. + 8.), Abs::pt(16.)),
                (stroke.clone(), Abs::pt(1. + 2. + 4. + 8. + 16. + 32.), Abs::pt(64.)),
                (
                    stroke.clone(),
                    Abs::pt(1. + 2. + 4. + 8. + 16. + 32. + 64. + 128.),
                    Abs::pt(256.),
                ),
                (
                    stroke.clone(),
                    Abs::pt(1. + 2. + 4. + 8. + 16. + 32. + 64. + 128. + 256. + 512.),
                    Abs::pt(1024.),
                ),
            ],
        ];
        for (x, expected_splits) in expected_vline_splits.iter().enumerate() {
            let tracks = rows.iter().map(|row| (row.y, row.height));
            assert_eq!(
                expected_splits,
                &generate_line_segments(
                    &grid,
                    tracks,
                    x,
                    &[],
                    x == grid.cols.len(),
                    vline_stroke_at_row
                )
                .into_iter()
                .collect::<Vec<_>>(),
            );
        }
    }

    #[test]
    fn test_vline_splitting_with_gutter_and_explicit_vlines() {
        let stroke = Arc::new(Stroke::default());
        let grid = sample_grid(true);
        let rows = &[
            RowPiece { height: Abs::pt(1.0), y: 0 },
            RowPiece { height: Abs::pt(2.0), y: 1 },
            RowPiece { height: Abs::pt(4.0), y: 2 },
            RowPiece { height: Abs::pt(8.0), y: 3 },
            RowPiece { height: Abs::pt(16.0), y: 4 },
            RowPiece { height: Abs::pt(32.0), y: 5 },
            RowPiece { height: Abs::pt(64.0), y: 6 },
            RowPiece { height: Abs::pt(128.0), y: 7 },
            RowPiece { height: Abs::pt(256.0), y: 8 },
            RowPiece { height: Abs::pt(512.0), y: 9 },
            RowPiece { height: Abs::pt(1024.0), y: 10 },
        ];
        let expected_vline_splits = &[
            // left border
            vec![(
                stroke.clone(),
                Abs::pt(0.),
                Abs::pt(1. + 2. + 4. + 8. + 16. + 32. + 64. + 128. + 256. + 512. + 1024.),
            )],
            // gutter line below
            vec![(
                stroke.clone(),
                Abs::pt(0.),
                Abs::pt(1. + 2. + 4. + 8. + 16. + 32. + 64. + 128. + 256. + 512. + 1024.),
            )],
            vec![(
                stroke.clone(),
                Abs::pt(0.),
                Abs::pt(1. + 2. + 4. + 8. + 16. + 32. + 64. + 128. + 256. + 512. + 1024.),
            )],
            // gutter line below
            // the two lines below are interrupted multiple times by colspans
            vec![
                (stroke.clone(), Abs::pt(0.), Abs::pt(1. + 2.)),
                (stroke.clone(), Abs::pt(1. + 2. + 4.), Abs::pt(8. + 16. + 32.)),
                (
                    stroke.clone(),
                    Abs::pt(1. + 2. + 4. + 8. + 16. + 32. + 64. + 128. + 256.),
                    Abs::pt(512. + 1024.),
                ),
            ],
            vec![
                (stroke.clone(), Abs::pt(0.), Abs::pt(1. + 2.)),
                (stroke.clone(), Abs::pt(1. + 2. + 4.), Abs::pt(8. + 16. + 32.)),
                (
                    stroke.clone(),
                    Abs::pt(1. + 2. + 4. + 8. + 16. + 32. + 64. + 128. + 256.),
                    Abs::pt(512. + 1024.),
                ),
            ],
            // gutter line below
            // the two lines below can only cross certain gutter rows, because
            // all non-gutter cells in the following column are merged with
            // cells from the previous column.
            vec![
                (stroke.clone(), Abs::pt(1.), Abs::pt(2.)),
                (stroke.clone(), Abs::pt(1. + 2. + 4.), Abs::pt(8.)),
                (stroke.clone(), Abs::pt(1. + 2. + 4. + 8. + 16.), Abs::pt(32.)),
                (
                    stroke.clone(),
                    Abs::pt(1. + 2. + 4. + 8. + 16. + 32. + 64. + 128. + 256.),
                    Abs::pt(512.),
                ),
            ],
            vec![
                (stroke.clone(), Abs::pt(1.), Abs::pt(2.)),
                (stroke.clone(), Abs::pt(1. + 2. + 4.), Abs::pt(8.)),
                (stroke.clone(), Abs::pt(1. + 2. + 4. + 8. + 16.), Abs::pt(32.)),
                (
                    stroke.clone(),
                    Abs::pt(1. + 2. + 4. + 8. + 16. + 32. + 64. + 128. + 256.),
                    Abs::pt(512.),
                ),
            ],
            // right border
            vec![(
                stroke.clone(),
                Abs::pt(0.),
                Abs::pt(1. + 2. + 4. + 8. + 16. + 32. + 64. + 128. + 256. + 512. + 1024.),
            )],
        ];
        for (x, expected_splits) in expected_vline_splits.iter().enumerate() {
            let tracks = rows.iter().map(|row| (row.y, row.height));
            assert_eq!(
                expected_splits,
                &generate_line_segments(
                    &grid,
                    tracks,
                    x,
                    &[
                        Line {
                            index: x,
                            start: 0,
                            end: None,
                            stroke: Some(stroke.clone()),
                            position: LinePosition::Before
                        },
                        Line {
                            index: x,
                            start: 0,
                            end: None,
                            stroke: Some(stroke.clone()),
                            position: LinePosition::After
                        },
                    ],
                    x == grid.cols.len(),
                    vline_stroke_at_row
                )
                .into_iter()
                .collect::<Vec<_>>(),
            );
        }
    }
}
