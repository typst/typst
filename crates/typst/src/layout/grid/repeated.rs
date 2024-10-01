use super::rowspans::UnbreakableRowGroup;
use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::layout::{Abs, Axes, Frame, GridLayouter, Regions};

/// A repeatable grid header. Starts at the first row.
pub(super) struct Header {
    /// The index after the last row included in this header.
    pub(super) end: usize,
}

/// A repeatable grid footer. Stops at the last row.
pub(super) struct Footer {
    /// The first row included in this footer.
    pub(super) start: usize,
}

/// A possibly repeatable grid object.
/// It still exists even when not repeatable, but must not have additional
/// considerations by grid layout, other than for consistency (such as making
/// a certain group of rows unbreakable).
pub(super) enum Repeatable<T> {
    Repeated(T),
    NotRepeated(T),
}

impl<T> Repeatable<T> {
    /// Gets the value inside this repeatable, regardless of whether
    /// it repeats.
    pub(super) fn unwrap(&self) -> &T {
        match self {
            Self::Repeated(repeated) => repeated,
            Self::NotRepeated(not_repeated) => not_repeated,
        }
    }

    /// Returns `Some` if the value is repeated, `None` otherwise.
    pub(super) fn as_repeated(&self) -> Option<&T> {
        match self {
            Self::Repeated(repeated) => Some(repeated),
            Self::NotRepeated(_) => None,
        }
    }
}

impl<'a> GridLayouter<'a> {
    /// Layouts the header's rows.
    /// Skips regions as necessary.
    pub(super) fn layout_header(
        &mut self,
        header: &Header,
        engine: &mut Engine,
        disambiguator: usize,
    ) -> SourceResult<()> {
        let header_rows =
            self.simulate_header(header, &self.regions, engine, disambiguator)?;
        let mut skipped_region = false;
        while self.unbreakable_rows_left == 0
            && !self.regions.size.y.fits(header_rows.height + self.footer_height)
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

        // Header is unbreakable.
        // Thus, no risk of 'finish_region' being recursively called from
        // within 'layout_row'.
        self.unbreakable_rows_left += header.end;
        for y in 0..header.end {
            self.layout_row(y, engine, disambiguator)?;
        }
        Ok(())
    }

    /// Simulate the header's group of rows.
    pub(super) fn simulate_header(
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
            0,
            Some(header.end),
            regions,
            engine,
            disambiguator,
        )
    }

    /// Updates `self.footer_height` by simulating the footer, and skips to fitting region.
    pub(super) fn prepare_footer(
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
    pub(super) fn layout_footer(
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
    pub(super) fn simulate_footer(
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
