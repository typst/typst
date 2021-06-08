use std::usize;

use super::*;
use crate::library::GridUnits;

/// A node that stacks its children.
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct GridNode {
    /// The `main` and `cross` directions of this stack.
    ///
    /// The children are stacked along the `main` direction. The `cross`
    /// direction is required for aligning the children.
    pub dir: Dir,
    /// The nodes to be stacked.
    pub children: Vec<AnyNode>,
    pub tracks: Gen<GridUnits>,
    pub gutter: Gen<GridUnits>,
}

impl Layout for GridNode {
    fn layout(&self, ctx: &mut LayoutContext, regions: &Regions) -> Vec<Frame> {
        let layout = GridLayouter::new(self, regions).layout(ctx);
        layout
    }
}

#[derive(Debug)]
enum GridItem<'a> {
    Node(&'a AnyNode),
    Gutter,
}

#[derive(Debug)]
struct GridLayouter<'a> {
    items: Vec<GridItem<'a>>,
    cols: Vec<TrackSizing>,
    rows: Vec<TrackSizing>,
    region: Regions,
    dir: Dir,
    rrows: Vec<(usize, Option<Length>)>,
    rcols: Vec<Length>,
    frames: Vec<Frame>,
}

impl<'a> GridLayouter<'a> {
    fn new(
        grid: &'a GridNode,
        regions: &Regions,
    ) -> Self {
        let mut items = vec![];
        let mut col_sizes = vec![];
        let mut row_sizes = vec![];
        let cols = grid.tracks.cross.0.len();
        // Create at least as many rows as specified and a row to fit every item.
        let rows = if cols > 0 {
            let res = grid
            .tracks
            .main
            .0
            .len()
            .max(grid.children.len() / cols + (grid.children.len() % cols).clamp(0, 1));
            res
        } else {
            0
        };

        for (i, col_size) in grid.tracks.cross.0.iter().enumerate() {
            let last = i == cols - 1;
            col_sizes.push(*col_size);

            if !last {
                let gutter = grid.gutter.cross.get(i);
                col_sizes.push(gutter);
            }
        }

        for (i, row_size) in (0 .. rows).map(|i| (i, grid.tracks.main.get(i))) {
            let last = i == rows - 1;
            row_sizes.push(row_size);

            if !last {
                let gutter = grid.gutter.main.get(i);
                row_sizes.push(gutter);
            }
        }

        for (i, item) in grid.children.iter().enumerate() {
            if cols == 0 {
                break;
            }

            let row = i / cols;
            let col = i % cols;

            items.push(GridItem::Node(item));

            if col != cols - 1 {
                // Push gutter
                items.push(GridItem::Gutter);
            } else if row != rows - 1 {
                // Push gutter row.
                for _ in 0 .. col_sizes.len() {
                    items.push(GridItem::Gutter);
                }
            }
        }

        // Fill the thing up
        while items.len() < col_sizes.len() * row_sizes.len() {
            items.push(GridItem::Gutter)
        }

        GridLayouter {
            cols: col_sizes,
            rows: row_sizes,
            region: regions.clone(),
            dir: grid.dir,
            items,
            rrows: vec![],
            rcols: vec![],
            frames: vec![],
        }
    }

    fn get(&self, x: usize, y: usize) -> &GridItem<'_> {
        assert!(x < self.cols.len());
        assert!(y < self.rows.len());
        let row_cmp = y * self.cols.len();

        self.items.get(row_cmp + x).unwrap()
    }

    fn main(&self) -> SpecAxis {
        self.dir.axis().other()
    }

    fn cross(&self) -> SpecAxis {
        self.dir.axis()
    }

    fn finish_region(&mut self, ctx: &mut LayoutContext, total_frs: f64) {
        let mut pos = Gen::splat(Length::zero());
        let pos2point = |mut pos: Gen<Length>| {
            if !self.dir.is_positive() {
                pos.cross = -pos.cross;
            }
            pos.switch(self.main()).to_point()
        };
        let mut frame = Frame::new(Size::zero(), Length::zero());
        let mut total_cross = Length::zero();
        let mut total_main = Length::zero();

        for (x, &w) in self.rcols.iter().enumerate() {
            let total: Length = self.rrows.iter().filter_map(|(_, x)| *x).sum();
            let available = self.region.current.get(self.main()) - total;
            total_cross += w;

            for (y, h) in self.rrows.iter() {
                let element = self.get(x, *y);
                let h = if let Some(len) = h {
                    *len
                } else {
                    if let TrackSizing::Fractional(f) = self.rows[*y] {
                        if total_frs > 0.0 {
                            let res = available * (f.get() / total_frs);
                            if res.is_finite() {
                                res
                            } else {
                                Length::zero()
                            }
                        } else {
                            Length::zero()
                        }
                    } else {
                        unreachable!()
                    }
                };
                if x == 0 {
                    total_main += h;
                }

                if let GridItem::Node(n) = element {
                    let item = n.layout(ctx, &Regions::one(Gen::new(w, h).switch(self.main()).to_size(), Spec::splat(false))).remove(0);
                    frame.push_frame(pos2point(pos), item);
                }

                pos.main += h;
            }
            pos.main = Length::zero();
            pos.cross += self.dir.factor() as f64 * w;
        }

        if !self.dir.is_positive() {
            frame.translate(Gen::new(total_cross, Length::zero()).switch(self.main()).to_point());
        }

        frame.size = Gen::new(total_cross, total_main).switch(self.main()).to_size();
        frame.baseline = frame.size.height;

        self.frames.push(frame);

        self.rrows.clear();
        self.region.next();
    }

    fn layout(mut self, ctx: &mut LayoutContext) -> Vec<Frame> {
        // Shrink area by linear sizing.
        let mut available = self.region.current.get(self.cross());
        available -= self
            .cols
            .iter()
            .filter_map(|x| match x {
                TrackSizing::Linear(l) => Some(l.resolve(self.region.base.get(self.cross()))),
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

        // For each of the auto columns, lay out all elements with `preliminary_length`
        // rows and build max.
        for x in auto_columns {
            let mut max = Length::zero();
            for (y, row_height) in
                self.rows.iter().enumerate().map(|(y, s)| {
                    (y, s.preliminary_length(self.region.base.get(self.main())))
                })
            {
                let item = self.get(x, y);
                let size =
                    Gen::new(self.region.current.get(self.cross()), row_height).switch(self.main()).to_size();
                let region = Regions::one(size, Spec::splat(false));
                match item {
                    GridItem::Node(n) => {
                        max = max.max(
                            n.layout(ctx, &region).first().unwrap().size.get(self.cross()),
                        )
                    }
                    GridItem::Gutter => {}
                }
            }

            col_width.push((x, max));
        }

        // If accumulated auto column size exceeds available size, redistribute space
        // proportionally amongst elements that exceed their size allocation.
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
        for (x, len) in col_width.into_iter().map(|(x, s)| (x, Some(s))).chain(std::iter::once((self.cols.len(), None))) {
            for i in self.rcols.len() .. x {
                let len = match self.cols[i] {
                    TrackSizing::Linear(l) => l.resolve(self.region.base.get(self.cross())),
                    TrackSizing::Fractional(f) => {
                        if col_frac == 0.0 {
                            Length::zero()
                        } else {
                            let res: Length = (available - total) * (f.get() / col_frac);
                            if res.is_finite() {
                                res
                            } else {
                                Length::zero()
                            }
                        }
                    }
                    TrackSizing::Auto => unreachable!(),
                };

                self.rcols.push(len);
            }

            if let Some(len) = len {
                self.rcols.push(len);
            }
        }

        // Determine non-`fr` row heights
        let mut total_frs = 0.0;
        let mut current = self.region.current.get(self.main());

        for y in 0..self.rows.len() {
            let height = &self.rows[y];
            let resolved = match height {
                TrackSizing::Linear(l) => Some(l.resolve(self.region.base.get(self.main()))),
                TrackSizing::Auto => {
                    let mut max = Length::zero();
                    for (x, len) in self.rcols.iter().enumerate() {
                        let node = self.get(x, y);
                        if let GridItem::Node(node) = node {
                            let frames = node.layout(
                                ctx,
                                &Regions::one(
                                    Gen::new(*len, current)
                                        .switch(self.main())
                                        .to_size(),
                                    Spec::splat(false),
                                ),
                            );
                            max = max.max(frames.first().unwrap().size.get(self.main()));
                        }
                    }
                    Some(max)
                }
                TrackSizing::Fractional(f) => {
                    total_frs += f.get();
                    None
                },
            };

            if let Some(resolved) = resolved {
                while !current.fits(resolved) && !self.region.in_full_last() {
                    self.finish_region(ctx, total_frs);
                    current = self.region.current.get(self.main());
                    total_frs = 0.0;
                }
                current -= resolved;
            }

            self.rrows.push((y, resolved));
        }

        self.finish_region(ctx, total_frs);
        self.frames
    }
}

impl From<GridNode> for AnyNode {
    fn from(grid: GridNode) -> Self {
        Self::new(grid)
    }
}
