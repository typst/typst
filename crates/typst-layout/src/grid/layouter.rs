use std::fmt::Debug;

use typst_library::diag::{bail, SourceResult};
use typst_library::engine::Engine;
use typst_library::foundations::{Resolve, StyleChain};
use typst_library::layout::{
    Abs, Axes, Dir, Fr, Fragment, Frame, FrameItem, Length, Point, Region, Regions, Rel,
    Size, Sizing,
};
use typst_library::text::TextElem;
use typst_library::visualize::Geometry;
use typst_syntax::Span;
use typst_utils::{MaybeReverseIter, Numeric};

use super::{
    generate_line_segments, hline_stroke_at_column, vline_stroke_at_row, Cell, CellGrid,
    LinePosition, LineSegment, Repeatable, Rowspan, UnbreakableRowGroup,
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
    /// Resolve row sizes, by region.
    pub(super) rrows: Vec<Vec<RowPiece>>,
    /// Rows in the current region.
    pub(super) lrows: Vec<Row>,
    /// The amount of unbreakable rows remaining to be laid out in the
    /// current unbreakable row group. While this is positive, no region breaks
    /// should occur.
    pub(super) unbreakable_rows_left: usize,
    /// Rowspans not yet laid out because not all of their spanned rows were
    /// laid out yet.
    pub(super) rowspans: Vec<Rowspan>,
    /// The initial size of the current region before we started subtracting.
    pub(super) initial: Size,
    /// Frames for finished regions.
    pub(super) finished: Vec<Frame>,
    /// Whether this is an RTL grid.
    pub(super) is_rtl: bool,
    /// The simulated header height.
    /// This field is reset in `layout_header` and properly updated by
    /// `layout_auto_row` and `layout_relative_row`, and should not be read
    /// before all header rows are fully laid out. It is usually fine because
    /// header rows themselves are unbreakable, and unbreakable rows do not
    /// need to read this field at all.
    pub(super) header_height: Abs,
    /// The simulated footer height for this region.
    /// The simulation occurs before any rows are laid out for a region.
    pub(super) footer_height: Abs,
    /// The span of the grid element.
    pub(super) span: Span,
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
            lrows: vec![],
            unbreakable_rows_left: 0,
            rowspans: vec![],
            initial: regions.size,
            finished: vec![],
            is_rtl: TextElem::dir_in(styles) == Dir::RTL,
            header_height: Abs::zero(),
            footer_height: Abs::zero(),
            span,
        }
    }

    /// Determines the columns sizes and then layouts the grid row-by-row.
    pub fn layout(mut self, engine: &mut Engine) -> SourceResult<Fragment> {
        self.measure_columns(engine)?;

        if let Some(Repeatable::Repeated(footer)) = &self.grid.footer {
            // Ensure rows in the first region will be aware of the possible
            // presence of the footer.
            self.prepare_footer(footer, engine, 0)?;
            if matches!(self.grid.header, None | Some(Repeatable::NotRepeated(_))) {
                // No repeatable header, so we won't subtract it later.
                self.regions.size.y -= self.footer_height;
            }
        }

        for y in 0..self.grid.rows.len() {
            if let Some(Repeatable::Repeated(header)) = &self.grid.header {
                if y < header.end {
                    if y == 0 {
                        self.layout_header(header, engine, 0)?;
                        self.regions.size.y -= self.footer_height;
                    }
                    // Skip header rows during normal layout.
                    continue;
                }
            }

            if let Some(Repeatable::Repeated(footer)) = &self.grid.footer {
                if y >= footer.start {
                    if y == footer.start {
                        self.layout_footer(footer, engine, self.finished.len())?;
                    }
                    continue;
                }
            }

            self.layout_row(y, engine, 0)?;
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

    /// Layout the given row.
    pub(super) fn layout_row(
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
        if is_content_row || !self.lrows.is_empty() {
            match self.grid.rows[y] {
                Sizing::Auto => self.layout_auto_row(engine, disambiguator, y)?,
                Sizing::Rel(v) => {
                    self.layout_relative_row(engine, disambiguator, v, y)?
                }
                Sizing::Fr(v) => self.lrows.push(Row::Fr(v, y, disambiguator)),
            }
        }

        self.unbreakable_rows_left = self.unbreakable_rows_left.saturating_sub(1);

        Ok(())
    }

    /// Add lines and backgrounds.
    fn render_fills_strokes(mut self) -> SourceResult<Fragment> {
        let mut finished = std::mem::take(&mut self.finished);
        let frame_amount = finished.len();
        for ((frame_index, frame), rows) in
            finished.iter_mut().enumerate().zip(&self.rrows)
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
                .chain(std::iter::once(self.grid.rows.len()));

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
            for (y, dy) in hline_indices.zip(hline_offsets) {
                // Position of lines below the row index in the previous iteration.
                let expected_prev_line_position = prev_y
                    .map(|prev_y| {
                        expected_line_position(
                            prev_y + 1,
                            prev_y + 1 == self.grid.rows.len(),
                        )
                    })
                    .unwrap_or(LinePosition::Before);

                // FIXME: In the future, directly specify in 'self.rrows' when
                // we place a repeated header rather than its original rows.
                // That would let us remove most of those verbose checks, both
                // in 'lines.rs' and here. Those checks also aren't fully
                // accurate either, since they will also trigger when some rows
                // have been removed between the header and what's below it.
                let is_under_repeated_header = self
                    .grid
                    .header
                    .as_ref()
                    .and_then(Repeatable::as_repeated)
                    .zip(prev_y)
                    .is_some_and(|(header, prev_y)| {
                        // Note: 'y == header.end' would mean we're right below
                        // the NON-REPEATED header, so that case should return
                        // false.
                        prev_y < header.end && y > header.end
                    });

                // If some grid rows were omitted between the previous resolved
                // row and the current one, we ensure lines below the previous
                // row don't "disappear" and are considered, albeit with less
                // priority. However, don't do this when we're below a header,
                // as it must have more priority instead of less, so it is
                // chained later instead of before. The exception is when the
                // last row in the header is removed, in which case we append
                // both the lines under the row above us and also (later) the
                // lines under the header's (removed) last row.
                let prev_lines = prev_y
                    .filter(|prev_y| {
                        prev_y + 1 != y
                            && (!is_under_repeated_header
                                || self
                                    .grid
                                    .header
                                    .as_ref()
                                    .and_then(Repeatable::as_repeated)
                                    .is_some_and(|header| prev_y + 1 != header.end))
                    })
                    .map(|prev_y| get_hlines_at(prev_y + 1))
                    .unwrap_or(&[]);

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
                let header_hlines = if let Some((Repeatable::Repeated(header), prev_y)) =
                    self.grid.header.as_ref().zip(prev_y)
                {
                    if is_under_repeated_header
                        && (!self.grid.has_gutter
                            || matches!(
                                self.grid.rows[prev_y],
                                Sizing::Rel(length) if length.is_zero()
                            ))
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
                            header.end,
                            header.end == self.grid.rows.len(),
                        );
                        get_hlines_at(header.end)
                    } else {
                        &[]
                    }
                } else {
                    &[]
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
            for (x, &col) in self.rcols.iter().enumerate().rev_if(self.is_rtl) {
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
                            // In the grid, cell colspans expand to the right,
                            // so we're at the leftmost (lowest 'x') column
                            // spanned by the cell. However, in RTL, cells
                            // expand to the left. Therefore, without the
                            // offset below, cell fills would start at the
                            // rightmost visual position of a cell and extend
                            // over to unrelated columns to the right in RTL.
                            // We avoid this by ensuring the fill starts at the
                            // very left of the cell, even with colspan > 1.
                            let offset =
                                if self.is_rtl { -width + col } else { Abs::zero() };
                            let pos = Point::new(dx + offset, dy);
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
                let frame = cell.layout(engine, 0, self.styles, pod.into())?.into_frame();
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

            if self
                .grid
                .header
                .as_ref()
                .and_then(Repeatable::as_repeated)
                .is_some_and(|header| y < header.end)
            {
                // Add to header height.
                self.header_height += first;
            }

            return Ok(());
        }

        // Expand all but the last region.
        // Skip the first region if the space is eaten up by an fr row.
        let len = resolved.len();
        for ((i, region), target) in self
            .regions
            .iter()
            .enumerate()
            .zip(&mut resolved[..len - 1])
            .skip(self.lrows.iter().any(|row| matches!(row, Row::Fr(..))) as usize)
        {
            // Subtract header and footer heights from the region height when
            // it's not the first.
            target.set_max(
                region.y
                    - if i > 0 {
                        self.header_height + self.footer_height
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
                cell.layout(engine, disambiguator, self.styles, pod)?.into_frames();

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

        if self
            .grid
            .header
            .as_ref()
            .and_then(Repeatable::as_repeated)
            .is_some_and(|header| y < header.end)
        {
            // Add to header height.
            self.header_height += resolved;
        }

        // Skip to fitting region, but only if we aren't part of an unbreakable
        // row group. We use 'in_last_with_offset' so our 'in_last' call
        // properly considers that a header and a footer would be added on each
        // region break.
        let height = frame.height();
        while self.unbreakable_rows_left == 0
            && !self.regions.size.y.fits(height)
            && !in_last_with_offset(self.regions, self.header_height + self.footer_height)
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
        let mut pos = Point::zero();

        // Reverse the column order when using RTL.
        for (x, &rcol) in self.rcols.iter().enumerate().rev_if(self.is_rtl) {
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
                    let frame = cell
                        .layout(engine, disambiguator, self.styles, pod)?
                        .into_frame();
                    let mut pos = pos;
                    if self.is_rtl {
                        // In the grid, cell colspans expand to the right,
                        // so we're at the leftmost (lowest 'x') column
                        // spanned by the cell. However, in RTL, cells
                        // expand to the left. Therefore, without the
                        // offset below, the cell's contents would be laid out
                        // starting at its rightmost visual position and extend
                        // over to unrelated cells to its right in RTL.
                        // We avoid this by ensuring the rendered cell starts at
                        // the very left of the cell, even with colspan > 1.
                        let offset = -width + rcol;
                        pos.x += offset;
                    }
                    output.push_frame(pos, frame);
                }
            }

            pos.x += rcol;
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
        let mut pos = Point::zero();
        for (x, &rcol) in self.rcols.iter().enumerate().rev_if(self.is_rtl) {
            if let Some(cell) = self.grid.cell(x, y) {
                // Rowspans have a separate layout step
                if cell.rowspan.get() == 1 {
                    let width = self.cell_spanned_width(cell, x);
                    pod.size.x = width;

                    // Push the layouted frames into the individual output frames.
                    let fragment =
                        cell.layout(engine, disambiguator, self.styles, pod)?;
                    for (output, frame) in outputs.iter_mut().zip(fragment) {
                        let mut pos = pos;
                        if self.is_rtl {
                            let offset = -width + rcol;
                            pos.x += offset;
                        }
                        output.push_frame(pos, frame);
                    }
                }
            }

            pos.x += rcol;
        }

        Ok(Fragment::frames(outputs))
    }

    /// Push a row frame into the current region.
    /// The `is_last` parameter must be `true` if this is the last frame which
    /// will be pushed for this particular row. It can be `false` for rows
    /// spanning multiple regions.
    fn push_row(&mut self, frame: Frame, y: usize, is_last: bool) {
        self.regions.size.y -= frame.height();
        self.lrows.push(Row::Frame(frame, y, is_last));
    }

    /// Finish rows for one region.
    pub(super) fn finish_region(
        &mut self,
        engine: &mut Engine,
        last: bool,
    ) -> SourceResult<()> {
        if self
            .lrows
            .last()
            .is_some_and(|row| self.grid.is_gutter_track(row.index()))
        {
            // Remove the last row in the region if it is a gutter row.
            self.lrows.pop().unwrap();
        }

        // If no rows other than the footer have been laid out so far, and
        // there are rows beside the footer, then don't lay it out at all.
        // This check doesn't apply, and is thus overridden, when there is a
        // header.
        let mut footer_would_be_orphan = self.lrows.is_empty()
            && !in_last_with_offset(
                self.regions,
                self.header_height + self.footer_height,
            )
            && self
                .grid
                .footer
                .as_ref()
                .and_then(Repeatable::as_repeated)
                .is_some_and(|footer| footer.start != 0);

        if let Some(Repeatable::Repeated(header)) = &self.grid.header {
            if self.grid.rows.len() > header.end
                && self
                    .grid
                    .footer
                    .as_ref()
                    .and_then(Repeatable::as_repeated)
                    .map_or(true, |footer| footer.start != header.end)
                && self.lrows.last().is_some_and(|row| row.index() < header.end)
                && !in_last_with_offset(
                    self.regions,
                    self.header_height + self.footer_height,
                )
            {
                // Header and footer would be alone in this region, but there are more
                // rows beyond the header and the footer. Push an empty region.
                self.lrows.clear();
                footer_would_be_orphan = true;
            }
        }

        let mut laid_out_footer_start = None;
        if let Some(Repeatable::Repeated(footer)) = &self.grid.footer {
            // Don't layout the footer if it would be alone with the header in
            // the page, and don't layout it twice.
            if !footer_would_be_orphan
                && self.lrows.iter().all(|row| row.index() < footer.start)
            {
                laid_out_footer_start = Some(footer.start);
                self.layout_footer(footer, engine, self.finished.len())?;
            }
        }

        // Determine the height of existing rows in the region.
        let mut used = Abs::zero();
        let mut fr = Fr::zero();
        for row in &self.lrows {
            match row {
                Row::Frame(frame, _, _) => used += frame.height(),
                Row::Fr(v, _, _) => fr += *v,
            }
        }

        // Determine the size of the grid in this region, expanding fully if
        // there are fr rows.
        let mut size = Size::new(self.width, used).min(self.initial);
        if fr.get() > 0.0 && self.initial.y.is_finite() {
            size.y = self.initial.y;
        }

        // The frame for the region.
        let mut output = Frame::soft(size);
        let mut pos = Point::zero();
        let mut rrows = vec![];
        let current_region = self.finished.len();

        // Place finished rows and layout fractional rows.
        for row in std::mem::take(&mut self.lrows) {
            let (frame, y, is_last) = match row {
                Row::Frame(frame, y, is_last) => (frame, y, is_last),
                Row::Fr(v, y, disambiguator) => {
                    let remaining = self.regions.full - used;
                    let height = v.share(fr, remaining);
                    (self.layout_single_row(engine, disambiguator, height, y)?, y, true)
                }
            };

            let height = frame.height();

            // Ensure rowspans which span this row will have enough space to
            // be laid out over it later.
            for rowspan in self
                .rowspans
                .iter_mut()
                .filter(|rowspan| (rowspan.y..rowspan.y + rowspan.rowspan).contains(&y))
                .filter(|rowspan| {
                    rowspan.max_resolved_row.map_or(true, |max_row| y > max_row)
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
                    .extend(std::iter::repeat(Abs::zero()).take(amount_missing_heights));

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
                if laid_out_footer_start.map_or(true, |footer_start| {
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
                    self.layout_rowspan(rowspan, Some((&mut output, &rrows)), engine)?;
                } else {
                    i += 1;
                }
            }

            output.push_frame(pos, frame);
            rrows.push(RowPiece { height, y });
            pos.y += height;
        }

        self.finish_region_internal(output, rrows);

        if !last {
            let disambiguator = self.finished.len();
            if let Some(Repeatable::Repeated(footer)) = &self.grid.footer {
                self.prepare_footer(footer, engine, disambiguator)?;
            }

            if let Some(Repeatable::Repeated(header)) = &self.grid.header {
                // Add a header to the new region.
                self.layout_header(header, engine, disambiguator)?;
            }

            // Ensure rows don't try to overrun the footer.
            self.regions.size.y -= self.footer_height;
        }

        Ok(())
    }

    /// Advances to the next region, registering the finished output and
    /// resolved rows for the current region in the appropriate vectors.
    pub(super) fn finish_region_internal(
        &mut self,
        output: Frame,
        resolved_rows: Vec<RowPiece>,
    ) {
        self.finished.push(output);
        self.rrows.push(resolved_rows);
        self.regions.next();
        self.initial = self.regions.size;
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

/// Checks if the first region of a sequence of regions is the last usable
/// region, assuming that the last region will always be occupied by some
/// specific offset height, even after calling `.next()`, due to some
/// additional logic which adds content automatically on each region turn (in
/// our case, headers).
pub(super) fn in_last_with_offset(regions: Regions<'_>, offset: Abs) -> bool {
    regions.backlog.is_empty()
        && regions.last.map_or(true, |height| regions.size.y + offset == height)
}
