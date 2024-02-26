use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::Resolve;
use crate::layout::{
    Abs, Axes, Cell, GridLayouter, LayoutMultiple, Point, Regions, Size, Sizing,
};
use crate::util::{MaybeReverseIter, Numeric};

/// All information needed to layout a single rowspan.
pub(super) struct Rowspan {
    // First column of this rowspan.
    pub(super) x: usize,
    // First row of this rowspan.
    pub(super) y: usize,
    // Amount of rows spanned by the cell at (x, y).
    pub(super) rowspan: usize,
    /// The horizontal offset of this rowspan in all regions.
    pub(super) dx: Abs,
    /// The vertical offset of this rowspan in the first region.
    pub(super) dy: Abs,
    /// The index of the first region this rowspan appears in.
    pub(super) first_region: usize,
    // The full height in the first region this rowspan appears in, for
    // relative sizing.
    pub(super) region_full: Abs,
    /// The vertical space available for this rowspan in each region.
    pub(super) heights: Vec<Abs>,
}

impl<'a> GridLayouter<'a> {
    /// Layout rowspans over the already finished regions.
    /// We need to do this later once we already know the heights of all
    /// spanned rows.
    pub(super) fn layout_rowspans(&mut self, engine: &mut Engine) -> SourceResult<()> {
        for rowspan_data in std::mem::take(&mut self.rowspans) {
            let Rowspan {
                x, y, dx, dy, first_region, region_full, heights, ..
            } = rowspan_data;
            let Some((&first_height, backlog)) = heights.split_first() else {
                // Nothing to layout
                continue;
            };
            let first_column = self.rcols[x];
            let cell = self.grid.cell(x, y).unwrap();
            let width = self.cell_spanned_width(cell, x);

            // Prepare regions.
            let size = Size::new(width, first_height);
            let mut pod = Regions::one(size, Axes::splat(true));
            pod.full = region_full;
            pod.backlog = backlog;

            // Push the layouted frames directly into the finished frames.
            // At first, we draw the rowspan starting at its expected offset
            // in the first region.
            let mut pos = Point::new(dx, dy);
            let fragment = cell.layout(engine, self.styles, pod)?;
            for (finished, mut frame) in
                self.finished.iter_mut().skip(first_region).zip(fragment)
            {
                if self.is_rtl {
                    let offset = Point::with_x(-width + first_column);
                    frame.translate(offset);
                }
                finished.push_frame(pos, frame);

                // From the second region onwards, the rowspan's continuation
                // starts at the very top.
                pos.y = Abs::zero();
            }
        }

        Ok(())
    }

    /// Checks if a row contains the beginning of one or more rowspan cells.
    /// If so, adds them to the rowspan vector.
    /// Additionally, if the rowspan cells are unbreakable, updates the
    /// 'unbreakable_rows_left' counter such that the rows spanned by those
    /// cells are laid out together, in the same region.
    pub(super) fn check_for_rowspans(&mut self, y: usize) {
        // We will compute the horizontal offset of each rowspan in advance.
        // For that reason, we must reverse the column order when using RTL.
        let mut dx = Abs::zero();
        for (x, &rcol) in self.rcols.iter().enumerate().rev_if(self.is_rtl) {
            let Some(cell) = self.grid.cell(x, y) else {
                dx += rcol;
                continue;
            };
            let rowspan = self.grid.effective_rowspan_of_cell(cell);
            if rowspan == 1 {
                dx += rcol;
                continue;
            }
            // Rowspan detected. We will lay it out later.
            self.rowspans.push(Rowspan {
                x,
                y,
                rowspan,
                dx,
                // The four fields below will be updated in 'finish_region'.
                dy: Abs::zero(),
                first_region: usize::MAX,
                region_full: Abs::zero(),
                heights: vec![],
            });
            dx += rcol;
        }
    }

    /// Checks if the cell at a given position is the parent of an unbreakable
    /// rowspan. This only holds when the cell spans multiple rows, of which
    /// none are auto rows.
    pub(super) fn is_unbreakable_rowspan(&self, cell: &Cell, y: usize) -> bool {
        let rowspan = self.grid.effective_rowspan_of_cell(cell);
        // Unbreakable rowspans span more than one row and do not span any auto
        // rows.
        rowspan > 1
            && self
                .grid
                .rows
                .iter()
                .skip(y)
                .take(rowspan)
                .all(|&row| row != Sizing::Auto)
    }

    /// Checks if the upcoming rows will be grouped together under an
    /// unbreakable row group, and, if so, advances regions until there is
    /// enough space for them. This can be needed, for example, if there's an
    /// unbreakable rowspan crossing those rows.
    pub(super) fn check_for_unbreakable_rows(
        &mut self,
        current_row: usize,
        engine: &mut Engine,
    ) -> SourceResult<()> {
        if self.unbreakable_rows_left == 0 {
            let (unbreakable_rows, group_height) =
                self.simulate_unbreakable_row_group(current_row, engine)?;

            // Skip to fitting region.
            while !self.regions.size.y.fits(group_height) && !self.regions.in_last() {
                self.finish_region(engine)?;
            }
            self.unbreakable_rows_left = unbreakable_rows;
        }

        Ok(())
    }

    /// Simulates a group of unbreakable rows, starting with the index of the
    /// first row in the group. Keeps adding rows to the group until none have
    /// unbreakable cells in common.
    pub(super) fn simulate_unbreakable_row_group(
        &self,
        first_row: usize,
        engine: &mut Engine,
    ) -> SourceResult<(usize, Abs)> {
        let mut group_height = Abs::zero();
        let mut unbreakable_rows = vec![];
        let mut unbreakable_rows_left = 0;
        for (y, row) in self.grid.rows.iter().enumerate().skip(first_row) {
            let additional_unbreakable_rows = self.check_for_unbreakable_cells(y);
            unbreakable_rows_left =
                unbreakable_rows_left.max(additional_unbreakable_rows);
            if unbreakable_rows_left == 0 {
                // This check is in case the first row does not have any
                // unbreakable cells. Therefore, no unbreakable row group
                // is formed.
                break;
            }
            let height = match row {
                Sizing::Rel(v) => {
                    v.resolve(self.styles).relative_to(self.regions.base().y)
                }
                Sizing::Auto => self
                    .measure_auto_row(
                        engine,
                        y,
                        false,
                        true,
                        unbreakable_rows_left,
                        group_height,
                        &unbreakable_rows,
                    )?
                    .unwrap()
                    .first()
                    .copied()
                    .unwrap_or_else(Abs::zero),
                // Fractional rows don't matter when calculating the space
                // needed for unbreakable rows
                Sizing::Fr(_) => Abs::zero(),
            };
            group_height += height;
            unbreakable_rows.push((y, height));
            unbreakable_rows_left -= 1;
            if unbreakable_rows_left == 0 {
                // This second check is necessary so we can tell distinct
                // but consecutive unbreakable row groups apart. If the
                // unbreakable row group ended at this row, we stop before
                // checking the next one.
                break;
            }
        }
        Ok((unbreakable_rows.len(), group_height))
    }

    /// Checks if one or more of the cells at the given row are unbreakable.
    /// If so, returns the largest rowspan among the unbreakable cells;
    /// the spanned rows must, as a result, be laid out in the same region.
    pub(super) fn check_for_unbreakable_cells(&self, y: usize) -> usize {
        let mut unbreakable_rows_left = 0;
        for x in 0..self.grid.cols.len() {
            let Some(cell) = self.grid.cell(x, y) else {
                continue;
            };
            let rowspan = self.grid.effective_rowspan_of_cell(cell);
            if rowspan > 1 && self.is_unbreakable_rowspan(cell, y) {
                // At least the next 'rowspan' rows should be grouped together,
                // in the same page, as this rowspan can't be broken apart.
                // Since the last row in a rowspan is never gutter, here we
                // satisfy the invariant that a gutter row won't be the last
                // row in the unbreakable row group after the remaining rows
                // are added.
                unbreakable_rows_left = unbreakable_rows_left.max(rowspan);
            }
        }

        unbreakable_rows_left
    }

    /// Performs a simulation to predict by how much height the last spanned
    /// auto row will have to expand, given the current sizes of the auto row
    /// in each region and the pending rowspans' data (parent Y, rowspan amount
    /// and vector of requested sizes).
    pub(super) fn simulate_and_measure_rowspans_in_auto_row(
        &self,
        y: usize,
        unbreakable_rows_left: usize,
        previous_unbreakable_height: Abs,
        resolved: &mut Vec<Abs>,
        pending_rowspans: &[(usize, usize, Vec<Abs>)],
        engine: &mut Engine,
    ) -> SourceResult<()> {
        // To begin our simulation, we have to unify the sizes demanded by
        // each rowspan into one simple vector of sizes, as if they were
        // all a single rowspan. These sizes will be appended to
        // 'resolved' once we finish our simulation.
        let mut simulated_sizes: Vec<Abs> = vec![];
        let last_resolved_size = resolved.last().copied();
        let mut max_spanned_row = y;
        for (parent_y, rowspan, sizes) in pending_rowspans {
            let mut sizes = sizes.iter();
            for (target, size) in resolved.iter_mut().zip(&mut sizes) {
                // First, we update the already resolved sizes as required
                // by this rowspan. Our simulation won't otherwise change
                // already resolved sizes, other than, perhaps, the last
                // one.
                target.set_max(*size);
            }
            for (extra_rowspan_target, extra_size) in
                simulated_sizes.iter_mut().zip(&mut sizes)
            {
                // The remaining sizes are exclusive to rowspans, since
                // other cells in this row didn't require as many regions.
                extra_rowspan_target.set_max(*extra_size);
            }
            simulated_sizes.extend(sizes);
            max_spanned_row = max_spanned_row.max(parent_y + rowspan - 1);
        }
        if simulated_sizes.is_empty() && resolved.last().copied() == last_resolved_size {
            // The rowspans already fit in the already resolved sizes.
            // No need for simulation.
            return Ok(());
        }

        // We will be updating the last resolved size (expanding the auto
        // row) as needed. Therefore, consider it as part of the simulation.
        // At the end, we push it back.
        if let Some(modified_last_resolved_size) = resolved.pop() {
            simulated_sizes.insert(0, modified_last_resolved_size);
        }

        // Prepare regions for simulation.
        let mut simulated_regions = self.regions;
        simulated_regions.size.y -= previous_unbreakable_height;
        for _ in 0..resolved.len() {
            // Ensure we start at the region where we will expand the auto
            // row.
            simulated_regions.next();
        }
        if let Some(original_last_resolved_size) = last_resolved_size {
            // We're now at the (current) last region of this auto row.
            // Consider resolved height as already taken space.
            simulated_regions.size.y -= original_last_resolved_size;
        }

        // The max growable height, for now, will always correspond to the sum
        // of all spanned gutter heights. This number is defined after the
        // first simulation, at which we go through all gutter rows to define
        // their total sum.
        let mut max_growable_height = None;
        let mut amount_to_grow = Abs::zero();
        // Try to simulate up to 5 times. If it doesn't stabilize, we give up.
        for _attempt in 0..5 {
            let mut regions = simulated_regions;
            let mut total_spanned_gutter_height = Abs::zero();
            let mut unbreakable_rows_left = unbreakable_rows_left;

            // Total height that changed, prompting the auto row to grow a bit
            // more since the last simulation.
            let mut extra_amount_to_grow = Abs::zero();

            // Height of the latest spanned gutter row.
            // Zero if it was removed.
            let mut latest_spanned_gutter_height = Abs::zero();
            let spanned_rows = &self.grid.rows[y + 1..=max_spanned_row];
            for (offset, row) in spanned_rows.iter().enumerate() {
                if max_growable_height.is_some_and(|max_growable_height| {
                    total_spanned_gutter_height + amount_to_grow + extra_amount_to_grow
                        >= max_growable_height
                }) {
                    // Stop the simulation, as we have already fully covered
                    // the height rowspans need.
                    break;
                }
                let spanned_y = y + 1 + offset;
                let is_gutter = self.grid.is_gutter_track(spanned_y);

                if unbreakable_rows_left == 0 {
                    // Simulate unbreakable row groups
                    let (unbreakable_rows, group_height) =
                        self.simulate_unbreakable_row_group(spanned_y, engine)?;
                    while !self.regions.size.y.fits(group_height)
                        && !self.regions.in_last()
                    {
                        extra_amount_to_grow += latest_spanned_gutter_height;
                        latest_spanned_gutter_height = Abs::zero();
                        regions.next();
                    }

                    unbreakable_rows_left = unbreakable_rows;
                }

                match row {
                    // Fixed-size rows are what we are interested in.
                    Sizing::Rel(v) => {
                        let height = v.resolve(self.styles).relative_to(regions.base().y);
                        if is_gutter {
                            total_spanned_gutter_height += height;
                            latest_spanned_gutter_height = height;
                        }
                        let mut skipped_region = false;
                        while !regions.size.y.fits(height) && !regions.in_last() {
                            // A row was pushed to the next region. Therefore,
                            // the immediately preceding gutter row is removed.
                            extra_amount_to_grow += latest_spanned_gutter_height;
                            latest_spanned_gutter_height = Abs::zero();
                            skipped_region = true;
                            regions.next();
                        }
                        if !skipped_region || !is_gutter {
                            // No gutter at the top of a new region.
                            regions.size.y -= height;
                        }
                    }
                    Sizing::Auto => {
                        // We only simulate for rowspans which end at the
                        // current auto row. Therefore, there won't be any
                        // further auto rows.
                        unreachable!();
                    }
                    // For now, we ignore fractional rows on simulation.
                    Sizing::Fr(_) if is_gutter => {
                        latest_spanned_gutter_height = Abs::zero();
                    }
                    Sizing::Fr(_) => {}
                }

                unbreakable_rows_left = unbreakable_rows_left.saturating_sub(1);
            }
            let max_growable_height = max_growable_height.unwrap_or_else(|| {
                max_growable_height = Some(total_spanned_gutter_height);
                total_spanned_gutter_height
            });
            if extra_amount_to_grow.is_zero() {
                // The amount to grow has stabilized.
                // Reduce sizes by the amount actually spanned by gutter.
                subtract_end_sizes(
                    &mut simulated_sizes,
                    max_growable_height - amount_to_grow,
                );
                if let Some(last_resolved_size) = last_resolved_size {
                    // Ensure the first simulated size is at least as large as
                    // the last resolved size (its initial value). As it was
                    // already resolved before, we must not reduce below the
                    // resolved size to avoid problems with non-rowspan cells.
                    if let Some(first_simulated_size) = simulated_sizes.first_mut() {
                        first_simulated_size.set_max(last_resolved_size);
                    } else {
                        simulated_sizes.push(last_resolved_size);
                    }
                }
                break;
            }

            // The amount to grow the auto row by has changed since the last
            // simulation. Let's try again or abort if we reached the max
            // attempts.
            amount_to_grow += extra_amount_to_grow;

            // For the next simulation attempt, we consider that the auto row
            // has additionally grown by the amount given in this attempt, to
            // see if it will have to grow further in the next attempt.
            while !extra_amount_to_grow.is_zero()
                && simulated_regions.size.y < extra_amount_to_grow
            {
                extra_amount_to_grow -= regions.size.y.max(Abs::zero());
                regions.next();
            }
            simulated_regions.size.y -= extra_amount_to_grow;
        }

        // If the simulation didn't stabilize above, we will be pushing the
        // unmodified vector of rowspan sizes, ignoring gutter. That means the
        // auto row will expand more than it normally should. But we did try to
        // ensure it wouldn't expand more than it should through the above
        // simulation, to our best efforts.
        resolved.extend(simulated_sizes);

        Ok(())
    }
}

/// Subtracts some size from the end of a vector of sizes.
/// For example, subtracting 5pt from \[2pt, 1pt, 3pt\] will result in \[1pt\].
pub(super) fn subtract_end_sizes(sizes: &mut Vec<Abs>, mut subtract: Abs) {
    while subtract > Abs::zero() && sizes.last().is_some_and(|&size| size <= subtract) {
        subtract -= sizes.pop().unwrap();
    }
    if subtract > Abs::zero() {
        if let Some(last_size) = sizes.last_mut() {
            *last_size -= subtract;
        }
    }
}
