use std::fmt::Debug;

use typst_library::diag::{bail, SourceResult};
use typst_library::engine::Engine;
use typst_library::foundations::{Resolve, StyleChain};
use typst_library::layout::grid::resolve::{
    Cell, CellGrid, Header, LinePosition, Repeatable,
};
use typst_library::layout::{
    Abs, Axes, Dir, Fr, Fragment, Frame, FrameItem, Length, Point, Region, Regions, Rel,
    Size, Sizing,
};
use typst_library::text::TextElem;
use typst_library::visualize::Geometry;
use typst_syntax::Span;
use typst_utils::Numeric;

use super::{
    generate_line_segments, hline_stroke_at_column, layout_cell, vline_stroke_at_row,
    LineSegment, Rowspan, UnbreakableRowGroup,
};

/// Performs grid layout.
pub struct GridLayouter<'a> {
    /// The grid of cells.
    pub(super) grid: &'a CellGrid<'a>,
    /// The regions to layout children into.
    pub(super) regions: Regions<'a>,
    /// The inherited styles.
    pub(super) styles: StyleChain<'a>,
    /// Resolved column sizes.
    pub(super) rcols: Vec<Abs>,
    /// The sum of `rcols`.
    pub(super) width: Abs,
    /// Resolved row sizes, by region.
    pub(super) rrows: Vec<Vec<RowPiece>>,
    /// The amount of unbreakable rows remaining to be laid out in the
    /// current unbreakable row group. While this is positive, no region breaks
    /// should occur.
    pub(super) unbreakable_rows_left: usize,
    /// Rowspans not yet laid out because not all of their spanned rows were
    /// laid out yet.
    pub(super) rowspans: Vec<Rowspan>,
    /// Grid layout state for the current region.
    pub(super) current: Current,
    /// Frames for finished regions.
    pub(super) finished: Vec<Frame>,
    /// The amount and height of header rows on each finished region.
    pub(super) finished_header_rows: Vec<FinishedHeaderRowInfo>,
    /// Whether this is an RTL grid.
    pub(super) is_rtl: bool,
    /// Currently repeating headers, one per level. Sorted by increasing
    /// levels.
    ///
    /// Note that some levels may be absent, in particular level 0, which does
    /// not exist (so all levels are >= 1).
    pub(super) repeating_headers: Vec<&'a Header>,
    /// Headers, repeating or not, awaiting their first successful layout.
    /// Sorted by increasing levels.
    pub(super) pending_headers: &'a [Repeatable<Header>],
    /// Next headers to be processed.
    pub(super) upcoming_headers: &'a [Repeatable<Header>],
    /// State of the row being currently laid out.
    ///
    /// This is kept as a field to avoid passing down too many parameters from
    /// `layout_row` into called functions, which would then have to pass them
    /// down to `push_row`, which reads these values.
    pub(super) row_state: RowState,
    /// The span of the grid element.
    pub(super) span: Span,
}

/// Grid layout state for the current region. This should be reset or updated
/// on each region break.
pub(super) struct Current {
    /// The initial size of the current region before we started subtracting.
    pub(super) initial: Size,
    /// The height of the region after repeated headers were placed and footers
    /// prepared. This also includes pending repeating headers from the start,
    /// even if they were not repeated yet, since they will be repeated in the
    /// next region anyway (bar orphan prevention).
    ///
    /// This is used to quickly tell if any additional space in the region has
    /// been occupied since then, meaning that additional space will become
    /// available after a region break (see
    /// [`GridLayouter::may_progress_with_repeats`]).
    pub(super) initial_after_repeats: Abs,
    /// Whether `layouter.regions.may_progress()` was `true` at the top of the
    /// region.
    pub(super) could_progress_at_top: bool,
    /// Rows in the current region.
    pub(super) lrows: Vec<Row>,
    /// The amount of repeated header rows at the start of the current region.
    /// Thus, excludes rows from pending headers (which were placed for the
    /// first time).
    ///
    /// Note that `repeating_headers` and `pending_headers` can change if we
    /// find a new header inside the region (not at the top), so this field
    /// is required to access information from the top of the region.
    ///
    /// This information is used on finish region to calculate the total height
    /// of resolved header rows at the top of the region, which is used by
    /// multi-page rowspans so they can properly skip the header rows at the
    /// top of each region during layout.
    pub(super) repeated_header_rows: usize,
    /// The end bound of the row range of the last repeating header at the
    /// start of the region.
    ///
    /// The last row might have disappeared from layout due to being empty, so
    /// this is how we can become aware of where the last header ends without
    /// having to check the vector of rows. Line layout uses this to determine
    /// when to prioritize the last lines under a header.
    ///
    /// A value of zero indicates no repeated headers were placed.
    pub(super) last_repeated_header_end: usize,
    /// Stores the length of `lrows` before a sequence of rows equipped with
    /// orphan prevention was laid out. In this case, if no more rows without
    /// orphan prevention are laid out after those rows before the region ends,
    /// the rows will be removed, and there may be an attempt to place them
    /// again in the new region. Effectively, this is the mechanism used for
    /// orphan prevention of rows.
    ///
    /// At the moment, this is only used by repeated headers (they aren't laid
    /// out if alone in the region) and by new headers, which are moved to the
    /// `pending_headers` vector and so will automatically be placed again
    /// until they fit and are not orphans in at least one region (or exactly
    /// one, for non-repeated headers).
    pub(super) lrows_orphan_snapshot: Option<usize>,
    /// The height of effectively repeating headers, that is, ignoring
    /// non-repeating pending headers, in the current region.
    ///
    /// This is used by multi-page auto rows so they can inform cell layout on
    /// how much space should be taken by headers if they break across regions.
    /// In particular, non-repeating headers only occupy the initial region,
    /// but disappear on new regions, so they can be ignored.
    ///
    /// This field is reset on each new region and properly updated by
    /// `layout_auto_row` and `layout_relative_row`, and should not be read
    /// before all header rows are fully laid out. It is usually fine because
    /// header rows themselves are unbreakable, and unbreakable rows do not
    /// need to read this field at all.
    ///
    /// This height is not only computed at the beginning of the region. It is
    /// updated whenever a new header is found, subtracting the height of
    /// headers which stopped repeating and adding the height of all new
    /// headers.
    pub(super) repeating_header_height: Abs,
    /// The height for each repeating header that was placed in this region.
    /// Note that this includes headers not at the top of the region, before
    /// their first repetition (pending headers), and excludes headers removed
    /// by virtue of a new, conflicting header being found (short-lived
    /// headers).
    ///
    /// This is used to know how much to update `repeating_header_height` by
    /// when finding a new header and causing existing repeating headers to
    /// stop.
    pub(super) repeating_header_heights: Vec<Abs>,
    /// The simulated footer height for this region.
    ///
    /// The simulation occurs before any rows are laid out for a region.
    pub(super) footer_height: Abs,
}

/// Data about the row being laid out right now.
#[derive(Debug, Default)]
pub(super) struct RowState {
    /// If this is `Some`, this will be updated by the currently laid out row's
    /// height if it is auto or relative. This is used for header height
    /// calculation.
    pub(super) current_row_height: Option<Abs>,
    /// This is `true` when laying out non-short lived headers and footers.
    /// That is, headers and footers which are not immediately followed or
    /// preceded (respectively) by conflicting headers and footers of same or
    /// lower level, or the end or start of the table (respectively), which
    /// would cause them to never repeat, even once.
    ///
    /// If this is `false`, the next row to be laid out will remove an active
    /// orphan snapshot and will flush pending headers, as there is no risk
    /// that they will be orphans anymore.
    pub(super) in_active_repeatable: bool,
}

/// Data about laid out repeated header rows for a specific finished region.
#[derive(Debug, Default)]
pub(super) struct FinishedHeaderRowInfo {
    /// The amount of repeated headers at the top of the region.
    pub(super) repeated_amount: usize,
    /// The end bound of the row range of the last repeated header at the top
    /// of the region.
    pub(super) last_repeated_header_end: usize,
    /// The total height of repeated headers at the top of the region.
    pub(super) repeated_height: Abs,
}

/// Details about a resulting row piece.
#[derive(Debug)]
pub struct RowPiece {
    /// The height of the segment.
    pub height: Abs,
    /// The index of the row.
    pub y: usize,
}

/// Produced by initial row layout, auto and relative rows are already finished,
/// fractional rows not yet.
pub(super) enum Row {
    /// Finished row frame of auto or relative row with y index.
    /// The last parameter indicates whether or not this is the last region
    /// where this row is laid out, and it can only be false when a row uses
    /// `layout_multi_row`, which in turn is only used by breakable auto rows.
    Frame(Frame, usize, bool),
    /// Fractional row with y index and disambiguator.
    Fr(Fr, usize, usize),
}

impl Row {
    /// Returns the `y` index of this row.
    fn index(&self) -> usize {
        match self {
            Self::Frame(_, y, _) => *y,
            Self::Fr(_, y, _) => *y,
        }
    }
}

impl<'a> GridLayouter<'a> {
    /// Create a new grid layouter.
    ///
    /// This prepares grid layout by unifying content and gutter tracks.
    pub fn new(
        grid: &'a CellGrid<'a>,
        regions: Regions<'a>,
        styles: StyleChain<'a>,
        span: Span,
    ) -> Self {
        // We use these regions for auto row measurement. Since at that moment,
        // columns are already sized, we can enable horizontal expansion.
        let mut regions = regions;
        regions.expand = Axes::new(true, false);

        Self {
            grid,
            regions,
            styles,
            rcols: vec![Abs::zero(); grid.cols.len()],
            width: Abs::zero(),
            rrows: vec![],
            unbreakable_rows_left: 0,
            rowspans: vec![],
            finished: vec![],
            finished_header_rows: vec![],
            is_rtl: TextElem::dir_in(styles) == Dir::RTL,
            repeating_headers: vec![],
            upcoming_headers: &grid.headers,
            pending_headers: Default::default(),
            row_state: RowState::default(),
            current: Current {
                initial: regions.size,
                initial_after_repeats: regions.size.y,
                could_progress_at_top: regions.may_progress(),
                lrows: vec![],
                repeated_header_rows: 0,
                last_repeated_header_end: 0,
                lrows_orphan_snapshot: None,
                repeating_header_height: Abs::zero(),
                repeating_header_heights: vec![],
                footer_height: Abs::zero(),
            },
            span,
        }
    }

    /// Determines the columns sizes and then layouts the grid row-by-row.
    pub fn layout(mut self, engine: &mut Engine) -> SourceResult<Fragment> {
        self.measure_columns(engine)?;

        if let Some(footer) = self.grid.footer.as_ref().and_then(Repeatable::as_repeated)
        {
            // Ensure rows in the first region will be aware of the possible
            // presence of the footer.
            self.prepare_footer(footer, engine, 0)?;
            self.regions.size.y -= self.current.footer_height;
            self.current.initial_after_repeats = self.regions.size.y;
        }

        let mut y = 0;
        let mut consecutive_header_count = 0;
        while y < self.grid.rows.len() {
            if let Some(next_header) = self.upcoming_headers.get(consecutive_header_count)
            {
                if next_header.range.contains(&y) {
                    self.place_new_headers(&mut consecutive_header_count, engine)?;
                    y = next_header.range.end;

                    // Skip header rows during normal layout.
                    continue;
                }
            }

            if let Some(footer) =
                self.grid.footer.as_ref().and_then(Repeatable::as_repeated)
            {
                if y >= footer.start {
                    if y == footer.start {
                        self.layout_footer(footer, engine, self.finished.len())?;
                        self.flush_orphans();
                    }
                    y = footer.end;
                    continue;
                }
            }

            self.layout_row(y, engine, 0)?;

            // After the first non-header row is placed, pending headers are no
            // longer orphans and can repeat, so we move them to repeating
            // headers.
            //
            // Note that this is usually done in `push_row`, since the call to
            // `layout_row` above might trigger region breaks (for multi-page
            // auto rows), whereas this needs to be called as soon as any part
            // of a row is laid out. However, it's possible a row has no
            // visible output and thus does not push any rows even though it
            // was successfully laid out, in which case we additionally flush
            // here just in case.
            self.flush_orphans();

            y += 1;
        }

        self.finish_region(engine, true)?;

        // Layout any missing rowspans.
        // There are only two possibilities for rowspans not yet laid out
        // (usually, a rowspan is laid out as soon as its last row, or any row
        // after it, is laid out):
        // 1. The rowspan was fully empty and only spanned fully empty auto
        // rows, which were all prevented from being laid out. Those rowspans
        // are ignored by 'layout_rowspan', and are not of any concern.
        //
        // 2. The rowspan's last row was an auto row at the last region which
        // was not laid out, and no other rows were laid out after it. Those
        // might still need to be laid out, so we check for them.
        for rowspan in std::mem::take(&mut self.rowspans) {
            self.layout_rowspan(rowspan, None, engine)?;
        }

        self.render_fills_strokes()
    }

    /// Layout a row with a certain initial state, returning the final state.
    #[inline]
    pub(super) fn layout_row_with_state(
        &mut self,
        y: usize,
        engine: &mut Engine,
        disambiguator: usize,
        initial_state: RowState,
    ) -> SourceResult<RowState> {
        // Keep a copy of the previous value in the stack, as this function can
        // call itself recursively (e.g. if a region break is triggered and a
        // header is placed), so we shouldn't outright overwrite it, but rather
        // save and later restore the state when back to this call.
        let previous = std::mem::replace(&mut self.row_state, initial_state);

        // Keep it as a separate function to allow inlining the return below,
        // as it's usually not needed.
        self.layout_row_internal(y, engine, disambiguator)?;

        Ok(std::mem::replace(&mut self.row_state, previous))
    }

    /// Layout the given row with the default row state.
    #[inline]
    pub(super) fn layout_row(
        &mut self,
        y: usize,
        engine: &mut Engine,
        disambiguator: usize,
    ) -> SourceResult<()> {
        self.layout_row_with_state(y, engine, disambiguator, RowState::default())?;
        Ok(())
    }

    /// Layout the given row using the current state.
    pub(super) fn layout_row_internal(
        &mut self,
        y: usize,
        engine: &mut Engine,
        disambiguator: usize,
    ) -> SourceResult<()> {
        // Skip to next region if current one is full, but only for content
        // rows, not for gutter rows, and only if we aren't laying out an
        // unbreakable group of rows.
        let is_content_row = !self.grid.is_gutter_track(y);
        if self.unbreakable_rows_left == 0 && self.regions.is_full() && is_content_row {
            self.finish_region(engine, false)?;
        }

        if is_content_row {
            // Gutter rows have no rowspans or possibly unbreakable cells.
            self.check_for_rowspans(disambiguator, y);
            self.check_for_unbreakable_rows(y, engine)?;
        }

        // Don't layout gutter rows at the top of a region.
        if is_content_row || !self.current.lrows.is_empty() {
            match self.grid.rows[y] {
                Sizing::Auto => self.layout_auto_row(engine, disambiguator, y)?,
                Sizing::Rel(v) => {
                    self.layout_relative_row(engine, disambiguator, v, y)?
                }
                Sizing::Fr(v) => {
                    if !self.row_state.in_active_repeatable {
                        self.flush_orphans();
                    }
                    self.current.lrows.push(Row::Fr(v, y, disambiguator))
                }
            }
        }

        self.unbreakable_rows_left = self.unbreakable_rows_left.saturating_sub(1);

        Ok(())
    }

    /// Add lines and backgrounds.
    fn render_fills_strokes(mut self) -> SourceResult<Fragment> {
        let mut finished = std::mem::take(&mut self.finished);
        let frame_amount = finished.len();
        for (((frame_index, frame), rows), finished_header_rows) in
            finished.iter_mut().enumerate().zip(&self.rrows).zip(
                self.finished_header_rows
                    .iter()
                    .map(Some)
                    .chain(std::iter::repeat(None)),
            )
        {
            if self.rcols.is_empty() || rows.is_empty() {
                continue;
            }

            // Render grid lines.
            // We collect lines into a vector before rendering so we can sort
            // them based on thickness, such that the lines with largest
            // thickness are drawn on top; and also so we can prepend all of
            // them at once in the frame, as calling prepend() for each line,
            // and thus pushing all frame items forward each time, would result
            // in quadratic complexity.
            let mut lines = vec![];

            // Which line position to look for in the list of lines for a
            // track, such that placing lines with those positions will
            // correspond to placing them before the given track index.
            //
            // If the index represents a gutter track, this means the list of
            // lines will actually correspond to the list of lines in the
            // previous index, so we must look for lines positioned after the
            // previous index, and not before, to determine which lines should
            // be placed before gutter.
            //
            // Note that the maximum index is always an odd number when
            // there's gutter, so we must check for it to ensure we don't give
            // it the same treatment as a line before a gutter track.
            let expected_line_position = |index, is_max_index: bool| {
                if self.grid.is_gutter_track(index) && !is_max_index {
                    LinePosition::After
                } else {
                    LinePosition::Before
                }
            };

            // Render vertical lines.
            // Render them first so horizontal lines have priority later.
            for (x, dx) in points(self.rcols.iter().copied()).enumerate() {
                let dx = if self.is_rtl { self.width - dx } else { dx };
                let is_end_border = x == self.grid.cols.len();
                let expected_vline_position = expected_line_position(x, is_end_border);

                let vlines_at_column = self
                    .grid
                    .vlines
                    .get(if !self.grid.has_gutter {
                        x
                    } else if is_end_border {
                        // The end border has its own vector of lines, but
                        // dividing it by 2 and flooring would give us the
                        // vector of lines with the index of the last column.
                        // Add 1 so we get the border's lines.
                        x / 2 + 1
                    } else {
                        // If x is a gutter column, this will round down to the
                        // index of the previous content column, which is
                        // intentional - the only lines which can appear before
                        // a gutter column are lines for the previous column
                        // marked with "LinePosition::After". Therefore, we get
                        // the previous column's lines. Worry not, as
                        // 'generate_line_segments' will correctly filter lines
                        // based on their LinePosition for us.
                        //
                        // If x is a content column, this will correctly return
                        // its index before applying gutters, so nothing
                        // special here (lines with "LinePosition::After" would
                        // then be ignored for this column, as we are drawing
                        // lines before it, not after).
                        x / 2
                    })
                    .into_iter()
                    .flatten()
                    .filter(|line| line.position == expected_vline_position);

                let tracks = rows.iter().map(|row| (row.y, row.height));

                // Determine all different line segments we have to draw in
                // this column, and convert them to points and shapes.
                //
                // Even a single, uniform line might generate more than one
                // segment, if it happens to cross a colspan (over which it
                // must not be drawn).
                let segments = generate_line_segments(
                    self.grid,
                    tracks,
                    x,
                    vlines_at_column,
                    vline_stroke_at_row,
                )
                .map(|segment| {
                    let LineSegment { stroke, offset: dy, length, priority } = segment;
                    let stroke = (*stroke).clone().unwrap_or_default();
                    let thickness = stroke.thickness;
                    let half = thickness / 2.0;
                    let target = Point::with_y(length + thickness);
                    let vline = Geometry::Line(target).stroked(stroke);
                    (
                        thickness,
                        priority,
                        Point::new(dx, dy - half),
                        FrameItem::Shape(vline, self.span),
                    )
                });

                lines.extend(segments);
            }

            // Render horizontal lines.
            // They are rendered second as they default to appearing on top.
            // First, calculate their offsets from the top of the frame.
            let hline_offsets = points(rows.iter().map(|piece| piece.height));

            // Additionally, determine their indices (the indices of the
            // rows they are drawn on top of). In principle, this will
            // correspond to the rows' indices directly, except for the
            // last hline index, which must be (amount of rows) in order to
            // draw the table's bottom border.
            let hline_indices = rows
                .iter()
                .map(|piece| piece.y)
                .chain(std::iter::once(self.grid.rows.len()))
                .enumerate();

            // Converts a row to the corresponding index in the vector of
            // hlines.
            let hline_index_of_row = |y: usize| {
                if !self.grid.has_gutter {
                    y
                } else if y == self.grid.rows.len() {
                    y / 2 + 1
                } else {
                    // Check the vlines loop for an explanation regarding
                    // these index operations.
                    y / 2
                }
            };

            let get_hlines_at = |y| {
                self.grid
                    .hlines
                    .get(hline_index_of_row(y))
                    .map(Vec::as_slice)
                    .unwrap_or(&[])
            };

            let mut prev_y = None;
            for ((i, y), dy) in hline_indices.zip(hline_offsets) {
                // Position of lines below the row index in the previous iteration.
                let expected_prev_line_position = prev_y
                    .map(|prev_y| {
                        expected_line_position(
                            prev_y + 1,
                            prev_y + 1 == self.grid.rows.len(),
                        )
                    })
                    .unwrap_or(LinePosition::Before);

                // Header's lines at the bottom have priority when repeated.
                // This will store the end bound of the last header if the
                // current iteration is calculating lines under it.
                let last_repeated_header_end_above = match finished_header_rows {
                    Some(info) if prev_y.is_some() && i == info.repeated_amount => {
                        Some(info.last_repeated_header_end)
                    }
                    _ => None,
                };

                // If some grid rows were omitted between the previous resolved
                // row and the current one, we ensure lines below the previous
                // row don't "disappear" and are considered, albeit with less
                // priority. However, don't do this when we're below a header,
                // as it must have more priority instead of less, so it is
                // chained later instead of before (stored in the
                // 'header_hlines' variable below). The exception is when the
                // last row in the header is removed, in which case we append
                // both the lines under the row above us and also (later) the
                // lines under the header's (removed) last row.
                let prev_lines = match prev_y {
                    Some(prev_y)
                        if prev_y + 1 != y
                            && last_repeated_header_end_above.is_none_or(
                                |last_repeated_header_end| {
                                    prev_y + 1 != last_repeated_header_end
                                },
                            ) =>
                    {
                        get_hlines_at(prev_y + 1)
                    }

                    _ => &[],
                };

                let expected_hline_position =
                    expected_line_position(y, y == self.grid.rows.len());

                let hlines_at_y = get_hlines_at(y)
                    .iter()
                    .filter(|line| line.position == expected_hline_position);

                let top_border_hlines = if prev_y.is_none() && y != 0 {
                    // For lines at the top of the region, give priority to
                    // the lines at the top border.
                    get_hlines_at(0)
                } else {
                    &[]
                };

                let mut expected_header_line_position = LinePosition::Before;
                let header_hlines = match (last_repeated_header_end_above, prev_y) {
                    (Some(header_end_above), Some(prev_y))
                        if !self.grid.has_gutter
                            || matches!(
                                self.grid.rows[prev_y],
                                Sizing::Rel(length) if length.is_zero()
                            ) =>
                    {
                        // For lines below a header, give priority to the
                        // lines originally below the header rather than
                        // the lines of what's below the repeated header.
                        // However, no need to do that when we're laying
                        // out the header for the first time, since the
                        // lines being normally laid out then will be
                        // precisely the lines below the header.
                        //
                        // Additionally, we don't repeat lines above the row
                        // below the header when gutter is enabled, since, in
                        // that case, there will be a gutter row between header
                        // and content, so no lines should overlap. The
                        // exception is when the gutter at the end of the
                        // header has a size of zero, which happens when only
                        // column-gutter is specified, for example. In that
                        // case, we still repeat the line under the gutter.
                        expected_header_line_position = expected_line_position(
                            header_end_above,
                            header_end_above == self.grid.rows.len(),
                        );
                        get_hlines_at(header_end_above)
                    }

                    _ => &[],
                };

                // The effective hlines to be considered at this row index are
                // chained in order of increasing priority:
                // 1. Lines from the row right above us, if needed;
                // 2. Lines from the current row (usually, only those are
                // present);
                // 3. Lines from the top border (above the top cells, hence
                // 'before' position only);
                // 4. Lines from the header above us, if present.
                let hlines_at_row =
                    prev_lines
                        .iter()
                        .filter(|line| line.position == expected_prev_line_position)
                        .chain(hlines_at_y)
                        .chain(
                            top_border_hlines
                                .iter()
                                .filter(|line| line.position == LinePosition::Before),
                        )
                        .chain(header_hlines.iter().filter(|line| {
                            line.position == expected_header_line_position
                        }));

                let tracks = self.rcols.iter().copied().enumerate();

                // Normally, given an hline above row y, the row above it is
                // 'y - 1' (if y > 0). However, sometimes that's not true, for
                // example if 'y - 1' is in a previous region, or if 'y - 1'
                // was an empty auto row which was removed. Therefore, we tell
                // the hlines at this index which row is actually above them in
                // the laid out region so they can include that row's bottom
                // strokes in the folding process.
                let local_top_y = prev_y;

                // When we're in the last region, the bottom border stroke
                // doesn't necessarily gain priority like it does in previous
                // regions.
                let in_last_region = frame_index + 1 == frame_amount;

                // Determine all different line segments we have to draw in
                // this row, and convert them to points and shapes.
                let segments = generate_line_segments(
                    self.grid,
                    tracks,
                    y,
                    hlines_at_row,
                    |grid, y, x, stroke| {
                        hline_stroke_at_column(
                            grid,
                            rows,
                            local_top_y,
                            last_repeated_header_end_above,
                            in_last_region,
                            y,
                            x,
                            stroke,
                        )
                    },
                )
                .map(|segment| {
                    let LineSegment { stroke, offset: dx, length, priority } = segment;
                    let stroke = (*stroke).clone().unwrap_or_default();
                    let thickness = stroke.thickness;
                    let half = thickness / 2.0;
                    let dx = if self.is_rtl { self.width - dx - length } else { dx };
                    let target = Point::with_x(length + thickness);
                    let hline = Geometry::Line(target).stroked(stroke);
                    (
                        thickness,
                        priority,
                        Point::new(dx - half, dy),
                        FrameItem::Shape(hline, self.span),
                    )
                });

                // Draw later (after we sort all lines below.)
                lines.extend(segments);

                prev_y = Some(y);
            }

            // Sort by increasing thickness, so that we draw larger strokes
            // on top. When the thickness is the same, sort by priority.
            //
            // Sorting by thickness avoids layering problems where a smaller
            // hline appears "inside" a larger vline. When both have the same
            // size, hlines are drawn on top (since the sort is stable, and
            // they are pushed later).
            lines.sort_by_key(|(thickness, priority, ..)| (*thickness, *priority));

            // Render cell backgrounds.
            // We collect them into a vector so they can all be prepended at
            // once to the frame, together with lines.
            let mut fills = vec![];

            // Reverse with RTL so that later columns start first.
            let mut dx = Abs::zero();
            for (x, &col) in self.rcols.iter().enumerate() {
                let mut dy = Abs::zero();
                for row in rows {
                    // We want to only draw the fill starting at the parent
                    // positions of cells. However, sometimes the parent
                    // position is absent from the current region, either
                    // because the first few rows of a rowspan were empty auto
                    // rows and thus removed from layout, or because the parent
                    // cell was in a previous region (in which case we'd want
                    // to draw its fill again, in the current region).
                    // Therefore, we first analyze the parent position to see
                    // if the current row would be the first row spanned by the
                    // parent cell in this region. If so, this means we have to
                    // start drawing the cell's fill here. If not, we ignore
                    // the position `(x, row.y)`, as its fill will already have
                    // been rendered before.
                    //
                    // Note: In the case of gutter rows, we have to check the
                    // row below before discarding them fully, because a
                    // gutter row might be the first row spanned by a rowspan
                    // in this region (e.g. if the first row was empty and
                    // therefore removed), so its fill could start in that
                    // gutter row. That's why we use
                    // 'effective_parent_cell_position'.
                    let parent = self
                        .grid
                        .effective_parent_cell_position(x, row.y)
                        .filter(|parent| {
                            // Ensure this is the first column spanned by the
                            // cell before drawing its fill, otherwise we
                            // already rendered its fill in a previous
                            // iteration of the outer loop (and/or this is a
                            // gutter column, which we ignore).
                            //
                            // Additionally, we should only draw the fill when
                            // this row is the local parent Y for this cell,
                            // that is, the first row spanned by the cell's
                            // parent in this region, because if the parent
                            // cell's fill was already drawn in a previous
                            // region, we must render it again in later regions
                            // spanned by that cell. Note that said condition
                            // always holds when the current cell has a rowspan
                            // of 1 and we're not currently at a gutter row.
                            parent.x == x
                                && (parent.y == row.y
                                    || rows
                                        .iter()
                                        .find(|row| row.y >= parent.y)
                                        .is_some_and(|first_spanned_row| {
                                            first_spanned_row.y == row.y
                                        }))
                        });

                    if let Some(parent) = parent {
                        let cell = self.grid.cell(parent.x, parent.y).unwrap();
                        let fill = cell.fill.clone();
                        if let Some(fill) = fill {
                            let rowspan = self.grid.effective_rowspan_of_cell(cell);
                            let height = if rowspan == 1 {
                                row.height
                            } else {
                                rows.iter()
                                    .filter(|row| {
                                        (parent.y..parent.y + rowspan).contains(&row.y)
                                    })
                                    .map(|row| row.height)
                                    .sum()
                            };
                            let width = self.cell_spanned_width(cell, x);
                            let mut pos = Point::new(dx, dy);
                            if self.is_rtl {
                                // In RTL cells expand to the left, thus the
                                // position must additionally be offset by the
                                // cell's width.
                                pos.x = self.width - (dx + width);
                            }
                            let size = Size::new(width, height);
                            let rect = Geometry::Rect(size).filled(fill);
                            fills.push((pos, FrameItem::Shape(rect, self.span)));
                        }
                    }
                    dy += row.height;
                }
                dx += col;
            }

            // Now we render each fill and stroke by prepending to the frame,
            // such that both appear below cell contents. Fills come first so
            // that they appear below lines.
            frame.prepend_multiple(
                fills
                    .into_iter()
                    .chain(lines.into_iter().map(|(_, _, point, shape)| (point, shape))),
            );
        }

        Ok(Fragment::frames(finished))
    }

    /// Determine all column sizes.
    fn measure_columns(&mut self, engine: &mut Engine) -> SourceResult<()> {
        // Sum of sizes of resolved relative tracks.
        let mut rel = Abs::zero();

        // Sum of fractions of all fractional tracks.
        let mut fr = Fr::zero();

        // Resolve the size of all relative columns and compute the sum of all
        // fractional tracks.
        for (&col, rcol) in self.grid.cols.iter().zip(&mut self.rcols) {
            match col {
                Sizing::Auto => {}
                Sizing::Rel(v) => {
                    let resolved =
                        v.resolve(self.styles).relative_to(self.regions.base().x);
                    *rcol = resolved;
                    rel += resolved;
                }
                Sizing::Fr(v) => fr += v,
            }
        }

        // Size that is not used by fixed-size columns.
        let available = self.regions.size.x - rel;
        if available >= Abs::zero() {
            // Determine size of auto columns.
            let (auto, count) = self.measure_auto_columns(engine, available)?;

            // If there is remaining space, distribute it to fractional columns,
            // otherwise shrink auto columns.
            let remaining = available - auto;
            if remaining >= Abs::zero() {
                self.grow_fractional_columns(remaining, fr);
            } else {
                self.shrink_auto_columns(available, count);
            }
        }

        // Sum up the resolved column sizes once here.
        self.width = self.rcols.iter().sum();

        Ok(())
    }

    /// Total width spanned by the cell (among resolved columns).
    /// Includes spanned gutter columns.
    pub(super) fn cell_spanned_width(&self, cell: &Cell, x: usize) -> Abs {
        let colspan = self.grid.effective_colspan_of_cell(cell);
        self.rcols.iter().skip(x).take(colspan).sum()
    }

    /// Measure the size that is available to auto columns.
    fn measure_auto_columns(
        &mut self,
        engine: &mut Engine,
        available: Abs,
    ) -> SourceResult<(Abs, usize)> {
        let mut auto = Abs::zero();
        let mut count = 0;
        let all_frac_cols = self
            .grid
            .cols
            .iter()
            .enumerate()
            .filter(|(_, col)| col.is_fractional())
            .map(|(x, _)| x)
            .collect::<Vec<_>>();

        // Determine size of auto columns by laying out all cells in those
        // columns, measuring them and finding the largest one.
        for (x, &col) in self.grid.cols.iter().enumerate() {
            if col != Sizing::Auto {
                continue;
            }

            let mut resolved = Abs::zero();
            for y in 0..self.grid.rows.len() {
                // We get the parent cell in case this is a merged position.
                let Some(parent) = self.grid.parent_cell_position(x, y) else {
                    continue;
                };
                if parent.y != y {
                    // Don't check the width of rowspans more than once.
                    continue;
                }
                let cell = self.grid.cell(parent.x, parent.y).unwrap();
                let colspan = self.grid.effective_colspan_of_cell(cell);
                if colspan > 1 {
                    let last_spanned_auto_col = self
                        .grid
                        .cols
                        .iter()
                        .enumerate()
                        .skip(parent.x)
                        .take(colspan)
                        .rev()
                        .find(|(_, col)| **col == Sizing::Auto)
                        .map(|(x, _)| x);

                    if last_spanned_auto_col != Some(x) {
                        // A colspan only affects the size of the last spanned
                        // auto column.
                        continue;
                    }
                }

                if colspan > 1
                    && self.regions.size.x.is_finite()
                    && !all_frac_cols.is_empty()
                    && all_frac_cols
                        .iter()
                        .all(|x| (parent.x..parent.x + colspan).contains(x))
                {
                    // Additionally, as a heuristic, a colspan won't affect the
                    // size of auto columns if it already spans all fractional
                    // columns, since those would already expand to provide all
                    // remaining available after auto column sizing to that
                    // cell. However, this heuristic is only valid in finite
                    // regions (pages without 'auto' width), since otherwise
                    // the fractional columns don't expand at all.
                    continue;
                }

                // Sum the heights of spanned rows to find the expected
                // available height for the cell, unless it spans a fractional
                // or auto column.
                let rowspan = self.grid.effective_rowspan_of_cell(cell);
                let height = self
                    .grid
                    .rows
                    .iter()
                    .skip(y)
                    .take(rowspan)
                    .try_fold(Abs::zero(), |acc, col| {
                        // For relative rows, we can already resolve the correct
                        // base and for auto and fr we could only guess anyway.
                        match col {
                            Sizing::Rel(v) => Some(
                                acc + v
                                    .resolve(self.styles)
                                    .relative_to(self.regions.base().y),
                            ),
                            _ => None,
                        }
                    })
                    .unwrap_or_else(|| self.regions.base().y);

                // Don't expand this auto column more than the cell actually
                // needs. To do this, we check how much the other, previously
                // resolved columns provide to the cell in terms of width
                // (if it is a colspan), and subtract this from its expected
                // width when comparing with other cells in this column. Note
                // that, since this is the last auto column spanned by this
                // cell, all other auto columns will already have been resolved
                // and will be considered.
                // Only fractional columns will be excluded from this
                // calculation, which can lead to auto columns being expanded
                // unnecessarily when cells span both a fractional column and
                // an auto column. One mitigation for this is the heuristic
                // used above to not expand the last auto column spanned by a
                // cell if it spans all fractional columns in a finite region.
                let already_covered_width = self.cell_spanned_width(cell, parent.x);

                let size = Size::new(available, height);
                let pod = Region::new(size, Axes::splat(false));
                let frame =
                    layout_cell(cell, engine, 0, self.styles, pod.into())?.into_frame();
                resolved.set_max(frame.width() - already_covered_width);
            }

            self.rcols[x] = resolved;
            auto += resolved;
            count += 1;
        }

        Ok((auto, count))
    }

    /// Distribute remaining space to fractional columns.
    fn grow_fractional_columns(&mut self, remaining: Abs, fr: Fr) {
        if fr.is_zero() {
            return;
        }

        for (&col, rcol) in self.grid.cols.iter().zip(&mut self.rcols) {
            if let Sizing::Fr(v) = col {
                *rcol = v.share(fr, remaining);
            }
        }
    }

    /// Redistribute space to auto columns so that each gets a fair share.
    fn shrink_auto_columns(&mut self, available: Abs, count: usize) {
        let mut last;
        let mut fair = -Abs::inf();
        let mut redistribute = available;
        let mut overlarge = count;
        let mut changed = true;

        // Iteratively remove columns that don't need to be shrunk.
        while changed && overlarge > 0 {
            changed = false;
            last = fair;
            fair = redistribute / (overlarge as f64);

            for (&col, &rcol) in self.grid.cols.iter().zip(&self.rcols) {
                // Remove an auto column if it is not overlarge (rcol <= fair),
                // but also hasn't already been removed (rcol > last).
                if col == Sizing::Auto && rcol <= fair && rcol > last {
                    redistribute -= rcol;
                    overlarge -= 1;
                    changed = true;
                }
            }
        }

        // Redistribute space fairly among overlarge columns.
        for (&col, rcol) in self.grid.cols.iter().zip(&mut self.rcols) {
            if col == Sizing::Auto && *rcol > fair {
                *rcol = fair;
            }
        }
    }

    /// Layout a row with automatic height. Such a row may break across multiple
    /// regions.
    fn layout_auto_row(
        &mut self,
        engine: &mut Engine,
        disambiguator: usize,
        y: usize,
    ) -> SourceResult<()> {
        // Determine the size for each region of the row. If the first region
        // ends up empty for some column, skip the region and remeasure.
        let mut resolved = match self.measure_auto_row(
            engine,
            disambiguator,
            y,
            true,
            self.unbreakable_rows_left,
            None,
        )? {
            Some(resolved) => resolved,
            None => {
                self.finish_region(engine, false)?;
                self.measure_auto_row(
                    engine,
                    disambiguator,
                    y,
                    false,
                    self.unbreakable_rows_left,
                    None,
                )?
                .unwrap()
            }
        };

        // Nothing to layout.
        if resolved.is_empty() {
            return Ok(());
        }

        // Layout into a single region.
        if let &[first] = resolved.as_slice() {
            let frame = self.layout_single_row(engine, disambiguator, first, y)?;
            self.push_row(frame, y, true);

            if let Some(row_height) = &mut self.row_state.current_row_height {
                // Add to header height, as we are in a header row.
                *row_height += first;
            }

            return Ok(());
        }

        // Expand all but the last region.
        // Skip the first region if the space is eaten up by an fr row.
        let len = resolved.len();
        for ((i, region), target) in
            self.regions
                .iter()
                .enumerate()
                .zip(&mut resolved[..len - 1])
                .skip(self.current.lrows.iter().any(|row| matches!(row, Row::Fr(..)))
                    as usize)
        {
            // Subtract header and footer heights from the region height when
            // it's not the first. Ignore non-repeating headers as they only
            // appear on the first region by definition.
            target.set_max(
                region.y
                    - if i > 0 {
                        self.current.repeating_header_height + self.current.footer_height
                    } else {
                        Abs::zero()
                    },
            );
        }

        // Layout into multiple regions.
        let fragment = self.layout_multi_row(engine, disambiguator, &resolved, y)?;
        let len = fragment.len();
        for (i, frame) in fragment.into_iter().enumerate() {
            self.push_row(frame, y, i + 1 == len);
            if i + 1 < len {
                self.finish_region(engine, false)?;
            }
        }

        Ok(())
    }

    /// Measure the regions sizes of an auto row. The option is always `Some(_)`
    /// if `can_skip` is false.
    /// If `unbreakable_rows_left` is positive, this function shall only return
    /// a single frame. Useful when an unbreakable rowspan crosses this auto
    /// row.
    /// The `row_group_data` option is used within the unbreakable row group
    /// simulator to predict the height of the auto row if previous rows in the
    /// group were placed in the same region.
    pub(super) fn measure_auto_row(
        &self,
        engine: &mut Engine,
        disambiguator: usize,
        y: usize,
        can_skip: bool,
        unbreakable_rows_left: usize,
        row_group_data: Option<&UnbreakableRowGroup>,
    ) -> SourceResult<Option<Vec<Abs>>> {
        let breakable = unbreakable_rows_left == 0;
        let mut resolved: Vec<Abs> = vec![];
        let mut pending_rowspans: Vec<(usize, usize, Vec<Abs>)> = vec![];

        for x in 0..self.rcols.len() {
            // Get the parent cell in case this is a merged position.
            let Some(parent) = self.grid.parent_cell_position(x, y) else {
                // Skip gutter columns.
                continue;
            };
            if parent.x != x {
                // Only check the height of a colspan once.
                continue;
            }
            // The parent cell is never a gutter or merged position.
            let cell = self.grid.cell(parent.x, parent.y).unwrap();
            let rowspan = self.grid.effective_rowspan_of_cell(cell);

            if rowspan > 1 {
                let last_spanned_auto_row = self
                    .grid
                    .rows
                    .iter()
                    .enumerate()
                    .skip(parent.y)
                    .take(rowspan)
                    .rev()
                    .find(|(_, &row)| row == Sizing::Auto)
                    .map(|(y, _)| y);

                if last_spanned_auto_row != Some(y) {
                    // A rowspan should only affect the height of its last
                    // spanned auto row.
                    continue;
                }
            }

            let measurement_data = self.prepare_auto_row_cell_measurement(
                parent,
                cell,
                breakable,
                row_group_data,
            );
            let size = Axes::new(measurement_data.width, measurement_data.height);
            let backlog =
                measurement_data.backlog.unwrap_or(&measurement_data.custom_backlog);

            let pod = if !breakable {
                // Force cell to fit into a single region when the row is
                // unbreakable, even when it is a breakable rowspan, as a best
                // effort.
                let mut pod: Regions = Region::new(size, self.regions.expand).into();
                pod.full = measurement_data.full;

                if measurement_data.frames_in_previous_regions > 0 {
                    // Best effort to conciliate a breakable rowspan which
                    // started at a previous region going through an
                    // unbreakable auto row. Ensure it goes through previously
                    // laid out regions, but stops at this one when measuring.
                    pod.backlog = backlog;
                }

                pod
            } else {
                // This row is breakable, so measure the cell normally, with
                // the initial height and backlog determined previously.
                let mut pod = self.regions;
                pod.size = size;
                pod.backlog = backlog;
                pod.full = measurement_data.full;
                pod.last = measurement_data.last;

                pod
            };

            let frames =
                layout_cell(cell, engine, disambiguator, self.styles, pod)?.into_frames();

            // Skip the first region if one cell in it is empty. Then,
            // remeasure.
            if let Some([first, rest @ ..]) =
                frames.get(measurement_data.frames_in_previous_regions..)
            {
                if can_skip
                    && breakable
                    && first.is_empty()
                    && rest.iter().any(|frame| !frame.is_empty())
                {
                    return Ok(None);
                }
            }

            // Skip frames from previous regions if applicable.
            let mut sizes = frames
                .iter()
                .skip(measurement_data.frames_in_previous_regions)
                .map(|frame| frame.height())
                .collect::<Vec<_>>();

            // Don't expand this row more than the cell needs.
            // To figure out how much height the cell needs, we must first
            // subtract, from the cell's expected height, the already resolved
            // heights of its spanned rows. Note that this is the last spanned
            // auto row, so all previous auto rows were already resolved, as
            // well as fractional rows in previous regions.
            // Additionally, we subtract the heights of fixed-size rows which
            // weren't laid out yet, since those heights won't change in
            // principle.
            // Upcoming fractional rows are ignored.
            // Upcoming gutter rows might be removed, so we need to simulate
            // them.
            if rowspan > 1 {
                let should_simulate = self.prepare_rowspan_sizes(
                    y,
                    &mut sizes,
                    cell,
                    parent.y,
                    rowspan,
                    unbreakable_rows_left,
                    &measurement_data,
                );

                if should_simulate {
                    // Rowspan spans gutter and is breakable. We'll need to
                    // run a simulation to predict how much this auto row needs
                    // to expand so that the rowspan's contents fit into the
                    // table.
                    pending_rowspans.push((parent.y, rowspan, sizes));
                    continue;
                }
            }

            let mut sizes = sizes.into_iter();

            for (target, size) in resolved.iter_mut().zip(&mut sizes) {
                target.set_max(size);
            }

            // New heights are maximal by virtue of being new. Note that
            // this extend only uses the rest of the sizes iterator.
            resolved.extend(sizes);
        }

        // Simulate the upcoming regions in order to predict how much we need
        // to expand this auto row for rowspans which span gutter.
        if !pending_rowspans.is_empty() {
            self.simulate_and_measure_rowspans_in_auto_row(
                y,
                &mut resolved,
                &pending_rowspans,
                unbreakable_rows_left,
                row_group_data,
                disambiguator,
                engine,
            )?;
        }

        debug_assert!(breakable || resolved.len() <= 1);

        Ok(Some(resolved))
    }

    /// Layout a row with relative height. Such a row cannot break across
    /// multiple regions, but it may force a region break.
    fn layout_relative_row(
        &mut self,
        engine: &mut Engine,
        disambiguator: usize,
        v: Rel<Length>,
        y: usize,
    ) -> SourceResult<()> {
        let resolved = v.resolve(self.styles).relative_to(self.regions.base().y);
        let frame = self.layout_single_row(engine, disambiguator, resolved, y)?;

        if let Some(row_height) = &mut self.row_state.current_row_height {
            // Add to header height, as we are in a header row.
            *row_height += resolved;
        }

        // Skip to fitting region, but only if we aren't part of an unbreakable
        // row group. We use 'may_progress_with_repeats' to stop trying if we
        // would skip to a region with the same height and where the same
        // headers would be repeated.
        let height = frame.height();
        while self.unbreakable_rows_left == 0
            && !self.regions.size.y.fits(height)
            && self.may_progress_with_repeats()
        {
            self.finish_region(engine, false)?;

            // Don't skip multiple regions for gutter and don't push a row.
            if self.grid.is_gutter_track(y) {
                return Ok(());
            }
        }

        self.push_row(frame, y, true);

        Ok(())
    }

    /// Layout a row with fixed height and return its frame.
    fn layout_single_row(
        &mut self,
        engine: &mut Engine,
        disambiguator: usize,
        height: Abs,
        y: usize,
    ) -> SourceResult<Frame> {
        if !self.width.is_finite() {
            bail!(self.span, "cannot create grid with infinite width");
        }

        if !height.is_finite() {
            bail!(self.span, "cannot create grid with infinite height");
        }

        let mut output = Frame::soft(Size::new(self.width, height));
        let mut offset = Point::zero();

        for (x, &rcol) in self.rcols.iter().enumerate() {
            if let Some(cell) = self.grid.cell(x, y) {
                // Rowspans have a separate layout step
                if cell.rowspan.get() == 1 {
                    let width = self.cell_spanned_width(cell, x);
                    let size = Size::new(width, height);
                    let mut pod: Regions = Region::new(size, Axes::splat(true)).into();
                    if self.grid.rows[y] == Sizing::Auto
                        && self.unbreakable_rows_left == 0
                    {
                        // Cells at breakable auto rows have lengths relative
                        // to the entire page, unlike cells in unbreakable auto
                        // rows.
                        pod.full = self.regions.full;
                    }
                    let frame =
                        layout_cell(cell, engine, disambiguator, self.styles, pod)?
                            .into_frame();
                    let mut pos = offset;
                    if self.is_rtl {
                        // In RTL cells expand to the left, thus the position
                        // must additionally be offset by the cell's width.
                        pos.x = self.width - (pos.x + width);
                    }
                    output.push_frame(pos, frame);
                }
            }

            offset.x += rcol;
        }

        Ok(output)
    }

    /// Layout a row spanning multiple regions.
    fn layout_multi_row(
        &mut self,
        engine: &mut Engine,
        disambiguator: usize,
        heights: &[Abs],
        y: usize,
    ) -> SourceResult<Fragment> {
        // Prepare frames.
        let mut outputs: Vec<_> = heights
            .iter()
            .map(|&h| Frame::soft(Size::new(self.width, h)))
            .collect();

        // Prepare regions.
        let size = Size::new(self.width, heights[0]);
        let mut pod: Regions = Region::new(size, Axes::splat(true)).into();
        pod.full = self.regions.full;
        pod.backlog = &heights[1..];

        // Layout the row.
        let mut offset = Point::zero();
        for (x, &rcol) in self.rcols.iter().enumerate() {
            if let Some(cell) = self.grid.cell(x, y) {
                // Rowspans have a separate layout step
                if cell.rowspan.get() == 1 {
                    let width = self.cell_spanned_width(cell, x);
                    pod.size.x = width;

                    // Push the layouted frames into the individual output frames.
                    let fragment =
                        layout_cell(cell, engine, disambiguator, self.styles, pod)?;
                    for (output, frame) in outputs.iter_mut().zip(fragment) {
                        let mut pos = offset;
                        if self.is_rtl {
                            // In RTL cells expand to the left, thus the
                            // position must additionally be offset by the
                            // cell's width.
                            pos.x = self.width - (offset.x + width);
                        }
                        output.push_frame(pos, frame);
                    }
                }
            }

            offset.x += rcol;
        }

        Ok(Fragment::frames(outputs))
    }

    /// Push a row frame into the current region.
    /// The `is_last` parameter must be `true` if this is the last frame which
    /// will be pushed for this particular row. It can be `false` for rows
    /// spanning multiple regions.
    fn push_row(&mut self, frame: Frame, y: usize, is_last: bool) {
        if !self.row_state.in_active_repeatable {
            // There is now a row after the rows equipped with orphan
            // prevention, so no need to keep moving them anymore.
            self.flush_orphans();
        }
        self.regions.size.y -= frame.height();
        self.current.lrows.push(Row::Frame(frame, y, is_last));
    }

    /// Finish rows for one region.
    pub(super) fn finish_region(
        &mut self,
        engine: &mut Engine,
        last: bool,
    ) -> SourceResult<()> {
        // The latest rows have orphan prevention (headers) and no other rows
        // were placed, so remove those rows and try again in a new region,
        // unless this is the last region.
        if let Some(orphan_snapshot) = self.current.lrows_orphan_snapshot.take() {
            if !last {
                self.current.lrows.truncate(orphan_snapshot);
                self.current.repeated_header_rows =
                    self.current.repeated_header_rows.min(orphan_snapshot);

                if orphan_snapshot == 0 {
                    // Removed all repeated headers.
                    self.current.last_repeated_header_end = 0;
                }
            }
        }

        if self
            .current
            .lrows
            .last()
            .is_some_and(|row| self.grid.is_gutter_track(row.index()))
        {
            // Remove the last row in the region if it is a gutter row.
            self.current.lrows.pop().unwrap();
            self.current.repeated_header_rows =
                self.current.repeated_header_rows.min(self.current.lrows.len());
        }

        // If no rows other than the footer have been laid out so far
        // (e.g. due to header orphan prevention), and there are rows
        // beside the footer, then don't lay it out at all.
        //
        // It is worth noting that the footer is made non-repeatable at
        // the grid resolving stage if it is short-lived, that is, if
        // it is at the start of the table (or right after headers at
        // the start of the table).
        //
        // TODO(subfooters): explicitly check for short-lived footers.
        // TODO(subfooters): widow prevention for non-repeated footers with a
        // similar mechanism / when implementing multiple footers.
        let footer_would_be_widow = matches!(&self.grid.footer, Some(footer) if footer.repeated)
            && self.current.lrows.is_empty()
            && self.current.could_progress_at_top;

        let mut laid_out_footer_start = None;
        if !footer_would_be_widow {
            if let Some(footer) =
                self.grid.footer.as_ref().and_then(Repeatable::as_repeated)
            {
                // Don't layout the footer if it would be alone with the header
                // in the page (hence the widow check), and don't layout it
                // twice (check below).
                //
                // TODO(subfooters): this check can be replaced by a vector of
                // repeating footers in the future, and/or some "pending
                // footers" vector for footers we're about to place.
                if self.current.lrows.iter().all(|row| row.index() < footer.start) {
                    laid_out_footer_start = Some(footer.start);
                    self.layout_footer(footer, engine, self.finished.len())?;
                }
            }
        }

        // Determine the height of existing rows in the region.
        let mut used = Abs::zero();
        let mut fr = Fr::zero();
        for row in &self.current.lrows {
            match row {
                Row::Frame(frame, _, _) => used += frame.height(),
                Row::Fr(v, _, _) => fr += *v,
            }
        }

        // Determine the size of the grid in this region, expanding fully if
        // there are fr rows.
        let mut size = Size::new(self.width, used).min(self.current.initial);
        if fr.get() > 0.0 && self.current.initial.y.is_finite() {
            size.y = self.current.initial.y;
        }

        // The frame for the region.
        let mut output = Frame::soft(size);
        let mut pos = Point::zero();
        let mut rrows = vec![];
        let current_region = self.finished.len();
        let mut repeated_header_row_height = Abs::zero();

        // Place finished rows and layout fractional rows.
        for (i, row) in std::mem::take(&mut self.current.lrows).into_iter().enumerate() {
            let (frame, y, is_last) = match row {
                Row::Frame(frame, y, is_last) => (frame, y, is_last),
                Row::Fr(v, y, disambiguator) => {
                    let remaining = self.regions.full - used;
                    let height = v.share(fr, remaining);
                    (self.layout_single_row(engine, disambiguator, height, y)?, y, true)
                }
            };

            let height = frame.height();
            if i < self.current.repeated_header_rows {
                repeated_header_row_height += height;
            }

            // Ensure rowspans which span this row will have enough space to
            // be laid out over it later.
            for rowspan in self
                .rowspans
                .iter_mut()
                .filter(|rowspan| (rowspan.y..rowspan.y + rowspan.rowspan).contains(&y))
                .filter(|rowspan| {
                    rowspan.max_resolved_row.is_none_or(|max_row| y > max_row)
                })
            {
                // If the first region wasn't defined yet, it will have the
                // initial value of usize::MAX, so we can set it to the current
                // region's index.
                if rowspan.first_region > current_region {
                    rowspan.first_region = current_region;
                    // The rowspan starts at this region, precisely at this
                    // row. In other regions, it will start at dy = 0.
                    rowspan.dy = pos.y;
                    // When we layout the rowspan later, the full size of the
                    // pod must be equal to the full size of the first region
                    // it appears in.
                    rowspan.region_full = self.regions.full;
                }
                let amount_missing_heights = (current_region + 1)
                    .saturating_sub(rowspan.heights.len() + rowspan.first_region);

                // Ensure the vector of heights is long enough such that the
                // last height is the one for the current region.
                rowspan
                    .heights
                    .extend(std::iter::repeat_n(Abs::zero(), amount_missing_heights));

                // Ensure that, in this region, the rowspan will span at least
                // this row.
                *rowspan.heights.last_mut().unwrap() += height;

                if is_last {
                    // Do not extend the rowspan through this row again, even
                    // if it is repeated in a future region.
                    rowspan.max_resolved_row = Some(y);
                }
            }

            // We use a for loop over indices to avoid borrow checking
            // problems (we need to mutate the rowspans vector, so we can't
            // have an iterator actively borrowing it). We keep a separate
            // 'i' variable so we can step the counter back after removing
            // a rowspan (see explanation below).
            let mut i = 0;
            while let Some(rowspan) = self.rowspans.get(i) {
                // Layout any rowspans which end at this row, but only if this is
                // this row's last frame (to avoid having the rowspan stop being
                // laid out at the first frame of the row).
                // Any rowspans ending before this row are laid out even
                // on this row's first frame.
                if laid_out_footer_start.is_none_or(|footer_start| {
                    // If this is a footer row, then only lay out this rowspan
                    // if the rowspan is contained within the footer.
                    y < footer_start || rowspan.y >= footer_start
                }) && (rowspan.y + rowspan.rowspan < y + 1
                    || rowspan.y + rowspan.rowspan == y + 1 && is_last)
                {
                    // Rowspan ends at this or an earlier row, so we take
                    // it from the rowspans vector and lay it out.
                    // It's safe to pass the current region as a possible
                    // region for the rowspan to be laid out in, even if
                    // the rowspan's last row was at an earlier region,
                    // because the rowspan won't have an entry for this
                    // region in its 'heights' vector if it doesn't span
                    // any rows in this region.
                    //
                    // Here we don't advance the index counter ('i') because
                    // a new element we haven't checked yet in this loop
                    // will take the index of the now removed element, so
                    // we have to check the same index again in the next
                    // iteration.
                    let rowspan = self.rowspans.remove(i);
                    self.layout_rowspan(
                        rowspan,
                        Some((&mut output, repeated_header_row_height)),
                        engine,
                    )?;
                } else {
                    i += 1;
                }
            }

            output.push_frame(pos, frame);
            rrows.push(RowPiece { height, y });
            pos.y += height;
        }

        self.finish_region_internal(
            output,
            rrows,
            FinishedHeaderRowInfo {
                repeated_amount: self.current.repeated_header_rows,
                last_repeated_header_end: self.current.last_repeated_header_end,
                repeated_height: repeated_header_row_height,
            },
        );

        if !last {
            self.current.repeated_header_rows = 0;
            self.current.last_repeated_header_end = 0;
            self.current.repeating_header_height = Abs::zero();
            self.current.repeating_header_heights.clear();

            let disambiguator = self.finished.len();
            if let Some(footer) =
                self.grid.footer.as_ref().and_then(Repeatable::as_repeated)
            {
                self.prepare_footer(footer, engine, disambiguator)?;
            }

            // Ensure rows don't try to overrun the footer.
            // Note that header layout will only subtract this again if it has
            // to skip regions to fit headers, so there is no risk of
            // subtracting this twice.
            self.regions.size.y -= self.current.footer_height;
            self.current.initial_after_repeats = self.regions.size.y;

            if !self.repeating_headers.is_empty() || !self.pending_headers.is_empty() {
                // Add headers to the new region.
                self.layout_active_headers(engine)?;
            }
        }

        Ok(())
    }

    /// Advances to the next region, registering the finished output and
    /// resolved rows for the current region in the appropriate vectors.
    pub(super) fn finish_region_internal(
        &mut self,
        output: Frame,
        resolved_rows: Vec<RowPiece>,
        header_row_info: FinishedHeaderRowInfo,
    ) {
        self.finished.push(output);
        self.rrows.push(resolved_rows);
        self.regions.next();
        self.current.initial = self.regions.size;

        // Repeats haven't been laid out yet, so in the meantime, this will
        // represent the initial height after repeats laid out so far, and will
        // be gradually updated when preparing footers and repeating headers.
        self.current.initial_after_repeats = self.current.initial.y;

        self.current.could_progress_at_top = self.regions.may_progress();

        if !self.grid.headers.is_empty() {
            self.finished_header_rows.push(header_row_info);
        }

        // Ensure orphan prevention is handled before resolving rows.
        debug_assert!(self.current.lrows_orphan_snapshot.is_none());
    }
}

/// Turn an iterator of extents into an iterator of offsets before, in between,
/// and after the extents, e.g. [10mm, 5mm] -> [0mm, 10mm, 15mm].
pub(super) fn points(
    extents: impl IntoIterator<Item = Abs>,
) -> impl Iterator<Item = Abs> {
    let mut offset = Abs::zero();
    std::iter::once(Abs::zero()).chain(extents).map(move |extent| {
        offset += extent;
        offset
    })
}
