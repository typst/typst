use super::prelude::*;

/// `grid`: Arrange children into a grid.
pub fn grid(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    castable! {
        Vec<TrackSizing>,
        Expected: "integer or (auto, linear, fractional, or array thereof)",
        Value::Auto => vec![TrackSizing::Auto],
        Value::Length(v) => vec![TrackSizing::Linear(v.into())],
        Value::Relative(v) => vec![TrackSizing::Linear(v.into())],
        Value::Linear(v) => vec![TrackSizing::Linear(v)],
        Value::Fractional(v) => vec![TrackSizing::Fractional(v)],
        Value::Int(count) => vec![TrackSizing::Auto; count.max(0) as usize],
        Value::Array(values) => values
            .into_iter()
            .filter_map(|v| v.cast().ok())
            .collect(),
    }

    castable! {
        TrackSizing,
        Expected: "auto, linear, or fractional",
        Value::Auto => Self::Auto,
        Value::Length(v) => Self::Linear(v.into()),
        Value::Relative(v) => Self::Linear(v.into()),
        Value::Linear(v) => Self::Linear(v),
        Value::Fractional(v) => Self::Fractional(v),
    }

    let columns = args.named("columns")?.unwrap_or_default();
    let rows = args.named("rows")?.unwrap_or_default();
    let tracks = Spec::new(columns, rows);

    let base_gutter: Vec<TrackSizing> = args.named("gutter")?.unwrap_or_default();
    let column_gutter = args.named("column-gutter")?;
    let row_gutter = args.named("row-gutter")?;
    let gutter = Spec::new(
        column_gutter.unwrap_or_else(|| base_gutter.clone()),
        row_gutter.unwrap_or(base_gutter),
    );

    let children: Vec<Template> = args.all().collect();

    Ok(Value::Template(Template::from_block(move |style| {
        GridNode {
            tracks: tracks.clone(),
            gutter: gutter.clone(),
            children: children.iter().map(|child| child.to_stack(style).pack()).collect(),
        }
    })))
}

/// A node that arranges its children in a grid.
#[derive(Debug, Hash)]
pub struct GridNode {
    /// Defines sizing for content rows and columns.
    pub tracks: Spec<Vec<TrackSizing>>,
    /// Defines sizing of gutter rows and columns between content.
    pub gutter: Spec<Vec<TrackSizing>>,
    /// The nodes to be arranged in a grid.
    pub children: Vec<BlockNode>,
}

/// Defines how to size a grid cell along an axis.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum TrackSizing {
    /// Fit the cell to its contents.
    Auto,
    /// A length stated in absolute values and/or relative to the parent's size.
    Linear(Linear),
    /// A length that is the fraction of the remaining free space in the parent.
    Fractional(Fractional),
}

impl BlockLevel for GridNode {
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

/// Performs grid layout.
struct GridLayouter<'a> {
    /// The original expand state of the target region.
    expand: Spec<bool>,
    /// The column tracks including gutter tracks.
    cols: Vec<TrackSizing>,
    /// The row tracks including gutter tracks.
    rows: Vec<TrackSizing>,
    /// The children of the grid.
    children: &'a [BlockNode],
    /// The regions to layout into.
    regions: Regions,
    /// Resolved column sizes.
    rcols: Vec<Length>,
    /// The full block size of the current region.
    full: Length,
    /// The used-up size of the current region. The horizontal size is
    /// determined once after columns are resolved and not touched again.
    used: Size,
    /// The sum of fractional ratios in the current region.
    fr: Fractional,
    /// Rows in the current region.
    lrows: Vec<Row>,
    /// Constraints for the active region.
    cts: Constraints,
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
        let c = grid.tracks.x.len().max(1);

        // Number of content rows: At least as many as given, but also at least
        // as many as needed to place each item.
        let r = {
            let len = grid.children.len();
            let given = grid.tracks.y.len();
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
            cols.push(get_or(&grid.tracks.x, x, auto));
            cols.push(get_or(&grid.gutter.x, x, zero));
        }

        // Collect content and gutter rows.
        for y in 0 .. r {
            rows.push(get_or(&grid.tracks.y, y, auto));
            rows.push(get_or(&grid.gutter.y, y, zero));
        }

        // Remove superfluous gutter tracks.
        cols.pop();
        rows.pop();

        // We use the regions only for auto row measurement and constraints.
        let expand = regions.expand;
        regions.expand = Spec::new(true, false);

        Self {
            children: &grid.children,
            cts: Constraints::new(expand),
            full: regions.current.h,
            expand,
            rcols: vec![Length::zero(); cols.len()],
            lrows: vec![],
            used: Size::zero(),
            fr: Fractional::zero(),
            finished: vec![],
            cols,
            rows,
            regions,
        }
    }

    /// Determine all column sizes.
    fn measure_columns(&mut self, ctx: &mut LayoutContext) {
        enum Case {
            PurelyLinear,
            Fitting,
            Exact,
            Overflowing,
        }

        // The different cases affecting constraints.
        let mut case = Case::PurelyLinear;

        // Sum of sizes of resolved linear tracks.
        let mut linear = Length::zero();

        // Sum of fractions of all fractional tracks.
        let mut fr = Fractional::zero();

        // Resolve the size of all linear columns and compute the sum of all
        // fractional tracks.
        for (&col, rcol) in self.cols.iter().zip(&mut self.rcols) {
            match col {
                TrackSizing::Auto => {
                    case = Case::Fitting;
                }
                TrackSizing::Linear(v) => {
                    let resolved = v.resolve(self.regions.base.w);
                    *rcol = resolved;
                    linear += resolved;
                }
                TrackSizing::Fractional(v) => {
                    case = Case::Fitting;
                    fr += v;
                }
            }
        }

        // Size that is not used by fixed-size columns.
        let available = self.regions.current.w - linear;
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
        } else if matches!(case, Case::Fitting) {
            case = Case::Overflowing;
        }

        // Children could depend on base.
        self.cts.base = self.regions.base.to_spec().map(Some);

        // Set constraints depending on the case we hit.
        match case {
            Case::PurelyLinear => {}
            Case::Fitting => self.cts.min.x = Some(self.used.w),
            Case::Exact => self.cts.exact.x = Some(self.regions.current.w),
            Case::Overflowing => self.cts.max.x = Some(linear),
        }

        // Sum up the resolved column sizes once here.
        self.used.w = self.rcols.iter().sum();
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
            for y in 0 .. self.rows.len() {
                if let Some(node) = self.cell(x, y) {
                    let size = Size::new(available, Length::inf());
                    let mut regions =
                        Regions::one(size, self.regions.base, Spec::splat(false));

                    // For fractional rows, we can already resolve the correct
                    // base, for auto it's already correct and for fr we could
                    // only guess anyway.
                    if let TrackSizing::Linear(v) = self.rows[y] {
                        regions.base.h = v.resolve(self.regions.base.h);
                    }

                    let frame = node.layout(ctx, &regions).remove(0).item;
                    resolved.set_max(frame.size.w);
                }
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
                TrackSizing::Auto => self.layout_auto_row(ctx, y),
                TrackSizing::Linear(v) => self.layout_linear_row(ctx, v, y),
                TrackSizing::Fractional(v) => {
                    self.cts.exact.y = Some(self.full);
                    self.lrows.push(Row::Fr(v, y));
                    self.fr += v;
                }
            }
        }

        self.finish_region(ctx);
        self.finished
    }

    /// Layout a row with automatic size along the block axis. Such a row may
    /// break across multiple regions.
    fn layout_auto_row(&mut self, ctx: &mut LayoutContext, y: usize) {
        let mut resolved: Vec<Length> = vec![];

        // Determine the size for each region of the row.
        for (x, &rcol) in self.rcols.iter().enumerate() {
            if let Some(node) = self.cell(x, y) {
                let mut regions = self.regions.clone();
                regions.mutate(|size| size.w = rcol);

                // Set the horizontal base back to the parent region's base for
                // auto columns.
                if self.cols[x] == TrackSizing::Auto {
                    regions.base.w = self.regions.base.w;
                }

                let mut sizes =
                    node.layout(ctx, &regions).into_iter().map(|frame| frame.item.size.h);

                for (target, size) in resolved.iter_mut().zip(&mut sizes) {
                    target.set_max(size);
                }

                resolved.extend(sizes);
            }
        }

        // Nothing to layout.
        if resolved.is_empty() {
            return;
        }

        // Layout into a single region.
        if let &[first] = resolved.as_slice() {
            let frame = self.layout_single_row(ctx, first, y);
            self.push_row(frame);
            return;
        }

        // Expand all but the last region if the space is not
        // eaten up by any fr rows.
        if self.fr.is_zero() {
            let len = resolved.len();
            for (target, (current, _)) in
                resolved[.. len - 1].iter_mut().zip(self.regions.iter())
            {
                target.set_max(current.h);
            }
        }

        // Layout into multiple regions.
        let frames = self.layout_multi_row(ctx, &resolved, y);
        let len = frames.len();
        for (i, frame) in frames.into_iter().enumerate() {
            self.push_row(frame);
            if i + 1 < len {
                self.cts.exact.y = Some(self.full);
                self.finish_region(ctx);
            }
        }
    }

    /// Layout a row with linear sizing along the block axis. Such a row cannot
    /// break across multiple regions, but it may force a region break.
    fn layout_linear_row(&mut self, ctx: &mut LayoutContext, v: Linear, y: usize) {
        let resolved = v.resolve(self.regions.base.h);
        let frame = self.layout_single_row(ctx, resolved, y);

        // Skip to fitting region.
        let length = frame.size.h;
        while !self.regions.current.h.fits(length) && !self.regions.in_full_last() {
            self.cts.max.y = Some(self.used.h + length);
            self.finish_region(ctx);

            // Don't skip multiple regions for gutter and don't push a row.
            if y % 2 == 1 {
                return;
            }
        }

        self.push_row(frame);
    }

    /// Layout a row with a fixed size along the block axis and return its frame.
    fn layout_single_row(
        &self,
        ctx: &mut LayoutContext,
        height: Length,
        y: usize,
    ) -> Frame {
        let mut output = Frame::new(Size::new(self.used.w, height), height);
        let mut pos = Point::zero();

        for (x, &rcol) in self.rcols.iter().enumerate() {
            if let Some(node) = self.cell(x, y) {
                let size = Size::new(rcol, height);

                // Set the base to the size for non-auto rows.
                let mut base = self.regions.base;
                if self.cols[x] != TrackSizing::Auto {
                    base.w = size.w;
                }
                if self.rows[y] != TrackSizing::Auto {
                    base.h = size.h;
                }

                let regions = Regions::one(size, base, Spec::splat(true));
                let frame = node.layout(ctx, &regions).remove(0);
                output.push_frame(pos, frame.item);
            }

            pos.x += rcol;
        }

        output
    }

    /// Layout a row spanning multiple regions.
    fn layout_multi_row(
        &self,
        ctx: &mut LayoutContext,
        resolved: &[Length],
        y: usize,
    ) -> Vec<Frame> {
        // Prepare frames.
        let mut outputs: Vec<_> = resolved
            .iter()
            .map(|&h| Frame::new(Size::new(self.used.w, h), h))
            .collect();

        // Prepare regions.
        let size = Size::new(self.used.w, resolved[0]);
        let mut regions = Regions::one(size, self.regions.base, Spec::splat(true));
        regions.backlog = resolved[1 ..]
            .iter()
            .map(|&h| Size::new(self.used.w, h))
            .collect::<Vec<_>>()
            .into_iter();

        // Layout the row.
        let mut pos = Point::zero();
        for (x, &rcol) in self.rcols.iter().enumerate() {
            if let Some(node) = self.cell(x, y) {
                regions.mutate(|size| size.w = rcol);

                // Set the horizontal base back to the parent region's base for
                // auto columns.
                if self.cols[x] == TrackSizing::Auto {
                    regions.base.w = self.regions.base.w;
                }

                // Push the layouted frames into the individual output frames.
                let frames = node.layout(ctx, &regions);
                for (output, frame) in outputs.iter_mut().zip(frames) {
                    output.push_frame(pos, frame.item);
                }
            }

            pos.x += rcol;
        }

        outputs
    }

    /// Push a row frame into the current region.
    fn push_row(&mut self, frame: Frame) {
        self.regions.current.h -= frame.size.h;
        self.used.h += frame.size.h;
        self.lrows.push(Row::Frame(frame));
    }

    /// Finish rows for one region.
    fn finish_region(&mut self, ctx: &mut LayoutContext) {
        // Determine the size that remains for fractional rows.
        let remaining = self.full - self.used.h;

        // Determine the size of the grid in this region, expanding fully if
        // there are fr rows.
        let mut size = self.used;
        if !self.fr.is_zero() && self.full.is_finite() {
            size.h = self.full;
        }

        self.cts.min.y = Some(size.h);

        // The frame for the region.
        let mut output = Frame::new(size, size.h);
        let mut pos = Point::zero();

        // Place finished rows and layout fractional rows.
        for row in std::mem::take(&mut self.lrows) {
            let frame = match row {
                Row::Frame(frame) => frame,
                Row::Fr(v, y) => {
                    let ratio = v / self.fr;
                    if remaining.is_finite() && ratio.is_finite() {
                        let resolved = ratio * remaining;
                        self.layout_single_row(ctx, resolved, y)
                    } else {
                        continue;
                    }
                }
            };

            let height = frame.size.h;
            output.merge_frame(pos, frame);
            pos.y += height;
        }

        self.regions.next();
        self.full = self.regions.current.h;
        self.used.h = Length::zero();
        self.fr = Fractional::zero();
        self.finished.push(output.constrain(self.cts));
        self.cts = Constraints::new(self.expand);
    }

    /// Get the node in the cell in column `x` and row `y`.
    ///
    /// Returns `None` if it's a gutter cell.
    #[track_caller]
    fn cell(&self, x: usize, y: usize) -> Option<&'a BlockNode> {
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
}
