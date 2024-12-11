use std::num::NonZeroUsize;
use std::sync::Arc;

use typst_library::layout::Abs;
use typst_library::visualize::Stroke;

use super::{CellGrid, LinePosition, Repeatable};

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
