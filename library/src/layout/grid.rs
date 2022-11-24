use crate::prelude::*;

use super::Spacing;

/// Arrange content in a grid.
#[derive(Debug, Hash)]
pub struct GridNode {
    /// Defines sizing for content rows and columns.
    pub tracks: Axes<Vec<TrackSizing>>,
    /// Defines sizing of gutter rows and columns between content.
    pub gutter: Axes<Vec<TrackSizing>>,
    /// The content to be arranged in a grid.
    pub cells: Vec<Content>,
}

#[node(LayoutBlock)]
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

impl LayoutBlock for GridNode {
    fn layout_block(
        &self,
        world: Tracked<dyn World>,
        regions: &Regions,
        styles: StyleChain,
    ) -> SourceResult<Vec<Frame>> {
        // Prepare grid layout by unifying content and gutter tracks.
        let layouter = GridLayouter::new(
            world,
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
    Expected: "integer, auto, relative length, fraction, or array of the latter three",
    Value::Auto => Self(vec![TrackSizing::Auto]),
    Value::Length(v) => Self(vec![TrackSizing::Relative(v.into())]),
    Value::Ratio(v) => Self(vec![TrackSizing::Relative(v.into())]),
    Value::Relative(v) => Self(vec![TrackSizing::Relative(v)]),
    Value::Fraction(v) => Self(vec![TrackSizing::Fractional(v)]),
    Value::Int(v) => Self(vec![
        TrackSizing::Auto;
        Value::Int(v).cast::<NonZeroUsize>()?.get()
    ]),
    Value::Array(values) => Self(values
        .into_iter()
        .filter_map(|v| v.cast().ok())
        .collect()),
}

castable! {
    TrackSizing,
    Expected: "auto, relative length, or fraction",
    Value::Auto => Self::Auto,
    Value::Length(v) => Self::Relative(v.into()),
    Value::Ratio(v) => Self::Relative(v.into()),
    Value::Relative(v) => Self::Relative(v),
    Value::Fraction(v) => Self::Fractional(v),
}

/// Performs grid layout.
struct GridLayouter<'a> {
    /// The core context.
    world: Tracked<'a, dyn World>,
    /// The grid cells.
    cells: &'a [Content],
    /// The column tracks including gutter tracks.
    cols: Vec<TrackSizing>,
    /// The row tracks including gutter tracks.
    rows: Vec<TrackSizing>,
    /// The regions to layout children into.
    regions: Regions,
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

impl<'a> GridLayouter<'a> {
    /// Create a new grid layouter.
    ///
    /// This prepares grid layout by unifying content and gutter tracks.
    fn new(
        world: Tracked<'a, dyn World>,
        tracks: Axes<&[TrackSizing]>,
        gutter: Axes<&[TrackSizing]>,
        cells: &'a [Content],
        regions: &Regions,
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
            world,
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
    fn layout(mut self) -> SourceResult<Vec<Frame>> {
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
        Ok(self.finished)
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
                if !fr.is_zero() {
                    self.grow_fractional_columns(remaining, fr);
                }
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

                    let frame =
                        cell.layout_block(self.world, &pod, self.styles)?.remove(0);
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
        for (&col, rcol) in self.cols.iter().zip(&mut self.rcols) {
            if let TrackSizing::Fractional(v) = col {
                *rcol = v.share(fr, remaining);
            }
        }
    }

    /// Redistribute space to auto columns so that each gets a fair share.
    fn shrink_auto_columns(&mut self, available: Abs, count: usize) {
        // The fair share each auto column may have.
        let fair = available / count as f64;

        // The number of overlarge auto columns and the space that will be
        // equally redistributed to them.
        let mut overlarge: usize = 0;
        let mut redistribute = available;

        // Find out the number of and space used by overlarge auto columns.
        for (&col, rcol) in self.cols.iter().zip(&mut self.rcols) {
            if col == TrackSizing::Auto {
                if *rcol > fair {
                    overlarge += 1;
                } else {
                    redistribute -= *rcol;
                }
            }
        }

        // Redistribute the space equally.
        let share = redistribute / overlarge as f64;
        for (&col, rcol) in self.cols.iter().zip(&mut self.rcols) {
            if col == TrackSizing::Auto && *rcol > fair {
                *rcol = share;
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
                    .layout_block(self.world, &pod, self.styles)?
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
        let frames = self.layout_multi_row(&resolved, y)?;
        let len = frames.len();
        for (i, frame) in frames.into_iter().enumerate() {
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
                let frame = cell.layout_block(self.world, &pod, self.styles)?.remove(0);
                output.push_frame(pos, frame);
            }

            pos.x += rcol;
        }

        Ok(output)
    }

    /// Layout a row spanning multiple regions.
    fn layout_multi_row(
        &mut self,
        heights: &[Abs],
        y: usize,
    ) -> SourceResult<Vec<Frame>> {
        // Prepare frames.
        let mut outputs: Vec<_> = heights
            .iter()
            .map(|&h| Frame::new(Size::new(self.used.x, h)))
            .collect();

        // Prepare regions.
        let size = Size::new(self.used.x, heights[0]);
        let mut pod = Regions::one(size, self.regions.base, Axes::splat(true));
        pod.backlog = heights[1..].to_vec();

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
                let frames = cell.layout_block(self.world, &pod, self.styles)?;
                for (output, frame) in outputs.iter_mut().zip(frames) {
                    output.push_frame(pos, frame);
                }
            }

            pos.x += rcol;
        }

        Ok(outputs)
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
