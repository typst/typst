use std::ops::ControlFlow;

use typst_library::diag::SourceResult;
use typst_library::engine::Engine;
use typst_library::layout::grid::resolve::{Footer, Header, Repeatable};
use typst_library::layout::{Abs, Axes, Frame, Regions};

use super::layouter::GridLayouter;
use super::rowspans::UnbreakableRowGroup;

impl<'a> GridLayouter<'a> {
    #[inline]
    fn pending_headers(&self) -> &'a [Repeatable<Header>] {
        &self.upcoming_headers[..self.pending_header_end]
    }

    #[inline]
    pub fn bump_pending_headers(&mut self) {
        debug_assert!(!self.upcoming_headers.is_empty());
        self.pending_header_end += 1;
    }

    #[inline]
    pub fn peek_upcoming_header(&self) -> Option<&'a Repeatable<Header>> {
        self.upcoming_headers.get(self.pending_header_end)
    }

    pub fn flush_pending_headers(&mut self) {
        debug_assert!(!self.upcoming_headers.is_empty());
        debug_assert!(self.pending_header_end > 0);
        let headers = self.pending_headers();

        let [first_header, ..] = headers else {
            return;
        };

        self.repeating_headers.truncate(
            self.repeating_headers
                .partition_point(|h| h.level < first_header.unwrap().level),
        );

        for header in self.pending_headers() {
            if let Repeatable::Repeated(header) = header {
                // Vector remains sorted by increasing levels:
                // - It was sorted before, so the truncation above only keeps
                // elements with a lower level.
                // - Therefore, by pushing this header to the end, it will have
                // a level larger than all the previous headers, and is thus
                // in its 'correct' position.
                self.repeating_headers.push(header);
            }
        }

        self.upcoming_headers = self
            .upcoming_headers
            .get(self.pending_header_end..)
            .unwrap_or_default();

        self.pending_header_end = 0;
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
        headers: &[&Header],
        engine: &mut Engine,
        disambiguator: usize,
    ) -> SourceResult<()> {
        let header_height =
            self.simulate_header_height(&self.regions, engine, disambiguator)?;
        let mut skipped_region = false;
        while self.unbreakable_rows_left == 0
            && !self.regions.size.y.fits(header_height + self.footer_height)
            && self.regions.may_progress()
        {
            // Advance regions without any output until we can place the
            // header and the footer.
            self.finish_region_internal(Frame::soft(Axes::splat(Abs::zero())), vec![]);
            skipped_region = true;
        }

        // Reset the header height for this region.
        // It will be re-calculated when laying out each header row.
        self.header_height = Abs::zero();

        if let Some(Repeatable::Repeated(footer)) = &self.grid.footer {
            if skipped_region {
                // Simulate the footer again; the region's 'full' might have
                // changed.
                self.footer_height = self
                    .simulate_footer(footer, &self.regions, engine, disambiguator)?
                    .height;
            }
        }

        // Group of headers is unbreakable.
        // Thus, no risk of 'finish_region' being recursively called from
        // within 'layout_row'.
        self.unbreakable_rows_left += total_header_row_count(headers);
        for header in headers {
            for y in header.range() {
                self.layout_row(y, engine, disambiguator)?;
            }
        }
        Ok(())
    }

    /// Calculates the total expected height of several headers.
    pub fn simulate_header_height(
        &self,
        headers: &[&Header],
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
pub fn total_header_row_count(headers: &[&Header]) -> usize {
    headers.iter().map(|h| h.end - h.start).sum()
}
