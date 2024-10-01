use super::layout::{in_last_with_offset, points, Row, RowPiece};
use super::repeated::Repeatable;
use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::Resolve;
use crate::layout::{
    Abs, Axes, Cell, Frame, GridLayouter, Point, Region, Regions, Size, Sizing,
};
use crate::utils::MaybeReverseIter;

/// All information needed to layout a single rowspan.
pub(super) struct Rowspan {
    /// First column of this rowspan.
    pub(super) x: usize,
    /// First row of this rowspan.
    pub(super) y: usize,
    /// The disambiguator for laying out the cells.
    pub(super) disambiguator: usize,
    /// Amount of rows spanned by the cell at (x, y).
    pub(super) rowspan: usize,
    /// Whether all rows of the rowspan are part of an unbreakable row group.
    /// This is true e.g. in headers and footers, regardless of what the user
    /// specified for the parent cell's `breakable` field.
    pub(super) is_effectively_unbreakable: bool,
    /// The horizontal offset of this rowspan in all regions.
    pub(super) dx: Abs,
    /// The vertical offset of this rowspan in the first region.
    pub(super) dy: Abs,
    /// The index of the first region this rowspan appears in.
    pub(super) first_region: usize,
    /// The full height in the first region this rowspan appears in, for
    /// relative sizing.
    pub(super) region_full: Abs,
    /// The vertical space available for this rowspan in each region.
    pub(super) heights: Vec<Abs>,
    /// The index of the largest resolved spanned row so far.
    /// Once a spanned row is resolved and its height added to `heights`, this
    /// number is increased. Older rows, even if repeated through e.g. a
    /// header, will no longer contribute height to this rowspan.
    ///
    /// This is `None` if no spanned rows were resolved in `finish_region` yet.
    pub(super) max_resolved_row: Option<usize>,
}

/// The output of the simulation of an unbreakable row group.
#[derive(Default)]
pub(super) struct UnbreakableRowGroup {
    /// The rows in this group of unbreakable rows.
    /// Includes their indices and their predicted heights.
    pub(super) rows: Vec<(usize, Abs)>,
    /// The total height of this row group.
    pub(super) height: Abs,
}

/// Data used to measure a cell in an auto row.
pub(super) struct CellMeasurementData<'layouter> {
    /// The available width for the cell across all regions.
    pub(super) width: Abs,
    /// The available height for the cell in its first region.
    /// Infinite when the auto row is unbreakable.
    pub(super) height: Abs,
    /// The backlog of heights available for the cell in later regions.
    ///
    /// When this is `None`, the `custom_backlog` field should be used instead.
    /// That's because, otherwise, this field would have to contain a reference
    /// to the `custom_backlog` field, which isn't possible in Rust without
    /// resorting to unsafe hacks.
    pub(super) backlog: Option<&'layouter [Abs]>,
    /// If the backlog needs to be built from scratch instead of reusing the
    /// one at the current region, which is the case of a multi-region rowspan
    /// (needs to join its backlog of already laid out heights with the current
    /// backlog), then this vector will store the new backlog.
    pub(super) custom_backlog: Vec<Abs>,
    /// The full height of the first region of the cell.
    /// Infinite when the auto row is unbreakable.
    pub(super) full: Abs,
    /// The height of the last repeated region to use in the measurement pod,
    /// if any.
    pub(super) last: Option<Abs>,
    /// The total height of previous rows spanned by the cell in the current
    /// region (so far).
    pub(super) height_in_this_region: Abs,
    /// The amount of previous regions spanned by the cell.
    /// They are skipped for measurement purposes.
    pub(super) frames_in_previous_regions: usize,
}

impl<'a> GridLayouter<'a> {
    /// Layout a rowspan over the already finished regions, plus the current
    /// region's frame and resolved rows, if it wasn't finished yet (because
    /// we're being called from `finish_region`, but note that this function is
    /// also called once after all regions are finished, in which case
    /// `current_region_data` is `None`).
    ///
    /// We need to do this only once we already know the heights of all
    /// spanned rows, which is only possible after laying out the last row
    /// spanned by the rowspan (or some row immediately after the last one).
    pub(super) fn layout_rowspan(
        &mut self,
        rowspan_data: Rowspan,
        current_region_data: Option<(&mut Frame, &[RowPiece])>,
        engine: &mut Engine,
    ) -> SourceResult<()> {
        let Rowspan {
            x,
            y,
            disambiguator,
            rowspan,
            is_effectively_unbreakable,
            dx,
            dy,
            first_region,
            region_full,
            heights,
            ..
        } = rowspan_data;
        let [first_height, backlog @ ..] = heights.as_slice() else {
            // Nothing to layout.
            return Ok(());
        };
        let first_column = self.rcols[x];
        let cell = self.grid.cell(x, y).unwrap();
        let width = self.cell_spanned_width(cell, x);
        let dx = if self.is_rtl { dx - width + first_column } else { dx };

        // Prepare regions.
        let size = Size::new(width, *first_height);
        let mut pod: Regions = Region::new(size, Axes::splat(true)).into();
        pod.backlog = backlog;

        if !is_effectively_unbreakable
            && self.grid.rows[y..][..rowspan]
                .iter()
                .any(|spanned_row| spanned_row == &Sizing::Auto)
        {
            // If the rowspan spans an auto row and is breakable, it will see
            // '100%' as the full page height, at least at its first region.
            // This is consistent with how it is measured, and with how
            // non-rowspan cells behave in auto rows.
            pod.full = region_full;
        }

        // Push the layouted frames directly into the finished frames.
        let fragment = cell.layout(engine, disambiguator, self.styles, pod)?;
        let (current_region, current_rrows) = current_region_data.unzip();
        for ((i, finished), frame) in self
            .finished
            .iter_mut()
            .chain(current_region.into_iter())
            .skip(first_region)
            .enumerate()
            .zip(fragment)
        {
            let dy = if i == 0 {
                // At first, we draw the rowspan starting at its expected
                // vertical offset in the first region.
                dy
            } else {
                // The rowspan continuation starts after the header (thus,
                // at a position after the sum of the laid out header
                // rows).
                if let Some(Repeatable::Repeated(header)) = &self.grid.header {
                    let header_rows = self
                        .rrows
                        .get(i)
                        .map(Vec::as_slice)
                        .or(current_rrows)
                        .unwrap_or(&[])
                        .iter()
                        .take_while(|row| row.y < header.end);

                    header_rows.map(|row| row.height).sum()
                } else {
                    // Without a header, start at the very top of the region.
                    Abs::zero()
                }
            };

            finished.push_frame(Point::new(dx, dy), frame);
        }

        Ok(())
    }

    /// Checks if a row contains the beginning of one or more rowspan cells.
    /// If so, adds them to the rowspans vector.
    pub(super) fn check_for_rowspans(&mut self, disambiguator: usize, y: usize) {
        // We will compute the horizontal offset of each rowspan in advance.
        // For that reason, we must reverse the column order when using RTL.
        let offsets = points(self.rcols.iter().copied().rev_if(self.is_rtl));
        for (x, dx) in (0..self.rcols.len()).rev_if(self.is_rtl).zip(offsets) {
            let Some(cell) = self.grid.cell(x, y) else {
                continue;
            };
            let rowspan = self.grid.effective_rowspan_of_cell(cell);
            if rowspan > 1 {
                // Rowspan detected. We will lay it out later.
                self.rowspans.push(Rowspan {
                    x,
                    y,
                    disambiguator,
                    rowspan,
                    // The field below will be updated in
                    // 'check_for_unbreakable_rows'.
                    is_effectively_unbreakable: !cell.breakable,
                    dx,
                    // The four fields below will be updated in 'finish_region'.
                    dy: Abs::zero(),
                    first_region: usize::MAX,
                    region_full: Abs::zero(),
                    heights: vec![],
                    max_resolved_row: None,
                });
            }
        }
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
            // By default, the amount of unbreakable rows starting at the
            // current row is dynamic and depends on the amount of upcoming
            // unbreakable cells (with or without a rowspan setting).
            let mut amount_unbreakable_rows = None;
            if let Some(Repeatable::NotRepeated(header)) = &self.grid.header {
                if current_row < header.end {
                    // Non-repeated header, so keep it unbreakable.
                    amount_unbreakable_rows = Some(header.end);
                }
            }
            if let Some(Repeatable::NotRepeated(footer)) = &self.grid.footer {
                if current_row >= footer.start {
                    // Non-repeated footer, so keep it unbreakable.
                    amount_unbreakable_rows = Some(self.grid.rows.len() - footer.start);
                }
            }

            let row_group = self.simulate_unbreakable_row_group(
                current_row,
                amount_unbreakable_rows,
                &self.regions,
                engine,
                0,
            )?;

            // Skip to fitting region.
            while !self.regions.size.y.fits(row_group.height)
                && !in_last_with_offset(
                    self.regions,
                    self.header_height + self.footer_height,
                )
            {
                self.finish_region(engine, false)?;
            }

            // Update unbreakable rows left.
            self.unbreakable_rows_left = row_group.rows.len();
        }

        if self.unbreakable_rows_left > 1 {
            // Mark rowspans as effectively unbreakable where applicable
            // (if all of their spanned rows would be in the same unbreakable
            // row group).
            // Not needed if only one unbreakable row is left, since, then,
            // no rowspan will be effectively unbreakable, at least necessarily.
            // Note that this function is called after 'check_for_rowspans' and
            // potentially updates the amount of remaining unbreakable rows, so
            // it wouldn't be accurate to only check for this condition in that
            // function. We need to check here instead.
            for rowspan_data in
                self.rowspans.iter_mut().filter(|rowspan| rowspan.y == current_row)
            {
                rowspan_data.is_effectively_unbreakable |=
                    self.unbreakable_rows_left >= rowspan_data.rowspan;
            }
        }

        Ok(())
    }

    /// Simulates a group of unbreakable rows, starting with the index of the
    /// first row in the group. If `amount_unbreakable_rows` is `None`, keeps
    /// adding rows to the group until none have unbreakable cells in common.
    /// Otherwise, adds specifically the given amount of rows to the group.
    ///
    /// This is used to figure out how much height the next unbreakable row
    /// group (if any) needs.
    pub(super) fn simulate_unbreakable_row_group(
        &self,
        first_row: usize,
        amount_unbreakable_rows: Option<usize>,
        regions: &Regions<'_>,
        engine: &mut Engine,
        disambiguator: usize,
    ) -> SourceResult<UnbreakableRowGroup> {
        let mut row_group = UnbreakableRowGroup::default();
        let mut unbreakable_rows_left = amount_unbreakable_rows.unwrap_or(0);
        for (y, row) in self.grid.rows.iter().enumerate().skip(first_row) {
            if amount_unbreakable_rows.is_none() {
                // When we don't set a fixed amount of unbreakable rows,
                // determine the amount based on the rowspan of unbreakable
                // cells in rows.
                let additional_unbreakable_rows = self.check_for_unbreakable_cells(y);
                unbreakable_rows_left =
                    unbreakable_rows_left.max(additional_unbreakable_rows);
            }
            if unbreakable_rows_left == 0 {
                // This check is in case the first row does not have any
                // unbreakable cells. Therefore, no unbreakable row group
                // is formed.
                break;
            }
            let height = match row {
                Sizing::Rel(v) => v.resolve(self.styles).relative_to(regions.base().y),

                // No need to pass the regions to the auto row, since
                // unbreakable auto rows are always measured with infinite
                // height, ignore backlog, and do not invoke the rowspan
                // simulation procedure at all.
                Sizing::Auto => self
                    .measure_auto_row(
                        engine,
                        disambiguator,
                        y,
                        false,
                        unbreakable_rows_left,
                        Some(&row_group),
                    )?
                    .unwrap()
                    .first()
                    .copied()
                    .unwrap_or_else(Abs::zero),
                // Fractional rows don't matter when calculating the space
                // needed for unbreakable rows
                Sizing::Fr(_) => Abs::zero(),
            };
            row_group.height += height;
            row_group.rows.push((y, height));
            unbreakable_rows_left -= 1;
            if unbreakable_rows_left == 0 {
                // This second check is necessary so we can tell distinct
                // but consecutive unbreakable row groups apart. If the
                // unbreakable row group ended at this row, we stop before
                // checking the next one.
                break;
            }
        }

        Ok(row_group)
    }

    /// Checks if one or more of the cells at the given row are unbreakable.
    /// If so, returns the largest rowspan among the unbreakable cells;
    /// the spanned rows must, as a result, be laid out in the same region.
    pub(super) fn check_for_unbreakable_cells(&self, y: usize) -> usize {
        (0..self.grid.cols.len())
            .filter_map(|x| self.grid.cell(x, y))
            .filter(|cell| !cell.breakable)
            .map(|cell| self.grid.effective_rowspan_of_cell(cell))
            .max()
            .unwrap_or(0)
    }

    /// Used by `measure_auto_row` to gather data needed to measure the cell.
    pub(super) fn prepare_auto_row_cell_measurement(
        &self,
        parent: Axes<usize>,
        cell: &Cell,
        breakable: bool,
        row_group_data: Option<&UnbreakableRowGroup>,
    ) -> CellMeasurementData<'_> {
        let rowspan = self.grid.effective_rowspan_of_cell(cell);

        // This variable is used to construct a custom backlog if the cell
        // is a rowspan, or if headers or footers are used. When measuring, we
        // join the heights from previous regions to the current backlog to
        // form a rowspan's expected backlog. We also subtract the header's
        // and footer's heights from all regions.
        let mut custom_backlog: Vec<Abs> = vec![];

        // This function is used to subtract the expected header and footer
        // height from each upcoming region size in the current backlog and
        // last region.
        let mut subtract_header_footer_height_from_regions = || {
            // Only breakable auto rows need to update their backlogs based
            // on the presence of a header or footer, given that unbreakable
            // auto rows don't depend on the backlog, as they only span one
            // region.
            if breakable
                && (matches!(self.grid.header, Some(Repeatable::Repeated(_)))
                    || matches!(self.grid.footer, Some(Repeatable::Repeated(_))))
            {
                // Subtract header and footer height from all upcoming regions
                // when measuring the cell, including the last repeated region.
                //
                // This will update the 'custom_backlog' vector with the
                // updated heights of the upcoming regions.
                let mapped_regions = self.regions.map(&mut custom_backlog, |size| {
                    Size::new(size.x, size.y - self.header_height - self.footer_height)
                });

                // Callees must use the custom backlog instead of the current
                // backlog, so we return 'None'.
                return (None, mapped_regions.last);
            }

            // No need to change the backlog or last region.
            (Some(self.regions.backlog), self.regions.last)
        };

        // Each declaration, from top to bottom:
        // 1. The height available to the cell in the first region.
        // Usually, this will just be the size remaining in the current
        // region.
        // 2. The backlog of upcoming region heights to specify as
        // available to the cell.
        // 3. The full height of the first region of the cell.
        // 4. Height of the last repeated region to use in the measurement pod.
        // 5. The total height of the cell covered by previously spanned
        // rows in this region. This is used by rowspans to be able to tell
        // how much the auto row needs to expand.
        // 6. The amount of frames laid out by this cell in previous
        // regions. When the cell isn't a rowspan, this is always zero.
        // These frames are skipped after measuring.
        let height;
        let backlog;
        let full;
        let last;
        let height_in_this_region;
        let frames_in_previous_regions;

        if rowspan == 1 {
            // Not a rowspan, so the cell only occupies this row. Therefore:
            // 1. When we measure the cell below, use the available height
            // remaining in the region as the height it has available.
            // However, if the auto row is unbreakable, measure with infinite
            // height instead to see how much content expands.
            // 2. Use the region's backlog and last region when measuring,
            // however subtract the expected header and footer heights from
            // each upcoming size, if there is a header or footer.
            // 3. Use the same full region height.
            // 4. No height occupied by this cell in this region so far.
            // 5. Yes, this cell started in this region.
            height = if breakable { self.regions.size.y } else { Abs::inf() };
            (backlog, last) = subtract_header_footer_height_from_regions();
            full = if breakable { self.regions.full } else { Abs::inf() };
            height_in_this_region = Abs::zero();
            frames_in_previous_regions = 0;
        } else {
            // Height of the rowspan covered by spanned rows in the current
            // region.
            let laid_out_height: Abs = self
                .lrows
                .iter()
                .filter_map(|row| match row {
                    Row::Frame(frame, y, _)
                        if (parent.y..parent.y + rowspan).contains(y) =>
                    {
                        Some(frame.height())
                    }
                    // Either we have a row outside of the rowspan, or a
                    // fractional row, whose size we can't really guess.
                    _ => None,
                })
                .sum();

            // If we're currently simulating an unbreakable row group, also
            // consider the height of previously spanned rows which are in
            // the row group but not yet laid out.
            let unbreakable_height: Abs = row_group_data
                .into_iter()
                .flat_map(|row_group| &row_group.rows)
                .filter(|(y, _)| (parent.y..parent.y + rowspan).contains(y))
                .map(|(_, height)| height)
                .sum();

            height_in_this_region = laid_out_height + unbreakable_height;

            // Ensure we will measure the rowspan with the correct heights.
            // For that, we will gather the total height spanned by this
            // rowspan in previous regions.
            if let Some((rowspan_full, [rowspan_height, rowspan_other_heights @ ..])) =
                self.rowspans
                    .iter()
                    .find(|data| data.x == parent.x && data.y == parent.y)
                    .map(|data| (data.region_full, &*data.heights))
            {
                // The rowspan started in a previous region (as it already
                // has at least one region height).
                // Therefore, its initial height will be the height in its
                // first spanned region, and the backlog will be the
                // remaining heights, plus the current region's size, plus
                // the current backlog.
                frames_in_previous_regions = rowspan_other_heights.len() + 1;

                let heights_up_to_current_region = rowspan_other_heights
                    .iter()
                    .copied()
                    .chain(std::iter::once(if breakable {
                        self.initial.y - self.header_height - self.footer_height
                    } else {
                        // When measuring unbreakable auto rows, infinite
                        // height is available for content to expand.
                        Abs::inf()
                    }));

                custom_backlog = if breakable {
                    // This auto row is breakable. Therefore, join the
                    // rowspan's already laid out heights with the current
                    // region's height and current backlog to ensure a good
                    // level of accuracy in the measurements.
                    let backlog = self
                        .regions
                        .backlog
                        .iter()
                        .map(|&size| size - self.header_height - self.footer_height);

                    heights_up_to_current_region.chain(backlog).collect::<Vec<_>>()
                } else {
                    // No extra backlog if this is an unbreakable auto row.
                    // Ensure, when measuring, that the rowspan can be laid
                    // out through all spanned rows which were already laid
                    // out so far, but don't go further than this region.
                    heights_up_to_current_region.collect::<Vec<_>>()
                };

                height = *rowspan_height;
                backlog = None;
                full = rowspan_full;
                last = self
                    .regions
                    .last
                    .map(|size| size - self.header_height - self.footer_height);
            } else {
                // The rowspan started in the current region, as its vector
                // of heights in regions is currently empty.
                // Therefore, the initial height it has available will be
                // the current available size, plus the size spanned in
                // previous rows in this region (and/or unbreakable row
                // group, if it's being simulated).
                // The backlog and full will be that of the current region.
                // However, use infinite height instead if we're measuring an
                // unbreakable auto row.
                height = if breakable {
                    height_in_this_region + self.regions.size.y
                } else {
                    Abs::inf()
                };
                (backlog, last) = subtract_header_footer_height_from_regions();
                full = if breakable { self.regions.full } else { Abs::inf() };
                frames_in_previous_regions = 0;
            }
        }

        let width = self.cell_spanned_width(cell, parent.x);
        CellMeasurementData {
            width,
            height,
            backlog,
            custom_backlog,
            full,
            last,
            height_in_this_region,
            frames_in_previous_regions,
        }
    }

    /// Used in `measure_auto_row` to prepare a rowspan's `sizes` vector.
    /// Returns `true` if we'll need to run a simulation to more accurately
    /// expand the auto row based on the rowspan's demanded size, or `false`
    /// otherwise.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn prepare_rowspan_sizes(
        &self,
        auto_row_y: usize,
        sizes: &mut Vec<Abs>,
        cell: &Cell,
        parent_y: usize,
        rowspan: usize,
        unbreakable_rows_left: usize,
        measurement_data: &CellMeasurementData<'_>,
    ) -> bool {
        if sizes.len() <= 1
            && sizes.first().map_or(true, |&first_frame_size| {
                first_frame_size <= measurement_data.height_in_this_region
            })
        {
            // Ignore a rowspan fully covered by rows in previous
            // regions and/or in the current region.
            sizes.clear();
            return false;
        }
        if let Some(first_frame_size) = sizes.first_mut() {
            // Subtract already covered height from the size requested
            // by this rowspan to the auto row in the first region.
            *first_frame_size = (*first_frame_size
                - measurement_data.height_in_this_region)
                .max(Abs::zero());
        }

        let last_spanned_row = parent_y + rowspan - 1;

        // When the rowspan is unbreakable, or all of its upcoming
        // spanned rows are in the same unbreakable row group, its
        // spanned gutter will certainly be in the same region as all
        // of its other spanned rows, thus gutters won't be removed,
        // and we can safely reduce how much the auto row expands by
        // without using simulation.
        let is_effectively_unbreakable_rowspan =
            !cell.breakable || auto_row_y + unbreakable_rows_left > last_spanned_row;

        // If the rowspan doesn't end at this row and the grid has
        // gutter, we will need to run a simulation to find out how
        // much to expand this row by later. This is because gutters
        // spanned by this rowspan might be removed if they appear
        // around a pagebreak, so the auto row might have to expand a
        // bit more to compensate for the missing gutter height.
        // However, unbreakable rowspans aren't affected by that
        // problem.
        if auto_row_y != last_spanned_row
            && !sizes.is_empty()
            && self.grid.has_gutter
            && !is_effectively_unbreakable_rowspan
        {
            return true;
        }

        // We can only predict the resolved size of upcoming fixed-size
        // rows, but not fractional rows. In the future, we might be
        // able to simulate and circumvent the problem with fractional
        // rows. Relative rows are currently always measured relative
        // to the first region as well.
        // We can ignore auto rows since this is the last spanned auto
        // row.
        let will_be_covered_height: Abs = self
            .grid
            .rows
            .iter()
            .skip(auto_row_y + 1)
            .take(last_spanned_row - auto_row_y)
            .map(|row| match row {
                Sizing::Rel(v) => {
                    v.resolve(self.styles).relative_to(self.regions.base().y)
                }
                _ => Abs::zero(),
            })
            .sum();

        // Remove or reduce the sizes of the rowspan at the current or future
        // regions where it will already be covered by further rows spanned by
        // it.
        subtract_end_sizes(sizes, will_be_covered_height);

        // No need to run a simulation for this rowspan.
        false
    }

    /// Performs a simulation to predict by how much height the last spanned
    /// auto row will have to expand, given the current sizes of the auto row
    /// in each region and the pending rowspans' data (parent Y, rowspan amount
    /// and vector of requested sizes).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn simulate_and_measure_rowspans_in_auto_row(
        &self,
        y: usize,
        resolved: &mut Vec<Abs>,
        pending_rowspans: &[(usize, usize, Vec<Abs>)],
        unbreakable_rows_left: usize,
        row_group_data: Option<&UnbreakableRowGroup>,
        mut disambiguator: usize,
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
                // by this rowspan. No need to simulate this since the auto row
                // will already expand throughout already resolved regions.
                // Our simulation, therefore, won't otherwise change already
                // resolved sizes, other than, perhaps, the last one (at the
                // last currently resolved region, at which we can expand).
                target.set_max(*size);
            }
            for (simulated_target, rowspan_size) in
                simulated_sizes.iter_mut().zip(&mut sizes)
            {
                // The remaining sizes are exclusive to rowspans, since
                // other cells in this row didn't require as many regions.
                // We will perform a simulation to see how much of these sizes
                // does the auto row actually need to expand by, and how much
                // is already covered by upcoming rows spanned by the rowspans.
                simulated_target.set_max(*rowspan_size);
            }
            simulated_sizes.extend(sizes);
            max_spanned_row = max_spanned_row.max(parent_y + rowspan - 1);
        }
        if simulated_sizes.is_empty() && resolved.last() == last_resolved_size.as_ref() {
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
        // If we're currently inside an unbreakable row group simulation,
        // subtract the current row group height from the available space
        // when simulating rowspans in said group.
        let mut simulated_regions = self.regions;
        simulated_regions.size.y -=
            row_group_data.map_or(Abs::zero(), |row_group| row_group.height);

        for _ in 0..resolved.len() {
            // Ensure we start at the region where we will expand the auto
            // row.
            // Note that we won't accidentally call '.next()' once more than
            // desired (we won't skip the last resolved frame, where we will
            // expand) because we popped the last resolved size from the
            // resolved vector, above.
            simulated_regions.next();
            disambiguator += 1;

            // Subtract the initial header and footer height, since that's the
            // height we used when subtracting from the region backlog's
            // heights while measuring cells.
            simulated_regions.size.y -= self.header_height + self.footer_height;
        }

        if let Some(original_last_resolved_size) = last_resolved_size {
            // We're now at the (current) last region of this auto row.
            // Consider resolved height as already taken space.
            simulated_regions.size.y -= original_last_resolved_size;
        }

        // Now we run the simulation to check how much the auto row needs to
        // grow to ensure that rowspans have the height they need.
        let simulations_stabilized = self.run_rowspan_simulation(
            y,
            max_spanned_row,
            simulated_regions,
            &mut simulated_sizes,
            engine,
            last_resolved_size,
            unbreakable_rows_left,
            disambiguator,
        )?;

        if !simulations_stabilized {
            // If the simulation didn't stabilize above, we will just pretend
            // all gutters were removed, as a best effort. That means the auto
            // row will expand more than it normally should, but there isn't
            // much we can do.
            let will_be_covered_height = self
                .grid
                .rows
                .iter()
                .enumerate()
                .skip(y + 1)
                .take(max_spanned_row - y)
                .filter(|(y, _)| !self.grid.is_gutter_track(*y))
                .map(|(_, row)| match row {
                    Sizing::Rel(v) => {
                        v.resolve(self.styles).relative_to(self.regions.base().y)
                    }
                    _ => Abs::zero(),
                })
                .sum();

            subtract_end_sizes(&mut simulated_sizes, will_be_covered_height);
        }

        resolved.extend(simulated_sizes);

        Ok(())
    }

    /// Performs a simulation of laying out multiple rowspans (consolidated
    /// into a single vector of simulated sizes) ending in a certain auto row
    /// in order to find out how much the auto row will need to expand to cover
    /// the rowspans' requested sizes, considering how much size has been
    /// covered by other rows and by gutter between rows.
    ///
    /// For example, for a rowspan cell containing a block of 8pt of height
    /// spanning rows (1pt, auto, 0.5pt, 0.5pt), with a gutter of 1pt between
    /// each row, we have that the rows it spans provide 1pt + 0.5pt + 0.5pt
    /// = 2pt of height, plus 1pt + 1pt + 1pt = 3pt of gutter, with a total of
    /// 2pt + 3pt = 5pt of height already covered by fixed-size rows and
    /// gutters. This means that the auto row must (under normal conditions)
    /// expand by 3pt (8pt - 5pt) so that the rowspan has enough height across
    /// rows to fully draw its contents.
    ///
    /// However, it's possible that the last row is sent to the next page to
    /// respect a pagebreak, and then the 1pt gutter before it disappears. This
    /// would lead to our rowspan having a height of 7pt available if we fail
    /// to predict this situation when measuring the auto row.
    ///
    /// The algorithm below will, thus, attempt to simulate the layout of each
    /// spanned row, considering the space available in the current page and in
    /// upcoming pages (through the region backlog), in order to predict which
    /// rows will be sent to a new page and thus have their preceding gutter
    /// spacing removed (meaning the auto row has to grow a bit more). After
    /// simulating, we subtract the total height spanned by upcoming rows and
    /// gutter from the total rowspan height - this will be how much our auto
    /// row has to expand. We then simulate again to check if, if the auto row
    /// expanded by that amount, that would prompt the auto row to need to
    /// expand even more, because expanding the auto row might cause some other
    /// larger gutter spacing to disappear (leading to the rowspan having less
    /// space available instead of more); if so, we update the amount to expand
    /// and run the simulation again. Otherwise (if it should expand by the
    /// same amount, meaning we predicted correctly, or by less, meaning the
    /// auto row will be a bit larger than it should be, but that's a
    /// compromise we're willing to accept), we conclude the simulation
    /// (consider it stabilized) and return the result.
    ///
    /// Tries up to 5 times. If two consecutive simulations stabilize, then
    /// we subtract the predicted expansion height ('amount_to_grow') from the
    /// total height requested by rowspans (the 'requested_rowspan_height') to
    /// obtain how much height is covered by upcoming rows, according to our
    /// simulation, and the result of that operation is used to reduce or
    /// remove heights from the end of the vector of simulated sizes, such that
    /// the remaining heights are exactly how much the auto row should expand
    /// by. Then, we return `true`.
    ///
    /// If the simulations don't stabilize (they return 5 different and
    /// successively larger values), aborts and returns `false`.
    #[allow(clippy::too_many_arguments)]
    fn run_rowspan_simulation(
        &self,
        y: usize,
        max_spanned_row: usize,
        mut simulated_regions: Regions<'_>,
        simulated_sizes: &mut Vec<Abs>,
        engine: &mut Engine,
        last_resolved_size: Option<Abs>,
        unbreakable_rows_left: usize,
        mut disambiguator: usize,
    ) -> SourceResult<bool> {
        // The max amount this row can expand will be the total size requested
        // by rowspans which was not yet resolved. It is worth noting that,
        // earlier, we pushed the last resolved size to 'simulated_sizes' as
        // row expansion starts with it, so it's possible a rowspan requested
        // to extend that size (we will see, through the simulation, if that's
        // needed); however, we must subtract that resolved size from the total
        // sum of sizes, as it was already resolved and thus the auto row will
        // already grow by at least that much in the last resolved region (we
        // would grow by the same size twice otherwise).
        let requested_rowspan_height =
            simulated_sizes.iter().sum::<Abs>() - last_resolved_size.unwrap_or_default();

        // The amount the row will effectively grow by, according to the latest
        // simulation.
        let mut amount_to_grow = Abs::zero();

        // Try to simulate up to 5 times. If it doesn't stabilize at a value
        // which, when used and combined with upcoming spanned rows, covers all
        // of the requested rowspan height, we give up.
        for _attempt in 0..5 {
            let rowspan_simulator = RowspanSimulator::new(
                disambiguator,
                simulated_regions,
                self.header_height,
                self.footer_height,
            );

            let total_spanned_height = rowspan_simulator.simulate_rowspan_layout(
                y,
                max_spanned_row,
                amount_to_grow,
                requested_rowspan_height,
                unbreakable_rows_left,
                self,
                engine,
            )?;

            // If the total height spanned by upcoming spanned rows plus the
            // current amount we predict the auto row will have to grow (from
            // the previous iteration) are larger than the size requested by
            // rowspans, this means the auto row will grow enough in order to
            // cover the requested rowspan height, so we stop the simulation.
            //
            // If that's not yet the case, we will simulate again and make the
            // auto row grow even more, and do so until either the auto row has
            // grown enough, or we tried to do so over 5 times.
            //
            // A flaw of this approach is that we consider rowspans' content to
            // be contiguous. That is, we treat rowspans' requested heights as
            // a simple number, instead of properly using the vector of
            // requested heights in each region. This can lead to some
            // weirdness when using multi-page rowspans with content that
            // reacts to the amount of space available, including paragraphs.
            // However, this is probably the best we can do for now.
            if (total_spanned_height + amount_to_grow).fits(requested_rowspan_height) {
                // Reduce sizes by the amount to be covered by upcoming spanned
                // rows, which is equivalent to the amount that we don't grow.
                // We reduce from the end as that's where the spanned rows will
                // cover. The remaining sizes will all be covered by the auto
                // row instead (which will grow by those sizes).
                subtract_end_sizes(
                    simulated_sizes,
                    requested_rowspan_height - amount_to_grow,
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

                return Ok(true);
            }

            // For the next simulation, we will test if the auto row can grow
            // by precisely how much rowspan height is not covered by upcoming
            // spanned rows, according to the current simulation.
            // We know that the new amount to grow is larger (and thus the
            // auto row only expands between each simulation), because we
            // checked above if
            // 'total_spanned_height + (now old_)amount_to_grow >= requested_rowspan_height',
            // which was false, so it holds that
            // 'total_spanned_height + old_amount_to_grow < requested_rowspan_height'
            // Thus,
            // 'old_amount_to_grow < requested_rowspan_height - total_spanned_height'
            // Therefore, by definition, 'old_amount_to_grow < amount_to_grow'.
            let old_amount_to_grow = std::mem::replace(
                &mut amount_to_grow,
                requested_rowspan_height - total_spanned_height,
            );

            // We advance the 'regions' variable accordingly, so that, in the
            // next simulation, we consider already grown space as final.
            // That is, we effectively simulate how rows would be placed if the
            // auto row grew by precisely the new value of 'amount_to_grow'.
            let mut extra_amount_to_grow = amount_to_grow - old_amount_to_grow;
            while extra_amount_to_grow > Abs::zero()
                && simulated_regions.size.y < extra_amount_to_grow
            {
                extra_amount_to_grow -= simulated_regions.size.y.max(Abs::zero());
                simulated_regions.next();
                simulated_regions.size.y -= self.header_height + self.footer_height;
                disambiguator += 1;
            }
            simulated_regions.size.y -= extra_amount_to_grow;
        }

        // Simulation didn't succeed in 5 attempts.
        Ok(false)
    }
}

/// Auxiliary structure holding state during rowspan simulation.
struct RowspanSimulator<'a> {
    /// The number of finished regions.
    finished: usize,
    /// The state of regions during the simulation.
    regions: Regions<'a>,
    /// The height of the header in the currently simulated region.
    header_height: Abs,
    /// The height of the footer in the currently simulated region.
    footer_height: Abs,
    /// The total spanned height so far in the simulation.
    total_spanned_height: Abs,
    /// Height of the latest spanned gutter row in the simulation.
    /// Zero if it was removed.
    latest_spanned_gutter_height: Abs,
}

impl<'a> RowspanSimulator<'a> {
    /// Creates new rowspan simulation state with the given regions and initial
    /// header and footer heights. Other fields should always start as zero.
    fn new(
        finished: usize,
        regions: Regions<'a>,
        header_height: Abs,
        footer_height: Abs,
    ) -> Self {
        Self {
            finished,
            regions,
            header_height,
            footer_height,
            total_spanned_height: Abs::zero(),
            latest_spanned_gutter_height: Abs::zero(),
        }
    }

    /// Calculates the total spanned height of the rowspan.
    /// Stops calculating if, at any point in the simulation, the value of
    /// `total_spanned_height + amount_to_grow` becomes larger than
    /// `requested_rowspan_height`, as the results are not going to become any
    /// more useful after that point.
    #[allow(clippy::too_many_arguments)]
    fn simulate_rowspan_layout(
        mut self,
        y: usize,
        max_spanned_row: usize,
        amount_to_grow: Abs,
        requested_rowspan_height: Abs,
        mut unbreakable_rows_left: usize,
        layouter: &GridLayouter<'_>,
        engine: &mut Engine,
    ) -> SourceResult<Abs> {
        let spanned_rows = &layouter.grid.rows[y + 1..=max_spanned_row];
        for (offset, row) in spanned_rows.iter().enumerate() {
            if (self.total_spanned_height + amount_to_grow).fits(requested_rowspan_height)
            {
                // Stop the simulation, as the combination of upcoming
                // spanned rows (so far) and the current amount the auto
                // row expands by has already fully covered the height the
                // rowspans need.
                return Ok(self.total_spanned_height);
            }
            let spanned_y = y + 1 + offset;
            let is_gutter = layouter.grid.is_gutter_track(spanned_y);

            if unbreakable_rows_left == 0 {
                // Simulate unbreakable row groups, and skip regions until
                // they fit. There is no risk of infinite recursion, as
                // no auto rows participate in the simulation, so the
                // unbreakable row group simulator won't recursively call
                // 'measure_auto_row' or (consequently) this function.
                let row_group = layouter.simulate_unbreakable_row_group(
                    spanned_y,
                    None,
                    &self.regions,
                    engine,
                    0,
                )?;
                while !self.regions.size.y.fits(row_group.height)
                    && !in_last_with_offset(
                        self.regions,
                        self.header_height + self.footer_height,
                    )
                {
                    self.finish_region(layouter, engine)?;
                }

                unbreakable_rows_left = row_group.rows.len();
            }

            match row {
                // Fixed-size spanned rows are what we are interested in.
                // They contribute a fixed amount of height to our rowspan.
                Sizing::Rel(v) => {
                    let height =
                        v.resolve(layouter.styles).relative_to(self.regions.base().y);
                    self.total_spanned_height += height;
                    if is_gutter {
                        self.latest_spanned_gutter_height = height;
                    }

                    let mut skipped_region = false;
                    while unbreakable_rows_left == 0
                        && !self.regions.size.y.fits(height)
                        && !in_last_with_offset(
                            self.regions,
                            self.header_height + self.footer_height,
                        )
                    {
                        self.finish_region(layouter, engine)?;

                        skipped_region = true;
                    }

                    if !skipped_region || !is_gutter {
                        // No gutter at the top of a new region, so don't
                        // account for it if we just skipped a region.
                        self.regions.size.y -= height;
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
                    self.latest_spanned_gutter_height = Abs::zero();
                }
                Sizing::Fr(_) => {}
            }

            unbreakable_rows_left = unbreakable_rows_left.saturating_sub(1);
        }

        Ok(self.total_spanned_height)
    }

    fn simulate_header_footer_layout(
        &mut self,
        layouter: &GridLayouter<'_>,
        engine: &mut Engine,
    ) -> SourceResult<()> {
        // We can't just use the initial header/footer height on each region,
        // because header/footer height might vary depending on region size if
        // it contains rows with relative lengths. Therefore, we re-simulate
        // headers and footers on each new region.
        // It's true that, when measuring cells, we reduce each height in the
        // backlog to consider the initial header and footer heights; however,
        // our simulation checks what happens AFTER the auto row, so we can
        // just use the original backlog from `self.regions`.
        let disambiguator = self.finished;
        let header_height =
            if let Some(Repeatable::Repeated(header)) = &layouter.grid.header {
                layouter
                    .simulate_header(header, &self.regions, engine, disambiguator)?
                    .height
            } else {
                Abs::zero()
            };

        let footer_height =
            if let Some(Repeatable::Repeated(footer)) = &layouter.grid.footer {
                layouter
                    .simulate_footer(footer, &self.regions, engine, disambiguator)?
                    .height
            } else {
                Abs::zero()
            };

        let mut skipped_region = false;

        // Skip until we reach a fitting region for both header and footer.
        while !self.regions.size.y.fits(header_height + footer_height)
            && self.regions.may_progress()
        {
            self.regions.next();
            self.finished += 1;
            skipped_region = true;
        }

        if let Some(Repeatable::Repeated(header)) = &layouter.grid.header {
            self.header_height = if skipped_region {
                // Simulate headers again, at the new region, as
                // the full region height may change.
                layouter
                    .simulate_header(header, &self.regions, engine, disambiguator)?
                    .height
            } else {
                header_height
            };
        }

        if let Some(Repeatable::Repeated(footer)) = &layouter.grid.footer {
            self.footer_height = if skipped_region {
                // Simulate footers again, at the new region, as
                // the full region height may change.
                layouter
                    .simulate_footer(footer, &self.regions, engine, disambiguator)?
                    .height
            } else {
                footer_height
            };
        }

        // Consume the header's and footer's heights from the new region,
        // but don't consider them spanned. The rowspan does not go over the
        // header or footer (as an invariant, any rowspans spanning any header
        // or footer rows are fully contained within that header's or footer's rows).
        self.regions.size.y -= self.header_height + self.footer_height;

        Ok(())
    }

    fn finish_region(
        &mut self,
        layouter: &GridLayouter<'_>,
        engine: &mut Engine,
    ) -> SourceResult<()> {
        // If a row was pushed to the next region, the immediately
        // preceding gutter row is removed.
        self.total_spanned_height -= self.latest_spanned_gutter_height;
        self.latest_spanned_gutter_height = Abs::zero();
        self.regions.next();
        self.finished += 1;

        self.simulate_header_footer_layout(layouter, engine)
    }
}

/// Subtracts some size from the end of a vector of sizes.
/// For example, subtracting 5pt from \[2pt, 1pt, 3pt\] will result in \[1pt\].
fn subtract_end_sizes(sizes: &mut Vec<Abs>, mut subtract: Abs) {
    while subtract > Abs::zero() && sizes.last().is_some_and(|&size| size <= subtract) {
        subtract -= sizes.pop().unwrap();
    }
    if subtract > Abs::zero() {
        if let Some(last_size) = sizes.last_mut() {
            *last_size -= subtract;
        }
    }
}
