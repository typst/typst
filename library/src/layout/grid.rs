use crate::prelude::*;

use super::Spacing;

/// Arrange content in a grid.
///
/// # Parameters
/// - cells: Content (positional, variadic)
///   The contents of the table cells.
/// - rows: TrackSizings (named)
///   Defines the row sizes.
/// - columns: TrackSizings (named)
///   Defines the column sizes.
/// - gutter: TrackSizings (named)
///   Defines the gaps between rows & columns.
/// - column-gutter: TrackSizings (named)
///   Defines the gaps between columns. Takes precedence over `gutter`.
/// - row-gutter: TrackSizings (named)
///   Defines the gaps between rows. Takes precedence over `gutter`.
///
/// # Tags
/// - layout
#[func]
#[capable(Layout)]
#[derive(Debug, Hash)]
pub struct GridNode {
    /// Defines sizing for content rows and columns.
    pub tracks: Axes<Vec<TrackSizing>>,
    /// Defines sizing of gutter rows and columns between content.
    pub gutter: Axes<Vec<TrackSizing>>,
    /// The content to be arranged in a grid.
    pub cells: Vec<Content>,
}

#[node]
impl GridNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let TrackSizings(columns) = args.named("columns")?.unwrap_or_default();
        let TrackSizings(rows) = args.named("rows")?.unwrap_or_default();
        let TrackSizings(base_gutter) = args.named("gutter")?.unwrap_or_default();
        let column_gutter = args.named("column-gutter")?.map(|TrackSizings(v)| v);
        let row_gutter = args.named("row-gutter")?.map(|TrackSizings(v)| v);
        Ok(Self {
            tracks: Axes::new(columns, rows),
            gutter: Axes::new(
                column_gutter.unwrap_or_else(|| base_gutter.clone()),
                row_gutter.unwrap_or(base_gutter),
            ),
            cells: args.all()?,
        }
        .pack())
    }
}

impl Layout for GridNode {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        // Prepare grid layout by unifying content and gutter tracks.
        let layouter = GridLayouter::new(
            vt,
            self.tracks.as_deref(),
            self.gutter.as_deref(),
            &self.cells,
            regions,
            styles,
        );

        // Measure the columns and layout the grid row-by-row.
        layouter.layout()
    }
}

/// Defines how to size a grid cell along an axis.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum TrackSizing {
    /// A track that fits its cell's contents.
    Auto,
    /// A track size specified in absolute terms and relative to the parent's
    /// size.
    Relative(Rel<Length>),
    /// A track size specified as a fraction of the remaining free space in the
    /// parent.
    Fractional(Fr),
}

impl From<Spacing> for TrackSizing {
    fn from(spacing: Spacing) -> Self {
        match spacing {
            Spacing::Relative(rel) => Self::Relative(rel),
            Spacing::Fractional(fr) => Self::Fractional(fr),
        }
    }
}

/// Track sizing definitions.
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct TrackSizings(pub Vec<TrackSizing>);

castable! {
    TrackSizings,
    sizing: TrackSizing => Self(vec![sizing]),
    count: NonZeroUsize => Self(vec![TrackSizing::Auto; count.get()]),
    values: Array => Self(values
        .into_iter()
        .filter_map(|v| v.cast().ok())
        .collect()),
}

castable! {
    TrackSizing,
    _: AutoValue => Self::Auto,
    v: Rel<Length> => Self::Relative(v),
    v: Fr => Self::Fractional(v),
}

/// Performs grid layout.
struct GridLayouter<'a, 'v> {
    /// The core context.
    vt: &'a mut Vt<'v>,
    /// The grid cells.
    cells: &'a [Content],
    /// The column tracks including gutter tracks.
    cols: Vec<TrackSizing>,
    /// The row tracks including gutter tracks.
    rows: Vec<TrackSizing>,
    /// The regions to layout children into.
    regions: Regions<'a>,
    /// The inherited styles.
    styles: StyleChain<'a>,
    /// Resolved column sizes.
    rcols: Vec<Abs>,
    /// Rows in the current region.
    lrows: Vec<Row>,
    /// The full height of the current region.
    full: Abs,
    /// The used-up size of the current region. The horizontal size is
    /// determined once after columns are resolved and not touched again.
    used: Size,
    /// The sum of fractions in the current region.
    fr: Fr,
    /// Frames for finished regions.
    finished: Vec<Frame>,
}

/// Produced by initial row layout, auto and relative rows are already finished,
/// fractional rows not yet.
enum Row {
    /// Finished row frame of auto or relative row.
    Frame(Frame),
    /// Fractional row with y index.
    Fr(Fr, usize),
}

impl<'a, 'v> GridLayouter<'a, 'v> {
    /// Create a new grid layouter.
    ///
    /// This prepares grid layout by unifying content and gutter tracks.
    fn new(
        vt: &'a mut Vt<'v>,
        tracks: Axes<&[TrackSizing]>,
        gutter: Axes<&[TrackSizing]>,
        cells: &'a [Content],
        regions: Regions<'a>,
        styles: StyleChain<'a>,
    ) -> Self {
        let mut cols = vec![];
        let mut rows = vec![];

        // Number of content columns: Always at least one.
        let c = tracks.x.len().max(1);

        // Number of content rows: At least as many as given, but also at least
        // as many as needed to place each item.
        let r = {
            let len = cells.len();
            let given = tracks.y.len();
            let needed = len / c + (len % c).clamp(0, 1);
            given.max(needed)
        };

        let auto = TrackSizing::Auto;
        let zero = TrackSizing::Relative(Rel::zero());
        let get_or = |tracks: &[_], idx, default| {
            tracks.get(idx).or(tracks.last()).copied().unwrap_or(default)
        };

        // Collect content and gutter columns.
        for x in 0..c {
            cols.push(get_or(tracks.x, x, auto));
            cols.push(get_or(gutter.x, x, zero));
        }

        // Collect content and gutter rows.
        for y in 0..r {
            rows.push(get_or(tracks.y, y, auto));
            rows.push(get_or(gutter.y, y, zero));
        }

        // Remove superfluous gutter tracks.
        cols.pop();
        rows.pop();

        let full = regions.first.y;
        let rcols = vec![Abs::zero(); cols.len()];
        let lrows = vec![];

        // We use the regions for auto row measurement. Since at that moment,
        // columns are already sized, we can enable horizontal expansion.
        let mut regions = regions.clone();
        regions.expand = Axes::new(true, false);

        Self {
            vt,
            cells,
            cols,
            rows,
            regions,
            styles,
            rcols,
            lrows,
            full,
            used: Size::zero(),
            fr: Fr::zero(),
            finished: vec![],
        }
    }

    /// Determines the columns sizes and then layouts the grid row-by-row.
    fn layout(mut self) -> SourceResult<Fragment> {
        self.measure_columns()?;

        for y in 0..self.rows.len() {
            // Skip to next region if current one is full, but only for content
            // rows, not for gutter rows.
            if y % 2 == 0 && self.regions.is_full() {
                self.finish_region()?;
            }

            match self.rows[y] {
                TrackSizing::Auto => self.layout_auto_row(y)?,
                TrackSizing::Relative(v) => self.layout_relative_row(v, y)?,
                TrackSizing::Fractional(v) => {
                    self.lrows.push(Row::Fr(v, y));
                    self.fr += v;
                }
            }
        }

        self.finish_region()?;
        Ok(Fragment::frames(self.finished))
    }

    /// Determine all column sizes.
    fn measure_columns(&mut self) -> SourceResult<()> {
        // Sum of sizes of resolved relative tracks.
        let mut rel = Abs::zero();

        // Sum of fractions of all fractional tracks.
        let mut fr = Fr::zero();

        // Resolve the size of all relative columns and compute the sum of all
        // fractional tracks.
        for (&col, rcol) in self.cols.iter().zip(&mut self.rcols) {
            match col {
                TrackSizing::Auto => {}
                TrackSizing::Relative(v) => {
                    let resolved =
                        v.resolve(self.styles).relative_to(self.regions.base.x);
                    *rcol = resolved;
                    rel += resolved;
                }
                TrackSizing::Fractional(v) => fr += v,
            }
        }

        // Size that is not used by fixed-size columns.
        let available = self.regions.first.x - rel;
        if available >= Abs::zero() {
            // Determine size of auto columns.
            let (auto, count) = self.measure_auto_columns(available)?;

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
        self.used.x = self.rcols.iter().sum();

        Ok(())
    }

    /// Measure the size that is available to auto columns.
    fn measure_auto_columns(&mut self, available: Abs) -> SourceResult<(Abs, usize)> {
        let mut auto = Abs::zero();
        let mut count = 0;

        // Determine size of auto columns by laying out all cells in those
        // columns, measuring them and finding the largest one.
        for (x, &col) in self.cols.iter().enumerate() {
            if col != TrackSizing::Auto {
                continue;
            }

            let mut resolved = Abs::zero();
            for y in 0..self.rows.len() {
                if let Some(cell) = self.cell(x, y) {
                    let size = Size::new(available, self.regions.base.y);
                    let mut pod =
                        Regions::one(size, self.regions.base, Axes::splat(false));

                    // For relative rows, we can already resolve the correct
                    // base, for auto it's already correct and for fr we could
                    // only guess anyway.
                    if let TrackSizing::Relative(v) = self.rows[y] {
                        pod.base.y =
                            v.resolve(self.styles).relative_to(self.regions.base.y);
                    }

                    let frame = cell.layout(self.vt, self.styles, pod)?.into_frame();
                    resolved.set_max(frame.width());
                }
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

        for (&col, rcol) in self.cols.iter().zip(&mut self.rcols) {
            if let TrackSizing::Fractional(v) = col {
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

            for (&col, &rcol) in self.cols.iter().zip(&self.rcols) {
                // Remove an auto column if it is not overlarge (rcol <= fair),
                // but also hasn't already been removed (rcol > last).
                if col == TrackSizing::Auto && rcol <= fair && rcol > last {
                    redistribute -= rcol;
                    overlarge -= 1;
                    changed = true;
                }
            }
        }

        // Redistribute space fairly among overlarge columns.
        for (&col, rcol) in self.cols.iter().zip(&mut self.rcols) {
            if col == TrackSizing::Auto && *rcol > fair {
                *rcol = fair;
            }
        }
    }

    /// Layout a row with automatic height. Such a row may break across multiple
    /// regions.
    fn layout_auto_row(&mut self, y: usize) -> SourceResult<()> {
        let mut resolved: Vec<Abs> = vec![];

        // Determine the size for each region of the row.
        for (x, &rcol) in self.rcols.iter().enumerate() {
            if let Some(cell) = self.cell(x, y) {
                let mut pod = self.regions.clone();
                pod.first.x = rcol;
                pod.base.x = rcol;

                // All widths should be `rcol` except the base for auto columns.
                if self.cols[x] == TrackSizing::Auto {
                    pod.base.x = self.regions.base.x;
                }

                let mut sizes = cell
                    .layout(self.vt, self.styles, pod)?
                    .into_iter()
                    .map(|frame| frame.height());

                // For each region, we want to know the maximum height any
                // column requires.
                for (target, size) in resolved.iter_mut().zip(&mut sizes) {
                    target.set_max(size);
                }

                // New heights are maximal by virtue of being new. Note that
                // this extend only uses the rest of the sizes iterator.
                resolved.extend(sizes);
            }
        }

        // Nothing to layout.
        if resolved.is_empty() {
            return Ok(());
        }

        // Layout into a single region.
        if let &[first] = resolved.as_slice() {
            let frame = self.layout_single_row(first, y)?;
            self.push_row(frame);
            return Ok(());
        }

        // Expand all but the last region if the space is not
        // eaten up by any fr rows.
        if self.fr.is_zero() {
            let len = resolved.len();
            for (region, target) in self.regions.iter().zip(&mut resolved[..len - 1]) {
                target.set_max(region.y);
            }
        }

        // Layout into multiple regions.
        let fragment = self.layout_multi_row(&resolved, y)?;
        let len = fragment.len();
        for (i, frame) in fragment.into_iter().enumerate() {
            self.push_row(frame);
            if i + 1 < len {
                self.finish_region()?;
            }
        }

        Ok(())
    }

    /// Layout a row with relative height. Such a row cannot break across
    /// multiple regions, but it may force a region break.
    fn layout_relative_row(&mut self, v: Rel<Length>, y: usize) -> SourceResult<()> {
        let resolved = v.resolve(self.styles).relative_to(self.regions.base.y);
        let frame = self.layout_single_row(resolved, y)?;

        // Skip to fitting region.
        let height = frame.height();
        while !self.regions.first.y.fits(height) && !self.regions.in_last() {
            self.finish_region()?;

            // Don't skip multiple regions for gutter and don't push a row.
            if y % 2 == 1 {
                return Ok(());
            }
        }

        self.push_row(frame);

        Ok(())
    }

    /// Layout a row with fixed height and return its frame.
    fn layout_single_row(&mut self, height: Abs, y: usize) -> SourceResult<Frame> {
        let mut output = Frame::new(Size::new(self.used.x, height));
        let mut pos = Point::zero();

        for (x, &rcol) in self.rcols.iter().enumerate() {
            if let Some(cell) = self.cell(x, y) {
                let size = Size::new(rcol, height);

                // Set the base to the region's base for auto rows and to the
                // size for relative and fractional rows.
                let base = Axes::new(self.cols[x], self.rows[y])
                    .map(|s| s == TrackSizing::Auto)
                    .select(self.regions.base, size);

                let pod = Regions::one(size, base, Axes::splat(true));
                let frame = cell.layout(self.vt, self.styles, pod)?.into_frame();
                output.push_frame(pos, frame);
            }

            pos.x += rcol;
        }

        Ok(output)
    }

    /// Layout a row spanning multiple regions.
    fn layout_multi_row(&mut self, heights: &[Abs], y: usize) -> SourceResult<Fragment> {
        // Prepare frames.
        let mut outputs: Vec<_> = heights
            .iter()
            .map(|&h| Frame::new(Size::new(self.used.x, h)))
            .collect();

        // Prepare regions.
        let size = Size::new(self.used.x, heights[0]);
        let mut pod = Regions::one(size, self.regions.base, Axes::splat(true));
        pod.backlog = &heights[1..];

        // Layout the row.
        let mut pos = Point::zero();
        for (x, &rcol) in self.rcols.iter().enumerate() {
            if let Some(cell) = self.cell(x, y) {
                pod.first.x = rcol;
                pod.base.x = rcol;

                // All widths should be `rcol` except the base for auto columns.
                if self.cols[x] == TrackSizing::Auto {
                    pod.base.x = self.regions.base.x;
                }

                // Push the layouted frames into the individual output frames.
                let fragment = cell.layout(self.vt, self.styles, pod)?;
                for (output, frame) in outputs.iter_mut().zip(fragment) {
                    output.push_frame(pos, frame);
                }
            }

            pos.x += rcol;
        }

        Ok(Fragment::frames(outputs))
    }

    /// Push a row frame into the current region.
    fn push_row(&mut self, frame: Frame) {
        self.regions.first.y -= frame.height();
        self.used.y += frame.height();
        self.lrows.push(Row::Frame(frame));
    }

    /// Finish rows for one region.
    fn finish_region(&mut self) -> SourceResult<()> {
        // Determine the size of the grid in this region, expanding fully if
        // there are fr rows.
        let mut size = self.used;
        if self.fr.get() > 0.0 && self.full.is_finite() {
            size.y = self.full;
        }

        // The frame for the region.
        let mut output = Frame::new(size);
        let mut pos = Point::zero();

        // Place finished rows and layout fractional rows.
        for row in std::mem::take(&mut self.lrows) {
            let frame = match row {
                Row::Frame(frame) => frame,
                Row::Fr(v, y) => {
                    let remaining = self.full - self.used.y;
                    let height = v.share(self.fr, remaining);
                    self.layout_single_row(height, y)?
                }
            };

            let height = frame.height();
            output.push_frame(pos, frame);
            pos.y += height;
        }

        self.finished.push(output);
        self.regions.next();
        self.full = self.regions.first.y;
        self.used.y = Abs::zero();
        self.fr = Fr::zero();

        Ok(())
    }

    /// Get the content of the cell in column `x` and row `y`.
    ///
    /// Returns `None` if it's a gutter cell.
    #[track_caller]
    fn cell(&self, x: usize, y: usize) -> Option<&'a Content> {
        assert!(x < self.cols.len());
        assert!(y < self.rows.len());

        // Even columns and rows are children, odd ones are gutter.
        if x % 2 == 0 && y % 2 == 0 {
            let c = 1 + self.cols.len() / 2;
            self.cells.get((y / 2) * c + x / 2)
        } else {
            None
        }
    }
}
