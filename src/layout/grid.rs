use super::*;

/// A node that arranges its children in a grid.
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct GridNode {
    /// The column (cross) direction of this stack.
    pub column_dir: Dir,
    /// The nodes to be arranged in a grid.
    pub children: Vec<AnyNode>,
    /// Defines sizing for rows and columns.
    pub tracks: Gen<Tracks>,
    /// Defines sizing of the gutter between rows and columns.
    pub gutter: Gen<Tracks>,
}

impl Layout for GridNode {
    fn layout(&self, ctx: &mut LayoutContext, regions: &Regions) -> Vec<Frame> {
        GridLayouter::new(self, regions.clone()).layout(ctx)
    }
}

impl From<GridNode> for AnyNode {
    fn from(grid: GridNode) -> Self {
        Self::new(grid)
    }
}

struct GridLayouter<'a> {
    cross: SpecAxis,
    main: SpecAxis,
    cols: Vec<TrackSizing>,
    rows: Vec<TrackSizing>,
    cells: Vec<Cell<'a>>,
    regions: Regions,
    rrows: Vec<(usize, Option<Length>)>,
    rcols: Vec<Length>,
    finished: Vec<Frame>,
}

enum Cell<'a> {
    Node(&'a AnyNode),
    Gutter,
}

impl<'a> GridLayouter<'a> {
    fn new(grid: &'a GridNode, regions: Regions) -> Self {
        let mut col_sizes = vec![];
        let mut row_sizes = vec![];
        let mut cells = vec![];

        // A grid always needs to have at least one column.
        let cols = grid.tracks.cross.0.len().max(1);

        // Create at least as many rows as specified and also at least as many
        // as necessary to place each item.
        let rows = {
            let len = grid.children.len();
            let specified = grid.tracks.main.0.len();
            let necessary = len / cols + (len % cols).clamp(0, 1);
            specified.max(necessary)
        };

        // Collect the track sizing for all columns, including gutter columns.
        for i in 0 .. cols {
            col_sizes.push(grid.tracks.cross.get(i));
            if i < cols - 1 {
                col_sizes.push(grid.gutter.cross.get(i));
            }
        }

        // Collect the track sizing for all rows, including gutter rows.
        for i in 0 .. rows {
            row_sizes.push(grid.tracks.main.get(i));
            if i < rows - 1 {
                row_sizes.push(grid.gutter.main.get(i));
            }
        }

        // Build up the matrix of cells, including gutter cells.
        for (i, item) in grid.children.iter().enumerate() {
            cells.push(Cell::Node(item));

            let row = i / cols;
            let col = i % cols;

            if col < cols - 1 {
                // Push gutter after each child.
                cells.push(Cell::Gutter);
            } else if row < rows - 1 {
                // Except for the last child of each row.
                // There we push a gutter row.
                for _ in 0 .. col_sizes.len() {
                    cells.push(Cell::Gutter);
                }
            }
        }

        // Fill the thing up.
        while cells.len() < col_sizes.len() * row_sizes.len() {
            cells.push(Cell::Gutter)
        }

        Self {
            cross: grid.column_dir.axis(),
            main: grid.column_dir.axis().other(),
            cols: col_sizes,
            rows: row_sizes,
            cells,
            regions,
            rrows: vec![],
            rcols: vec![],
            finished: vec![],
        }
    }

    fn layout(mut self, ctx: &mut LayoutContext) -> Vec<Frame> {
        // Shrink area by linear sizing.
        let mut available = self.regions.current.get(self.cross);
        available -= self
            .cols
            .iter()
            .filter_map(|x| match x {
                TrackSizing::Linear(l) => {
                    Some(l.resolve(self.regions.base.get(self.cross)))
                }
                _ => None,
            })
            .sum();

        let col_frac: f64 = self
            .cols
            .iter()
            .filter_map(|x| match x {
                TrackSizing::Fractional(f) => Some(f.get()),
                _ => None,
            })
            .sum();

        let auto_columns = self
            .cols
            .iter()
            .enumerate()
            .filter_map(|(i, x)| (x == &TrackSizing::Auto).then(|| i));

        let mut col_width = vec![];

        // For each of the auto columns, lay out all elements with
        // `preliminary_length` rows and build max.
        for x in auto_columns {
            let mut max = Length::zero();

            for (y, row) in self.rows.iter().enumerate() {
                let mut size = self.regions.current;
                if let TrackSizing::Linear(l) = row {
                    *size.get_mut(self.main) =
                        l.resolve(self.regions.base.get(self.main));
                }

                let region = Regions::one(size, Spec::splat(false));
                if let Cell::Node(node) = self.get(x, y) {
                    let frame = node.layout(ctx, &region).remove(0);
                    max = max.max(frame.size.get(self.cross))
                }
            }

            col_width.push((x, max));
        }

        // If accumulated auto column size exceeds available size, redistribute
        // space proportionally amongst elements that exceed their size
        // allocation.
        let mut total: Length = col_width.iter().map(|(_, x)| *x).sum();
        if total > available {
            let alloc = available / col_width.len() as f64;

            let mut count: usize = 0;
            let mut redistributable = Length::zero();

            for &(_, l) in &col_width {
                if l > alloc {
                    redistributable += l;
                    count += 1;
                }
            }

            let x = (available - total + redistributable) / count as f64;

            if !redistributable.is_zero() {
                for (_, l) in &mut col_width {
                    if *l > alloc {
                        *l = x;
                    }
                }
            }

            total = available;
        }

        // Build rcols
        for (x, len) in col_width
            .into_iter()
            .map(|(x, s)| (x, Some(s)))
            .chain(std::iter::once((self.cols.len(), None)))
        {
            for i in self.rcols.len() .. x {
                let len = match self.cols[i] {
                    TrackSizing::Linear(l) => {
                        l.resolve(self.regions.base.get(self.cross))
                    }
                    TrackSizing::Fractional(f) => {
                        if col_frac == 0.0 {
                            Length::zero()
                        } else {
                            let res: Length = (available - total) * (f.get() / col_frac);
                            if res.is_finite() { res } else { Length::zero() }
                        }
                    }
                    TrackSizing::Auto => unreachable!("x is an auto track"),
                };

                self.rcols.push(len);
            }

            if let Some(len) = len {
                self.rcols.push(len);
            }
        }

        // Determine non-`fr` row heights
        let mut total_frs = 0.0;
        let mut current = self.regions.current.get(self.main);

        for y in 0 .. self.rows.len() {
            let resolved = match self.rows[y] {
                TrackSizing::Linear(l) => {
                    Some(l.resolve(self.regions.base.get(self.main)))
                }
                TrackSizing::Auto => {
                    let mut max = Length::zero();
                    for (x, len) in self.rcols.iter().enumerate() {
                        if let Cell::Node(node) = self.get(x, y) {
                            let regions = Regions::one(
                                Gen::new(*len, current).to_size(self.main),
                                Spec::splat(false),
                            );
                            let frame = node.layout(ctx, &regions).remove(0);
                            max = max.max(frame.size.get(self.main));
                        }
                    }
                    Some(max)
                }
                TrackSizing::Fractional(f) => {
                    total_frs += f.get();
                    None
                }
            };

            if let Some(resolved) = resolved {
                while !current.fits(resolved) && !self.regions.in_full_last() {
                    self.finish_region(ctx, total_frs);
                    current = self.regions.current.get(self.main);
                    total_frs = 0.0;
                }
                current -= resolved;
            }

            self.rrows.push((y, resolved));
        }

        self.finish_region(ctx, total_frs);
        self.finished
    }

    fn finish_region(&mut self, ctx: &mut LayoutContext, total_frs: f64) {
        let mut pos = Gen::splat(Length::zero());
        let mut frame = Frame::new(Size::zero(), Length::zero());
        let mut total_cross = Length::zero();
        let mut total_main = Length::zero();

        for (x, &w) in self.rcols.iter().enumerate() {
            let total: Length = self.rrows.iter().filter_map(|(_, x)| *x).sum();
            let available = self.regions.current.get(self.main) - total;
            total_cross += w;

            for (y, h) in self.rrows.iter() {
                let element = self.get(x, *y);
                let h = if let Some(len) = h {
                    *len
                } else if let TrackSizing::Fractional(f) = self.rows[*y] {
                    if total_frs > 0.0 {
                        let res = available * (f.get() / total_frs);
                        if res.is_finite() { res } else { Length::zero() }
                    } else {
                        Length::zero()
                    }
                } else {
                    unreachable!("non-fractional tracks are already resolved");
                };

                if x == 0 {
                    total_main += h;
                }

                if let Cell::Node(n) = element {
                    let regions = Regions::one(
                        Gen::new(w, h).to_size(self.main),
                        Spec::splat(false),
                    );
                    let item = n.layout(ctx, &regions).remove(0);
                    frame.push_frame(pos.to_point(self.main), item);
                }

                pos.main += h;
            }
            pos.main = Length::zero();
            pos.cross += w;
        }

        frame.size = Gen::new(total_cross, total_main).to_size(self.main);
        frame.baseline = frame.size.height;

        self.rrows.clear();
        self.regions.next();
        self.finished.push(frame);
    }

    fn get(&self, x: usize, y: usize) -> &Cell<'a> {
        assert!(x < self.cols.len());
        assert!(y < self.rows.len());
        self.cells.get(y * self.cols.len() + x).unwrap()
    }
}

/// A list of track sizing definitions.
#[derive(Default, Debug, Clone, PartialEq, Hash)]
pub struct Tracks(pub Vec<TrackSizing>);

impl Tracks {
    /// Get the sizing for the track at the given `idx`.
    fn get(&self, idx: usize) -> TrackSizing {
        self.0
            .get(idx)
            .or(self.0.last())
            .copied()
            .unwrap_or(TrackSizing::Auto)
    }
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
