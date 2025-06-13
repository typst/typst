use std::ops::Deref;

use typst_library::diag::SourceResult;
use typst_library::engine::Engine;
use typst_library::layout::grid::resolve::{Footer, Header, Repeatable};
use typst_library::layout::{Abs, Axes, Frame, Regions};

use super::layouter::{GridLayouter, RowState};
use super::rowspans::UnbreakableRowGroup;

impl<'a> GridLayouter<'a> {
    /// Checks whether a region break could help a situation where we're out of
    /// space for the next row. The criteria are:
    ///
    /// 1. If we could progress at the top of the region, that indicates the
    ///    region has a backlog, or (if we're at the first region) a region break
    ///    is at all possible (`regions.last` is `Some()`), so that's sufficient.
    ///
    /// 2. Otherwise, we may progress if another region break is possible
    ///    (`regions.last` is still `Some()`) and non-repeating rows have been
    ///    placed, since that means the space they occupy will be available in the
    ///    next region.
    #[inline]
    pub fn may_progress_with_repeats(&self) -> bool {
        // TODO(subfooters): check below isn't enough to detect non-repeating
        // footers... we can also change 'initial_after_repeats' to stop being
        // calculated if there were any non-repeating footers.
        self.current.could_progress_at_top
            || self.regions.last.is_some()
                && self.regions.size.y != self.current.initial_after_repeats
    }

    pub fn place_new_headers(
        &mut self,
        consecutive_header_count: &mut usize,
        engine: &mut Engine,
    ) -> SourceResult<()> {
        *consecutive_header_count += 1;
        let (consecutive_headers, new_upcoming_headers) =
            self.upcoming_headers.split_at(*consecutive_header_count);

        if new_upcoming_headers.first().is_some_and(|next_header| {
            consecutive_headers.last().is_none_or(|latest_header| {
                !latest_header.short_lived
                    && next_header.range.start == latest_header.range.end
            }) && !next_header.short_lived
        }) {
            // More headers coming, so wait until we reach them.
            return Ok(());
        }

        self.upcoming_headers = new_upcoming_headers;
        *consecutive_header_count = 0;

        let [first_header, ..] = consecutive_headers else {
            self.flush_orphans();
            return Ok(());
        };

        // Assuming non-conflicting headers sorted by increasing y, this must
        // be the header with the lowest level (sorted by increasing levels).
        let first_level = first_header.level;

        // Stop repeating conflicting headers, even if the new headers are
        // short-lived or won't repeat.
        //
        // If we go to a new region before the new headers fit alongside their
        // children (or in general, for short-lived), the old headers should
        // not be displayed anymore.
        let first_conflicting_pos =
            self.repeating_headers.partition_point(|h| h.level < first_level);
        self.repeating_headers.truncate(first_conflicting_pos);

        // Ensure upcoming rows won't see that these headers will occupy any
        // space in future regions anymore.
        for removed_height in
            self.current.repeating_header_heights.drain(first_conflicting_pos..)
        {
            self.current.repeating_header_height -= removed_height;
        }

        // Layout short-lived headers immediately.
        if consecutive_headers.last().is_some_and(|h| h.short_lived) {
            // No chance of orphans as we're immediately placing conflicting
            // headers afterwards, which basically are not headers, for all intents
            // and purposes. It is therefore guaranteed that all new headers have
            // been placed at least once.
            self.flush_orphans();

            // Layout each conflicting header independently, without orphan
            // prevention (as they don't go into 'pending_headers').
            // These headers are short-lived as they are immediately followed by a
            // header of the same or lower level, such that they never actually get
            // to repeat.
            self.layout_new_headers(consecutive_headers, true, engine)?;
        } else {
            // Let's try to place pending headers at least once.
            // This might be a waste as we could generate an orphan and thus have
            // to try to place old and new headers all over again, but that happens
            // for every new region anyway, so it's rather unavoidable.
            let snapshot_created =
                self.layout_new_headers(consecutive_headers, false, engine)?;

            // Queue the new headers for layout. They will remain in this
            // vector due to orphan prevention.
            //
            // After the first subsequent row is laid out, move to repeating, as
            // it's then confirmed the headers won't be moved due to orphan
            // prevention anymore.
            self.pending_headers = consecutive_headers;

            if !snapshot_created {
                // Region probably couldn't progress.
                //
                // Mark new pending headers as final and ensure there isn't a
                // snapshot.
                self.flush_orphans();
            }
        }

        Ok(())
    }

    /// Lays out rows belonging to a header, returning the calculated header
    /// height only for that header. Indicates to the laid out rows that they
    /// should inform their laid out heights if appropriate (auto or fixed
    /// size rows only).
    #[inline]
    fn layout_header_rows(
        &mut self,
        header: &Header,
        engine: &mut Engine,
        disambiguator: usize,
        as_short_lived: bool,
    ) -> SourceResult<Abs> {
        let mut header_height = Abs::zero();
        for y in header.range.clone() {
            header_height += self
                .layout_row_with_state(
                    y,
                    engine,
                    disambiguator,
                    RowState {
                        current_row_height: Some(Abs::zero()),
                        in_active_repeatable: !as_short_lived,
                    },
                )?
                .current_row_height
                .unwrap_or_default();
        }
        Ok(header_height)
    }

    /// This function should be called each time an additional row has been
    /// laid out in a region to indicate that orphan prevention has succeeded.
    ///
    /// It removes the current orphan snapshot and flushes pending headers,
    /// such that a non-repeating header won't try to be laid out again
    /// anymore, and a repeating header will begin to be part of
    /// `repeating_headers`.
    pub fn flush_orphans(&mut self) {
        self.current.lrows_orphan_snapshot = None;
        self.flush_pending_headers();
    }

    /// Indicates all currently pending headers have been successfully placed
    /// once, since another row has been placed after them, so they are
    /// certainly not orphans.
    pub fn flush_pending_headers(&mut self) {
        if self.pending_headers.is_empty() {
            return;
        }

        for header in self.pending_headers {
            if header.repeated {
                // Vector remains sorted by increasing levels:
                // - 'pending_headers' themselves are sorted, since we only
                // push non-mutually-conflicting headers at a time.
                // - Before pushing new pending headers in
                // 'layout_new_pending_headers', we truncate repeating headers
                // to remove anything with the same or higher levels as the
                // first pending header.
                // - Assuming it was sorted before, that truncation only keeps
                // elements with a lower level.
                // - Therefore, by pushing this header to the end, it will have
                // a level larger than all the previous headers, and is thus
                // in its 'correct' position.
                self.repeating_headers.push(header);
            }
        }

        self.pending_headers = Default::default();
    }

    /// Lays out the rows of repeating and pending headers at the top of the
    /// region.
    ///
    /// Assumes the footer height for the current region has already been
    /// calculated. Skips regions as necessary to fit all headers and all
    /// footers.
    pub fn layout_active_headers(&mut self, engine: &mut Engine) -> SourceResult<()> {
        // Generate different locations for content in headers across its
        // repetitions by assigning a unique number for each one.
        let disambiguator = self.finished.len();

        let header_height = self.simulate_header_height(
            self.repeating_headers
                .iter()
                .copied()
                .chain(self.pending_headers.iter().map(Repeatable::deref)),
            &self.regions,
            engine,
            disambiguator,
        )?;

        // We already take the footer into account below.
        // While skipping regions, footer height won't be automatically
        // re-calculated until the end.
        let mut skipped_region = false;
        while self.unbreakable_rows_left == 0
            && !self.regions.size.y.fits(header_height)
            && self.may_progress_with_repeats()
        {
            // Advance regions without any output until we can place the
            // header and the footer.
            self.finish_region_internal(
                Frame::soft(Axes::splat(Abs::zero())),
                vec![],
                Default::default(),
            );

            // TODO(layout model): re-calculate heights of headers and footers
            // on each region if 'full' changes? (Assuming height doesn't
            // change for now...)
            //
            // Would remove the footer height update below (move it here).
            skipped_region = true;

            self.regions.size.y -= self.current.footer_height;
            self.current.initial_after_repeats = self.regions.size.y;
        }

        if !self.repeating_footers.is_empty() && skipped_region {
            // Simulate the footer again; the region's 'full' might have
            // changed.
            let (footer_height, footer_heights) = self.simulate_footer_heights(
                self.repeating_footers.iter().map(|x| *x),
                &self.regions,
                engine,
                disambiguator,
            )?;

            self.current.footer_height = footer_height;
            self.current.repeating_footer_heights.extend(footer_heights);
        }

        let repeating_header_rows =
            total_header_row_count(self.repeating_headers.iter().copied());

        let pending_header_rows =
            total_header_row_count(self.pending_headers.iter().map(Repeatable::deref));

        // Group of headers is unbreakable.
        // Thus, no risk of 'finish_region' being recursively called from
        // within 'layout_row'.
        self.unbreakable_rows_left += repeating_header_rows + pending_header_rows;

        self.current.last_repeated_header_end =
            self.repeating_headers.last().map(|h| h.range.end).unwrap_or_default();

        // Reset the header height for this region.
        // It will be re-calculated when laying out each header row.
        self.current.repeating_header_height = Abs::zero();
        self.current.repeating_header_heights.clear();

        debug_assert!(self.current.lrows.is_empty());
        debug_assert!(self.current.lrows_orphan_snapshot.is_none());
        let may_progress = self.may_progress_with_repeats();

        if may_progress {
            // Enable orphan prevention for headers at the top of the region.
            // Otherwise, we will flush pending headers below, after laying
            // them out.
            //
            // It is very rare for this to make a difference as we're usually
            // at the 'last' region after the first skip, at which the snapshot
            // is handled by 'layout_new_headers'. Either way, we keep this
            // here for correctness.
            self.current.lrows_orphan_snapshot = Some(self.current.lrows.len());
        }

        // Use indices to avoid double borrow. We don't mutate headers in
        // 'layout_row' so this is fine.
        let mut i = 0;
        while let Some(&header) = self.repeating_headers.get(i) {
            let header_height =
                self.layout_header_rows(header, engine, disambiguator, false)?;
            self.current.repeating_header_height += header_height;

            // We assume that this vector will be sorted according
            // to increasing levels like 'repeating_headers' and
            // 'pending_headers' - and, in particular, their union, as this
            // vector is pushed repeating heights from both.
            //
            // This is guaranteed by:
            // 1. We always push pending headers after repeating headers,
            // as we assume they don't conflict because we remove
            // conflicting repeating headers when pushing a new pending
            // header.
            //
            // 2. We push in the same order as each.
            //
            // 3. This vector is also modified when pushing a new pending
            // header, where we remove heights for conflicting repeating
            // headers which have now stopped repeating. They are always at
            // the end and new pending headers respect the existing sort,
            // so the vector will remain sorted.
            self.current.repeating_header_heights.push(header_height);

            i += 1;
        }

        self.current.repeated_header_rows = self.current.lrows.len();
        self.current.initial_after_repeats = self.regions.size.y;

        let mut has_non_repeated_pending_header = false;
        for header in self.pending_headers {
            if !header.repeated {
                self.current.initial_after_repeats = self.regions.size.y;
                has_non_repeated_pending_header = true;
            }
            let header_height =
                self.layout_header_rows(header, engine, disambiguator, false)?;
            if header.repeated {
                self.current.repeating_header_height += header_height;
                self.current.repeating_header_heights.push(header_height);
            }
        }

        if !has_non_repeated_pending_header {
            self.current.initial_after_repeats = self.regions.size.y;
        }

        if !may_progress {
            // Flush pending headers immediately, as placing them again later
            // won't help.
            self.flush_orphans();
        }

        Ok(())
    }

    /// Lays out headers found for the first time during row layout.
    ///
    /// If 'short_lived' is true, these headers are immediately followed by
    /// a conflicting header, so it is assumed they will not be pushed to
    /// pending headers.
    ///
    /// Returns whether orphan prevention was successfully setup, or couldn't
    /// due to short-lived headers or the region couldn't progress.
    pub fn layout_new_headers(
        &mut self,
        headers: &'a [Repeatable<Header>],
        short_lived: bool,
        engine: &mut Engine,
    ) -> SourceResult<bool> {
        // At first, only consider the height of the given headers. However,
        // for upcoming regions, we will have to consider repeating headers as
        // well.
        let header_height = self.simulate_header_height(
            headers.iter().map(Repeatable::deref),
            &self.regions,
            engine,
            0,
        )?;

        while self.unbreakable_rows_left == 0
            && !self.regions.size.y.fits(header_height)
            && self.may_progress_with_repeats()
        {
            // Note that, after the first region skip, the new headers will go
            // at the top of the region, but after the repeating headers that
            // remained (which will be automatically placed in 'finish_region').
            self.finish_region(engine, false)?;
        }

        // Remove new headers at the end of the region if the upcoming row
        // doesn't fit.
        // TODO(subfooters): what if there is a footer right after it?
        let should_snapshot = !short_lived
            && self.current.lrows_orphan_snapshot.is_none()
            && self.may_progress_with_repeats();

        if should_snapshot {
            // If we don't enter this branch while laying out non-short lived
            // headers, that means we will have to immediately flush pending
            // headers and mark them as final, since trying to place them in
            // the next page won't help get more space.
            self.current.lrows_orphan_snapshot = Some(self.current.lrows.len());
        }

        let mut at_top = self.regions.size.y == self.current.initial_after_repeats;

        self.unbreakable_rows_left +=
            total_header_row_count(headers.iter().map(Repeatable::deref));

        for header in headers {
            let header_height = self.layout_header_rows(header, engine, 0, false)?;

            // Only store this header height if it is actually going to
            // become a pending header. Otherwise, pretend it's not a
            // header... This is fine for consumers of 'header_height' as
            // it is guaranteed this header won't appear in a future
            // region, so multi-page rows and cells can effectively ignore
            // this header.
            if !short_lived && header.repeated {
                self.current.repeating_header_height += header_height;
                self.current.repeating_header_heights.push(header_height);
                if at_top {
                    self.current.initial_after_repeats = self.regions.size.y;
                }
            } else {
                at_top = false;
            }
        }

        Ok(should_snapshot)
    }

    /// Calculates the total expected height of several headers.
    pub fn simulate_header_height<'h: 'a>(
        &self,
        headers: impl IntoIterator<Item = &'h Header>,
        regions: &Regions<'_>,
        engine: &mut Engine,
        disambiguator: usize,
    ) -> SourceResult<Abs> {
        let mut height = Abs::zero();
        for header in headers {
            height +=
                self.simulate_header(header, regions, engine, disambiguator)?.height;
        }
        Ok(height)
    }

    /// Simulate the header's group of rows.
    pub fn simulate_header(
        &self,
        header: &Header,
        regions: &Regions<'_>,
        engine: &mut Engine,
        disambiguator: usize,
    ) -> SourceResult<UnbreakableRowGroup> {
        // Note that we assume the invariant that any rowspan in a header is
        // fully contained within that header. Therefore, there won't be any
        // unbreakable rowspans exceeding the header's rows, and we can safely
        // assume that the amount of unbreakable rows following the first row
        // in the header will be precisely the rows in the header.
        self.simulate_unbreakable_row_group(
            header.range.start,
            Some(header.range.end - header.range.start),
            regions,
            engine,
            disambiguator,
        )
    }

    /// Place a footer we have reached through normal row layout.
    pub fn place_new_footer(
        &mut self,
        engine: &mut Engine,
        footer: &Repeatable<Footer>,
    ) -> SourceResult<()> {
        // TODO(subfooters): short-lived check
        if !footer.repeated {
            // TODO(subfooters): widow prevention for this.
            // Will need some lookahead. For now, act as short-lived.
            let footer_height =
                self.simulate_footer(footer, &self.regions, engine, 0)?.height;

            // Skip to fitting region where only this footer fits.
            while self.unbreakable_rows_left == 0
                && !self.regions.size.y.fits(footer_height)
                && self.may_progress_with_repeats()
            {
                // Advance regions until we can place the footer.
                // Treat as a normal row group.
                self.finish_region(engine, false)?;
            }

            self.layout_footer(footer, true, engine, 0)?;
        } else {
            // Placing a non-short-lived repeating footer, so it must be
            // the latest one in the repeating footers vector.
            let latest_repeating_footer = self.repeating_footers.pop().unwrap();
            assert_eq!(latest_repeating_footer.range.start, footer.range.start);

            let expected_footer_height =
                self.current.repeating_footer_heights.pop().unwrap();

            // Ensure upcoming rows won't see that this footer will occupy
            // any space in future regions anymore.
            self.current.footer_height -= expected_footer_height;

            // Ensure footer rows have their own expected height
            // available. While not that relevant for them, as they will be
            // laid out as an unbreakable row group, it's relevant for any
            // further rows in the same region.
            self.regions.size.y += expected_footer_height;

            self.layout_footer(footer, false, engine, self.finished.len())?;
        }

        // If the next group of footers would conflict with other repeating
        // footers, wait for them to finish repeating before adding more to
        // repeat.
        if self.repeating_footers.is_empty()
            || self
                .upcoming_sorted_footers
                .first()
                .is_none_or(|f| f.level >= footer.level)
        {
            self.prepare_next_repeating_footers(false, engine)?;
            return Ok(());
        }

        Ok(())
    }

    /// Takes all non-conflicting consecutive footers which are about to start
    /// repeating, skips to the first region where they all fit, and pushes
    /// them to `repeating_footers`, sorted by ascending levels.
    pub fn prepare_next_repeating_footers(
        &mut self,
        first_footers: bool,
        engine: &mut Engine,
    ) -> SourceResult<()> {
        let [next_footer, other_footers @ ..] = self.upcoming_sorted_footers else {
            // No footers to take.
            return Ok(());
        };

        // TODO(subfooters): also ignore short-lived footers.
        if !next_footer.repeated {
            // Skip this footer and don't do anything until we get to it.
            //
            // TODO(subfooters): grouping and laying out non-repeated with
            // repeated, with widow prevention.
            self.upcoming_sorted_footers = other_footers;
            return Ok(());
        }

        let first_conflicting_index = other_footers
            .iter()
            .take_while(|f| f.level > next_footer.level)
            .count()
            + 1;

        let (next_repeating_footers, new_upcoming_footers) =
            self.upcoming_sorted_footers.split_at(first_conflicting_index);

        self.upcoming_sorted_footers = new_upcoming_footers;
        self.prepare_repeating_footers(
            next_repeating_footers.iter().map(Repeatable::deref),
            first_footers,
            engine,
            0,
        )?;

        self.repeating_footers
            .extend(next_repeating_footers.iter().filter_map(Repeatable::as_repeated));

        Ok(())
    }

    /// Updates `self.current.repeating_footer_height` by simulating repeating
    /// footers, and skips to fitting region.
    pub fn prepare_repeating_footers(
        &mut self,
        footers: impl Iterator<Item = &'a Footer> + ExactSizeIterator + Clone,
        at_region_top: bool,
        engine: &mut Engine,
        disambiguator: usize,
    ) -> SourceResult<()> {
        let (mut expected_footer_height, mut expected_footer_heights) = self
            .simulate_footer_heights(
                footers.clone(),
                &self.regions,
                engine,
                disambiguator,
            )?;

        // Skip to fitting region where all of them fit at once.
        //
        // Can't be widows: they are assumed to not be short-lived, so
        // there is at least one non-footer before them, and this
        // function is called right after placing a new footer, but
        // before the next non-footer, or at the top of the region,
        // at which point we haven't reached the row before the highest
        // level footer yet since the footer itself won't cause a
        // region break.
        let mut skipped_region = false;
        while self.unbreakable_rows_left == 0
            && !self.regions.size.y.fits(expected_footer_height)
            && self.regions.may_progress()
        {
            // Advance regions without any output until we can place the
            // footer.
            if at_region_top {
                self.finish_region_internal(
                    Frame::soft(Axes::splat(Abs::zero())),
                    vec![],
                    Default::default(),
                );
            } else {
                self.finish_region(engine, false)?;
            }
            skipped_region = true;
        }

        if skipped_region {
            // Simulate the footer again; the region's 'full' might have
            // changed, and the vector of heights was cleared.
            (expected_footer_height, expected_footer_heights) = self
                .simulate_footer_heights(footers, &self.regions, engine, disambiguator)?;
        }

        self.current.footer_height += expected_footer_height;
        self.current.repeating_footer_heights.extend(expected_footer_heights);

        Ok(())
    }

    pub fn simulate_footer_heights(
        &self,
        footers: impl Iterator<Item = &'a Footer> + ExactSizeIterator,
        regions: &Regions<'_>,
        engine: &mut Engine,
        disambiguator: usize,
    ) -> SourceResult<(Abs, Vec<Abs>)> {
        let mut total_footer_height = Abs::zero();
        let mut footer_heights = Vec::with_capacity(footers.len());
        for footer in footers {
            let footer_height =
                self.simulate_footer(footer, regions, engine, disambiguator)?.height;

            total_footer_height += footer_height;
            footer_heights.push(footer_height);
        }

        Ok((total_footer_height, footer_heights))
    }

    /// Lays out all rows in the footer.
    /// They are unbreakable.
    pub fn layout_footer(
        &mut self,
        footer: &Footer,
        as_short_lived: bool,
        engine: &mut Engine,
        disambiguator: usize,
    ) -> SourceResult<()> {
        let footer_len = footer.range.end - footer.range.start;
        self.unbreakable_rows_left += footer_len;

        // TODO(subfooters): also consider omitted gutter before the footer
        // when there is a header right before it taking it.
        for y in footer.range.clone() {
            self.layout_row_with_state(
                y,
                engine,
                disambiguator,
                RowState {
                    in_active_repeatable: !as_short_lived,
                    ..Default::default()
                },
            )?;
        }

        Ok(())
    }

    // Simulate the footer's group of rows.
    pub fn simulate_footer(
        &self,
        footer: &Footer,
        regions: &Regions<'_>,
        engine: &mut Engine,
        disambiguator: usize,
    ) -> SourceResult<UnbreakableRowGroup> {
        // Note that we assume the invariant that any rowspan in a footer is
        // fully contained within that footer. Therefore, there won't be any
        // unbreakable rowspans exceeding the footer's rows, and we can safely
        // assume that the amount of unbreakable rows following the first row
        // in the footer will be precisely the rows in the footer.
        self.simulate_unbreakable_row_group(
            footer.range.start,
            Some(footer.range.end - footer.range.start),
            regions,
            engine,
            disambiguator,
        )
    }
}

/// The total amount of rows in the given list of headers.
#[inline]
pub fn total_header_row_count<'h>(
    headers: impl IntoIterator<Item = &'h Header>,
) -> usize {
    headers.into_iter().map(|h| h.range.end - h.range.start).sum()
}
