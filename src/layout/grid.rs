use super::*;

/// A node that arranges its children in a grid.
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct GridNode {
    /// The `main` and `cross` directions of this grid.
    ///
    /// The rows go along the `main` direction and the columns along the `cross`
    /// direction.
    pub dirs: Gen<Dir>,
    /// Defines sizing for content rows and columns.
    pub tracks: Gen<Vec<TrackSizing>>,
    /// Defines sizing of gutter rows and columns between content.
    pub gutter: Gen<Vec<TrackSizing>>,
    /// The nodes to be arranged in a grid.
    pub children: Vec<AnyNode>,
}

/// Defines how to size a grid cell along an axis.
#[derive(Debug, Copy, Clone, PartialEq, Hash)]
pub enum TrackSizing {
    /// Fit the cell to its contents.
    Auto,
    /// A length stated in absolute values and fractions of the parent's size.
    Linear(Linear),
    /// A length that is the fraction of the remaining free space in the parent.
    Fractional(Fractional),
}

impl Layout for GridNode {
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
    ) -> Vec<Constrained<Rc<Frame>>> {
        // Prepare grid layout by unifying content and gutter tracks.
        let mut layouter = GridLayouter::new(self, regions.clone());

        // Determine all column sizes.
        layouter.measure_columns(ctx);

        // Layout the grid row-by-row.
        layouter.layout(ctx)
    }
}

impl From<GridNode> for AnyNode {
    fn from(grid: GridNode) -> Self {
        Self::new(grid)
    }
}

/// Performs grid layout.
struct GridLayouter<'a> {
    /// The axis of the cross direction.
    cross: SpecAxis,
    /// The axis of the main direction.
    main: SpecAxis,
    /// The original expand state of the target region.
    expand: Spec<bool>,
    /// The column tracks including gutter tracks.
    cols: Vec<TrackSizing>,
    /// The row tracks including gutter tracks.
    rows: Vec<TrackSizing>,
    /// The children of the grid.
    children: &'a [AnyNode],
    /// The region to layout into.
    regions: Regions,
    /// Resolved column sizes.
    rcols: Vec<Length>,
    /// The full main size of the current region.
    full: Length,
    /// The used-up size of the current region. The cross size is determined
    /// once after columns are resolved and not touched again.
    used: Gen<Length>,
    /// The sum of fractional ratios in the current region.
    fr: Fractional,
    /// Rows in the current region.
    lrows: Vec<Row>,
    /// Constraints for the active region.
    constraints: Constraints,
    /// Frames for finished regions.
    finished: Vec<Constrained<Rc<Frame>>>,
}

/// Produced by initial row layout, auto and linear rows are already finished,
/// fractional rows not yet.
enum Row {
    /// Finished row frame of auto or linear row.
    Frame(Frame),
    /// Ratio of a fractional row and y index of the track.
    Fr(Fractional, usize),
}

impl<'a> GridLayouter<'a> {
    /// Prepare grid layout by unifying content and gutter tracks.
    fn new(grid: &'a GridNode, mut regions: Regions) -> Self {
        let mut cols = vec![];
        let mut rows = vec![];

        // Number of content columns: Always at least one.
        let c = grid.tracks.cross.len().max(1);

        // Number of content rows: At least as many as given, but also at least
        // as many as needed to place each item.
        let r = {
            let len = grid.children.len();
            let given = grid.tracks.main.len();
            let needed = len / c + (len % c).clamp(0, 1);
            given.max(needed)
        };

        let auto = TrackSizing::Auto;
        let zero = TrackSizing::Linear(Linear::zero());
        let get_or = |tracks: &[_], idx, default| {
            tracks.get(idx).or(tracks.last()).copied().unwrap_or(default)
        };

        // Collect content and gutter columns.
        for x in 0 .. c {
            cols.push(get_or(&grid.tracks.cross, x, auto));
            cols.push(get_or(&grid.gutter.cross, x, zero));
        }

        // Collect content and gutter rows.
        for y in 0 .. r {
            rows.push(get_or(&grid.tracks.main, y, auto));
            rows.push(get_or(&grid.gutter.main, y, zero));
        }

        // Remove superfluous gutter tracks.
        cols.pop();
        rows.pop();

        let cross = grid.dirs.cross.axis();
        let main = grid.dirs.main.axis();
        let full = regions.current.get(main);
        let rcols = vec![Length::zero(); cols.len()];

        // We use the regions only for auto row measurement and constraints.
        let expand = regions.expand;
        regions.expand = Gen::new(true, false).to_spec(main);

        Self {
            cross,
            main,
            cols,
            rows,
            children: &grid.children,
            constraints: Constraints::new(expand),
            regions,
            expand,
            rcols,
            lrows: vec![],
            full,
            used: Gen::zero(),
            fr: Fractional::zero(),
            finished: vec![],
        }
    }

    /// Determine all column sizes.
    fn measure_columns(&mut self, ctx: &mut LayoutContext) {
        enum Case {
            PurelyLinear,
            Fitting,
            Overflowing,
            Exact,
        }

        // The different cases affecting constraints.
        let mut case = Case::PurelyLinear;

        // Sum of sizes of resolved linear tracks.
        let mut linear = Length::zero();

        // Sum of fractions of all fractional tracks.
        let mut fr = Fractional::zero();

        // Generic version of current and base size.
        let current = self.regions.current.to_gen(self.main);
        let base = self.regions.base.to_gen(self.main);

        // Resolve the size of all linear columns and compute the sum of all
        // fractional tracks.
        for (&col, rcol) in self.cols.iter().zip(&mut self.rcols) {
            match col {
                TrackSizing::Auto => {
                    case = Case::Fitting;
                }
                TrackSizing::Linear(v) => {
                    let resolved = v.resolve(base.cross);
                    *rcol = resolved;
                    linear += resolved;
                    self.constraints
                        .base
                        .set(self.cross, Some(self.regions.base.get(self.cross)));
                }
                TrackSizing::Fractional(v) => {
                    case = Case::Fitting;
                    fr += v;
                }
            }
        }

        // Size that is not used by fixed-size columns.
        let available = current.cross - linear;
        if available >= Length::zero() {
            // Determine size of auto columns.
            let (auto, count) = self.measure_auto_columns(ctx, available);

            // If there is remaining space, distribute it to fractional columns,
            // otherwise shrink auto columns.
            let remaining = available - auto;
            if remaining >= Length::zero() {
                if !fr.is_zero() {
                    self.grow_fractional_columns(remaining, fr);
                    case = Case::Exact;
                }
            } else {
                self.shrink_auto_columns(available, count);
                case = Case::Exact;
            }
        } else if let Case::Fitting = case {
            case = Case::Overflowing;
        }

        self.used.cross = self.rcols.iter().sum();

        match case {
            Case::PurelyLinear => {}
            Case::Fitting => {
                self.constraints.min.set(self.cross, Some(self.used.cross));
            }
            Case::Overflowing => {
                self.constraints.max.set(self.cross, Some(linear));
            }
            Case::Exact => {
                self.constraints
                    .exact
                    .set(self.cross, Some(self.regions.current.get(self.cross)));
            }
        }
    }

    /// Measure the size that is available to auto columns.
    fn measure_auto_columns(
        &mut self,
        ctx: &mut LayoutContext,
        available: Length,
    ) -> (Length, usize) {
        let mut auto = Length::zero();
        let mut count = 0;

        // Determine size of auto columns by laying out all cells in those
        // columns, measuring them and finding the largest one.
        for (x, &col) in self.cols.iter().enumerate() {
            if col != TrackSizing::Auto {
                continue;
            }

            let mut resolved = Length::zero();
            for node in (0 .. self.rows.len()).filter_map(|y| self.cell(x, y)) {
                let size = Gen::new(available, Length::inf()).to_size(self.main);
                let regions = Regions::one(size, Spec::splat(false));
                let frame = node.layout(ctx, &regions).remove(0);
                resolved.set_max(frame.size.get(self.cross));
            }

            self.rcols[x] = resolved;
            auto += resolved;
            count += 1;
        }

        (auto, count)
    }

    /// Distribute remaining space to fractional columns.
    fn grow_fractional_columns(&mut self, remaining: Length, fr: Fractional) {
        for (&col, rcol) in self.cols.iter().zip(&mut self.rcols) {
            if let TrackSizing::Fractional(v) = col {
                let ratio = v / fr;
                if ratio.is_finite() {
                    *rcol = ratio * remaining;
                }
            }
        }
    }

    /// Redistribute space to auto columns so that each gets a fair share.
    fn shrink_auto_columns(&mut self, available: Length, count: usize) {
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

    /// Layout the grid row-by-row.
    fn layout(mut self, ctx: &mut LayoutContext) -> Vec<Constrained<Rc<Frame>>> {
        for y in 0 .. self.rows.len() {
            match self.rows[y] {
                TrackSizing::Auto => {
                    self.layout_auto_row(ctx, y);
                }
                TrackSizing::Linear(v) => {
                    let base = self.regions.base.get(self.main);
                    if v.is_relative() {
                        self.constraints.base.set(self.main, Some(base));
                    }
                    let resolved = v.resolve(base);
                    let frame = self.layout_single_row(ctx, resolved, y);
                    self.push_row(ctx, frame);
                }
                TrackSizing::Fractional(v) => {
                    self.fr += v;
                    self.constraints.exact.set(self.main, Some(self.full));
                    self.lrows.push(Row::Fr(v, y));
                }
            }
        }

        self.finish_region(ctx);
        self.finished
    }

    /// Layout a row with automatic size along the main axis. Such a row may
    /// break across multiple regions.
    fn layout_auto_row(&mut self, ctx: &mut LayoutContext, y: usize) {
        let mut first = Length::zero();
        let mut rest: Vec<Length> = vec![];

        // Determine the size for each region of the row.
        for (x, &rcol) in self.rcols.iter().enumerate() {
            if let Some(node) = self.cell(x, y) {
                let cross = self.cross;
                self.regions.mutate(|size| size.set(cross, rcol));

                let mut sizes = node
                    .layout(ctx, &self.regions)
                    .into_iter()
                    .map(|frame| frame.size.get(self.main));

                if let Some(size) = sizes.next() {
                    first.set_max(size);
                }

                for (resolved, size) in rest.iter_mut().zip(&mut sizes) {
                    resolved.set_max(size);
                }

                rest.extend(sizes);
            }
        }

        // Layout the row.
        if rest.is_empty() {
            let frame = self.layout_single_row(ctx, first, y);
            self.push_row(ctx, frame);
        } else {
            let frames = self.layout_multi_row(ctx, first, &rest, y);
            let len = frames.len();
            for (i, frame) in frames.into_iter().enumerate() {
                if i + 1 != len {
                    self.constraints.exact.set(self.main, Some(self.full));
                }
                self.push_row(ctx, frame);
            }
        }
    }

    /// Layout a row with a fixed size along the main axis.
    fn layout_single_row(
        &self,
        ctx: &mut LayoutContext,
        length: Length,
        y: usize,
    ) -> Frame {
        let size = self.to_size(length);
        let mut output = Frame::new(size, size.height);
        let mut pos = Gen::zero();

        for (x, &rcol) in self.rcols.iter().enumerate() {
            if let Some(node) = self.cell(x, y) {
                let size = Gen::new(rcol, length).to_size(self.main);
                let regions = Regions::one(size, Spec::splat(true));
                let frame = node.layout(ctx, &regions).remove(0);
                output.push_frame(pos.to_point(self.main), frame.item);
            }

            pos.cross += rcol;
        }

        output
    }

    /// Layout a row spanning multiple regions.
    fn layout_multi_row(
        &self,
        ctx: &mut LayoutContext,
        first: Length,
        rest: &[Length],
        y: usize,
    ) -> Vec<Frame> {
        // Prepare frames.
        let mut outputs: Vec<_> = std::iter::once(first)
            .chain(rest.iter().copied())
            .map(|v| self.to_size(v))
            .map(|size| Frame::new(size, size.height))
            .collect();

        // Prepare regions.
        let mut regions = Regions::one(self.to_size(first), Spec::splat(true));
        regions.backlog = rest.iter().rev().map(|&v| self.to_size(v)).collect();

        // Layout the row.
        let mut pos = Gen::zero();
        for (x, &rcol) in self.rcols.iter().enumerate() {
            if let Some(node) = self.cell(x, y) {
                regions.mutate(|size| size.set(self.cross, rcol));

                // Push the layouted frames into the individual output frames.
                let frames = node.layout(ctx, &regions);
                for (output, frame) in outputs.iter_mut().zip(frames) {
                    output.push_frame(pos.to_point(self.main), frame.item);
                }
            }

            pos.cross += rcol;
        }

        outputs
    }

    /// Push a row frame into the current or next fitting region, finishing
    /// regions (including layouting fractional rows) if necessary.
    fn push_row(&mut self, ctx: &mut LayoutContext, frame: Frame) {
        let length = frame.size.get(self.main);

        // Skip to fitting region.
        while !self.regions.current.get(self.main).fits(length)
            && !self.regions.in_full_last()
        {
            self.constraints.max.set(self.main, Some(self.used.main + length));
            self.finish_region(ctx);
        }

        *self.regions.current.get_mut(self.main) -= length;
        self.used.main += length;
        self.lrows.push(Row::Frame(frame));
    }

    /// Finish rows for one region.
    fn finish_region(&mut self, ctx: &mut LayoutContext) {
        // Determine the size of the region.
        let length = if self.fr.is_zero() { self.used.main } else { self.full };
        let size = self.to_size(length);
        self.constraints.min.set(self.main, Some(length));

        // The frame for the region.
        let mut output = Frame::new(size, size.height);
        let mut pos = Gen::zero();

        // Determine the remaining size for fractional rows.
        let remaining = self.full - self.used.main;

        // Place finished rows and layout fractional rows.
        for row in std::mem::take(&mut self.lrows) {
            let frame = match row {
                Row::Frame(frame) => frame,
                Row::Fr(v, y) => {
                    let ratio = v / self.fr;
                    if remaining > Length::zero() && ratio.is_finite() {
                        let resolved = ratio * remaining;
                        self.layout_single_row(ctx, resolved, y)
                    } else {
                        continue;
                    }
                }
            };

            let main = frame.size.get(self.main);
            output.merge_frame(pos.to_point(self.main), frame);
            pos.main += main;
        }

        self.regions.next();
        self.full = self.regions.current.get(self.main);
        self.used.main = Length::zero();
        self.fr = Fractional::zero();
        self.finished.push(output.constrain(self.constraints));
        self.constraints = Constraints::new(self.expand);
    }

    /// Get the node in the cell in column `x` and row `y`.
    ///
    /// Returns `None` if it's a gutter cell.
    fn cell(&self, x: usize, y: usize) -> Option<&'a AnyNode> {
        assert!(x < self.cols.len());
        assert!(y < self.rows.len());

        // Even columns and rows are children, odd ones are gutter.
        if x % 2 == 0 && y % 2 == 0 {
            let c = 1 + self.cols.len() / 2;
            self.children.get((y / 2) * c + x / 2)
        } else {
            None
        }
    }

    /// Return a size where the cross axis spans the whole grid and the main
    /// axis the given length.
    fn to_size(&self, main_size: Length) -> Size {
        Gen::new(self.used.cross, main_size).to_size(self.main)
    }
}
