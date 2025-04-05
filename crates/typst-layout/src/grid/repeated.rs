use std::ops::Deref;

use typst_library::diag::SourceResult;
use typst_library::engine::Engine;
use typst_library::layout::grid::resolve::{Footer, Header, Repeatable};
use typst_library::layout::{Abs, Axes, Frame, Regions};

use super::layouter::GridLayouter;
use super::rowspans::UnbreakableRowGroup;

pub enum HeadersToLayout<'a> {
    RepeatingAndPending,
    NewHeaders(&'a [Repeatable<Header>]),
}

impl<'a> GridLayouter<'a> {
    pub fn place_new_headers(
        &mut self,
        first_header: &Repeatable<Header>,
        consecutive_header_count: usize,
        engine: &mut Engine,
    ) -> SourceResult<()> {
        // Next row either isn't a header. or is in a
        // conflicting one, which is the sign that we need to go.
        let (consecutive_headers, new_upcoming_headers) =
            self.upcoming_headers.split_at(consecutive_header_count);
        self.upcoming_headers = new_upcoming_headers;

        let (non_conflicting_headers, conflicting_headers) = match self
            .upcoming_headers
            .get(consecutive_header_count)
            .map(Repeatable::unwrap)
        {
            Some(next_header) if next_header.level <= first_header.unwrap().level => {
                // All immediately conflicting headers will
                // be placed as normal rows.
                consecutive_headers.split_at(
                    consecutive_headers
                        .partition_point(|h| next_header.level > h.unwrap().level),
                )
            }
            _ => (consecutive_headers, Default::default()),
        };

        self.layout_new_pending_headers(non_conflicting_headers, engine);

        // Layout each conflicting header independently, without orphan
        // prevention (as they don't go into 'pending_headers').
        // These headers are short-lived as they are immediately followed by a
        // header of the same or lower level, such that they never actually get
        // to repeat.
        for conflicting_header in conflicting_headers.chunks_exact(1) {
            self.layout_headers(
                // Using 'chunks_exact", we pass a slice of length one instead
                // of a reference for type consistency.
                HeadersToLayout::NewHeaders(conflicting_header),
                engine,
            )?
        }

        Ok(())
    }

    /// Queues new pending headers for layout. Headers remain pending until
    /// they are successfully laid out in some page once. Then, they will be
    /// moved to `repeating_headers`, at which point it is safe to stop them
    /// from repeating at any time.
    fn layout_new_pending_headers(
        &mut self,
        headers: &'a [Repeatable<Header>],
        engine: &mut Engine,
    ) {
        let [first_header, ..] = headers else {
            return;
        };
        // Assuming non-conflicting headers sorted by increasing y, this must
        // be the header with the lowest level (sorted by increasing levels).
        let first_level = first_header.unwrap().level;

        // Stop repeating conflicting headers.
        // If we go to a new region before the pending headers fit alongside
        // their children, the old headers should not be displayed anymore.
        self.repeating_headers
            .truncate(self.repeating_headers.partition_point(|h| h.level < first_level));

        // Let's try to place them at least once.
        // This might be a waste as we could generate an orphan and thus have
        // to try to place old and new headers all over again, but that happens
        // for every new region anyway, so it's rather unavoidable.
        self.layout_headers(HeadersToLayout::NewHeaders(headers), engine);

        // After the first subsequent row is laid out, move to repeating, as
        // it's then confirmed the headers won't be moved due to orphan
        // prevention anymore.
        self.pending_headers = headers;
    }

    pub fn flush_pending_headers(&mut self) {
        for header in self.pending_headers {
            if let Repeatable::Repeated(header) = header {
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

    pub fn bump_repeating_headers(&mut self) {
        debug_assert!(!self.upcoming_headers.is_empty());

        let [next_header, ..] = self.upcoming_headers else {
            return;
        };

        // Keep only lower level headers. Assume sorted by increasing levels.
        self.repeating_headers.truncate(
            self.repeating_headers
                .partition_point(|h| h.level < next_header.unwrap().level),
        );

        if let Repeatable::Repeated(next_header) = next_header {
            // Vector remains sorted by increasing levels:
            // - It was sorted before, so the truncation above only keeps
            // elements with a lower level.
            // - Therefore, by pushing this header to the end, it will have
            // a level larger than all the previous headers, and is thus
            // in its 'correct' position.
            self.repeating_headers.push(next_header);
        }

        // Laying out the next header now.
        self.upcoming_headers = self.upcoming_headers.get(1..).unwrap_or_default();
    }

    /// Layouts the headers' rows.
    ///
    /// Assumes the footer height for the current region has already been
    /// calculated. Skips regions as necessary to fit all headers and all
    /// footers.
    pub fn layout_headers(
        &mut self,
        headers: HeadersToLayout<'a>,
        engine: &mut Engine,
    ) -> SourceResult<()> {
        // Generate different locations for content in headers across its
        // repetitions by assigning a unique number for each one.
        let disambiguator = self.finished.len();

        // At first, only consider the height of the given headers. However,
        // for upcoming regions, we will have to consider repeating headers as
        // well.
        let mut header_height = match headers {
            HeadersToLayout::RepeatingAndPending => self.simulate_header_height(
                self.repeating_headers
                    .iter()
                    .map(Deref::deref)
                    .chain(self.pending_headers.into_iter().map(Repeatable::unwrap)),
                &self.regions,
                engine,
                disambiguator,
            )?,
            HeadersToLayout::NewHeaders(headers) => self.simulate_header_height(
                headers.into_iter().map(Repeatable::unwrap),
                &self.regions,
                engine,
                disambiguator,
            )?,
        };

        // We already take the footer into account below.
        // While skipping regions, footer height won't be automatically
        // re-calculated until the end.
        let mut skipped_region = false;
        while self.unbreakable_rows_left == 0
            && !self.regions.size.y.fits(header_height)
            && self.regions.may_progress()
        {
            // Advance regions without any output until we can place the
            // header and the footer.
            self.finish_region_internal(Frame::soft(Axes::splat(Abs::zero())), vec![]);

            // TODO: re-calculate heights of headers and footers on each region
            // if 'full'changes? (Assuming height doesn't change for now...)
            if !skipped_region {
                if let HeadersToLayout::NewHeaders(headers) = headers {
                    header_height =
                    // Laying out new headers, so we have to consider the
                    // combined height of already repeating headers as well
                    // when beginning a new region.
                    self.simulate_header_height(
                        self.repeating_headers
                            .iter()
                            .map(|h| *h)
                            .chain(self.pending_headers.into_iter().chain(headers).map(Repeatable::unwrap)),
                        &self.regions,
                        engine,
                        disambiguator,
                    )?;
                }
            }

            skipped_region = true;

            // Ensure we also take the footer into account for remaining space.
            self.regions.size.y -= self.footer_height;
        }

        // Reset the header height for this region.
        // It will be re-calculated when laying out each header row.
        self.header_height = Abs::zero();

        if let Some(Repeatable::Repeated(footer)) = &self.grid.footer {
            if skipped_region {
                // Simulate the footer again; the region's 'full' might have
                // changed.
                // TODO: maybe this should go in the loop, a bit hacky as is...
                self.regions.size.y += self.footer_height;
                self.footer_height = self
                    .simulate_footer(footer, &self.regions, engine, disambiguator)?
                    .height;
                self.regions.size.y -= self.footer_height;
            }
        }

        // Group of headers is unbreakable.
        // Thus, no risk of 'finish_region' being recursively called from
        // within 'layout_row'.
        if let HeadersToLayout::NewHeaders(headers) = headers {
            // Do this before laying out repeating and pending headers from a
            // new region to make sure row code is aware that all of those
            // headers should stay together!
            self.unbreakable_rows_left +=
                total_header_row_count(headers.into_iter().map(Repeatable::unwrap));
        }

        // Need to relayout ALL headers if we skip a region, not only the
        // provided headers.
        // TODO: maybe extract this into a function to share code with multiple
        // footers.
        if matches!(headers, HeadersToLayout::RepeatingAndPending) || skipped_region {
            self.unbreakable_rows_left +=
                total_header_row_count(self.repeating_headers.iter().map(Deref::deref));

            // Use indices to avoid double borrow. We don't mutate headers in
            // 'layout_row' so this is fine.
            let mut i = 0;
            while let Some(&header) = self.repeating_headers.get(i) {
                for y in header.range() {
                    self.layout_row(y, engine, disambiguator)?;
                }
                i += 1;
            }

            for header in self.pending_headers {
                for y in header.unwrap().range() {
                    self.layout_row(y, engine, disambiguator)?;
                }
            }
        }

        if let HeadersToLayout::NewHeaders(headers) = headers {
            for header in headers {
                for y in header.unwrap().range() {
                    self.layout_row(y, engine, disambiguator)?;
                }
            }
        }

        Ok(())
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
            header.start,
            Some(header.end),
            regions,
            engine,
            disambiguator,
        )
    }

    /// Updates `self.footer_height` by simulating the footer, and skips to fitting region.
    pub fn prepare_footer(
        &mut self,
        footer: &Footer,
        engine: &mut Engine,
        disambiguator: usize,
    ) -> SourceResult<()> {
        let footer_height = self
            .simulate_footer(footer, &self.regions, engine, disambiguator)?
            .height;
        let mut skipped_region = false;
        while self.unbreakable_rows_left == 0
            && !self.regions.size.y.fits(footer_height)
            && self.regions.may_progress()
        {
            // Advance regions without any output until we can place the
            // footer.
            self.finish_region_internal(Frame::soft(Axes::splat(Abs::zero())), vec![]);
            skipped_region = true;
        }

        self.footer_height = if skipped_region {
            // Simulate the footer again; the region's 'full' might have
            // changed.
            self.simulate_footer(footer, &self.regions, engine, disambiguator)?
                .height
        } else {
            footer_height
        };

        Ok(())
    }

    /// Lays out all rows in the footer.
    /// They are unbreakable.
    pub fn layout_footer(
        &mut self,
        footer: &Footer,
        engine: &mut Engine,
        disambiguator: usize,
    ) -> SourceResult<()> {
        // Ensure footer rows have their own height available.
        // Won't change much as we're creating an unbreakable row group
        // anyway, so this is mostly for correctness.
        self.regions.size.y += self.footer_height;

        let footer_len = self.grid.rows.len() - footer.start;
        self.unbreakable_rows_left += footer_len;
        for y in footer.start..self.grid.rows.len() {
            self.layout_row(y, engine, disambiguator)?;
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
            footer.start,
            Some(self.grid.rows.len() - footer.start),
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
    headers.into_iter().map(|h| h.end - h.start).sum()
}
