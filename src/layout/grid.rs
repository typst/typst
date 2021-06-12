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

#[derive(Debug)]
struct GridLayouter<'a> {
    cross: SpecAxis,
    main: SpecAxis,
    cols: Vec<TrackSizing>,
    rows: Vec<TrackSizing>,
    cells: Vec<Cell<'a>>,
    regions: Regions,
    rcols: Vec<Length>,
    rrows: Vec<(usize, Option<Length>, Option<Vec<Option<Frame>>>)>,
    finished: Vec<Frame>,
}

#[derive(Debug)]
enum Cell<'a> {
    Node(&'a AnyNode),
    Gutter,
}

impl<'a> GridLayouter<'a> {
    fn new(grid: &'a GridNode, regions: Regions) -> Self {
        let cross = grid.dirs.cross.axis();
        let main = grid.dirs.main.axis();

        let mut cols = vec![];
        let mut rows = vec![];
        let mut cells = vec![];

        // A grid always needs to have at least one column.
        let content_cols = grid.tracks.cross.len().max(1);

        // Create at least as many rows as specified and also at least as many
        // as necessary to place each item.
        let content_rows = {
            let len = grid.children.len();
            let specified = grid.tracks.main.len();
            let necessary = len / content_cols + (len % content_cols).clamp(0, 1);
            specified.max(necessary)
        };

        // Collect the track sizing for all columns, including gutter columns.
        for i in 0 .. content_cols {
            cols.push(grid.tracks.cross.get_or_last(i));
            if i < content_cols - 1 {
                cols.push(grid.gutter.cross.get_or_last(i));
            }
        }

        // Collect the track sizing for all rows, including gutter rows.
        for i in 0 .. content_rows {
            rows.push(grid.tracks.main.get_or_last(i));
            if i < content_rows - 1 {
                rows.push(grid.gutter.main.get_or_last(i));
            }
        }

        // Build up the matrix of cells, including gutter cells.
        for (i, item) in grid.children.iter().enumerate() {
            cells.push(Cell::Node(item));

            let row = i / content_cols;
            let col = i % content_cols;

            if col < content_cols - 1 {
                // Push gutter after each child.
                cells.push(Cell::Gutter);
            } else if row < content_rows - 1 {
                // Except for the last child of each row.
                // There we push a gutter row.
                for _ in 0 .. cols.len() {
                    cells.push(Cell::Gutter);
                }
            }
        }

        // Fill the thing up.
        while cells.len() < cols.len() * rows.len() {
            cells.push(Cell::Gutter);
        }

        Self {
            cross,
            main,
            cols,
            rows,
            cells,
            regions,
            rcols: vec![],
            rrows: vec![],
            finished: vec![],
        }
    }

    fn layout(mut self, ctx: &mut LayoutContext) -> Vec<Frame> {
        self.rcols = self.resolve_columns(ctx);
        self.layout_rows(ctx);
        self.finished
    }

    /// Determine the size of all columns.
    fn resolve_columns(&self, ctx: &mut LayoutContext) -> Vec<Length> {
        let current = self.regions.current.to_gen(self.main);
        let base = self.regions.base.to_gen(self.main);

        // Prepare vector for resolved column lengths.
        let mut rcols = vec![Length::zero(); self.cols.len()];

        // - Sum of sizes of resolved linear tracks,
        // - Sum of fractions of all fractional tracks,
        // - Sum of sizes of resolved (through layouting) auto tracks,
        // - Number of auto tracks.
        let mut linear = Length::zero();
        let mut fr = Fractional::zero();
        let mut auto = Length::zero();
        let mut auto_count = 0;

        // Resolve size of linear columns and compute the sum of all fractional
        // tracks.
        for (&col, rcol) in self.cols.iter().zip(&mut rcols) {
            match col {
                TrackSizing::Auto => {}
                TrackSizing::Linear(v) => {
                    let resolved = v.resolve(base.cross);
                    *rcol = resolved;
                    linear += resolved;
                }
                TrackSizing::Fractional(v) => fr += v,
            }
        }

        // Size available to auto columns (not used by fixed-size columns).
        let available = current.cross - linear;
        if available <= Length::zero() {
            return rcols;
        }

        // Resolve size of auto columns by laying out all cells in those
        // columns, measuring them and finding the largest one.
        for (x, (&col, rcol)) in self.cols.iter().zip(&mut rcols).enumerate() {
            if col == TrackSizing::Auto {
                let mut resolved = Length::zero();

                for (y, &row) in self.rows.iter().enumerate() {
                    if let Cell::Node(node) = self.get(x, y) {
                        // Set the correct main size if the row is fixed-size.
                        let main = match row {
                            TrackSizing::Linear(v) => v.resolve(base.main),
                            _ => current.main,
                        };

                        let size = Gen::new(available, main).to_size(self.main);
                        let regions = Regions::one(size, Spec::splat(false));
                        let frame = node.layout(ctx, &regions).remove(0);
                        resolved = resolved.max(frame.size.get(self.cross))
                    }
                }

                *rcol = resolved;
                auto += resolved;
                auto_count += 1;
            }
        }

        // If there is remaining space, distribute it to fractional columns,
        // otherwise shrink auto columns.
        let remaining = available - auto;
        if remaining >= Length::zero() {
            for (&col, rcol) in self.cols.iter().zip(&mut rcols) {
                if let TrackSizing::Fractional(v) = col {
                    let ratio = v / fr;
                    if ratio.is_finite() {
                        *rcol = ratio * remaining;
                    }
                }
            }
        } else {
            // The fair share each auto column may have.
            let fair = available / auto_count as f64;

            // The number of overlarge auto columns and the space that will be
            // equally redistributed to them.
            let mut overlarge: usize = 0;
            let mut redistribute = available;

            for (&col, rcol) in self.cols.iter().zip(&mut rcols) {
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
            if overlarge > 0 {
                for (&col, rcol) in self.cols.iter().zip(&mut rcols) {
                    if col == TrackSizing::Auto && *rcol > fair {
                        *rcol = share;
                    }
                }
            }
        }

        rcols
    }

    fn layout_rows(&mut self, ctx: &mut LayoutContext) {
        // Determine non-`fr` row heights
        let mut total_frs = 0.0;
        let mut current = self.regions.current.get(self.main);

        for y in 0 .. self.rows.len() {
            let resolved = match self.rows[y] {
                TrackSizing::Linear(l) => {
                    (Some(l.resolve(self.regions.base.get(self.main))), None)
                }
                TrackSizing::Auto => {
                    let mut max = Length::zero();
                    let mut local_max = max;
                    let mut multi_region = false;
                    let mut last_size = vec![];
                    for (x, &col_size) in self.rcols.iter().enumerate() {
                        if let Cell::Node(node) = self.get(x, y) {
                            let colsize = Gen::new(col_size, current).to_size(self.main);

                            let mut regions = self.regions.map(|mut s| {
                                *s.get_mut(self.cross) = col_size;
                                s
                            });

                            regions.base = colsize;
                            regions.current = colsize;
                            regions.expand = Spec::splat(false);

                            let mut frames = node.layout(ctx, &regions);
                            multi_region |= frames.len() > 1;
                            last_size.push((
                                frames.len() - 1,
                                frames.last().unwrap().size.get(self.main),
                            ));
                            let frame = frames.remove(0);
                            local_max = local_max.max(frame.size.get(self.main));

                            if !multi_region {
                                max = local_max;
                            }
                        } else {
                            last_size.push((0, Length::zero()))
                        }
                    }

                    let overshoot = if multi_region {
                        self.rrows.push((y, Some(local_max), None));
                        let res = self.finish_region(ctx, total_frs, Some(last_size));
                        max = if let Some(overflow) = res.as_ref() {
                            overflow
                                .iter()
                                .filter_map(|x| x.as_ref())
                                .map(|x| x.size.get(self.main))
                                .max()
                                .unwrap_or(Length::zero())
                        } else {
                            local_max
                        };

                        current = self.regions.current.get(self.main);
                        total_frs = 0.0;
                        if res.is_none() {
                            continue;
                        }

                        res
                    } else {
                        None
                    };

                    // If multi-region results: finish_regions, returning
                    // the last non-set frames.
                    (Some(max), overshoot)
                }
                TrackSizing::Fractional(f) => {
                    total_frs += f.get();
                    (None, None)
                }
            };

            if let (Some(resolved), _) = resolved {
                while !current.fits(resolved) && !self.regions.in_full_last() {
                    self.finish_region(ctx, total_frs, None);
                    current = self.regions.current.get(self.main);
                    total_frs = 0.0;
                }
                current -= resolved;
            }

            self.rrows.push((y, resolved.0, resolved.1));
        }

        self.finish_region(ctx, total_frs, None);
    }

    fn finish_region(
        &mut self,
        ctx: &mut LayoutContext,
        total_frs: f64,
        multiregion_sizing: Option<Vec<(usize, Length)>>,
    ) -> Option<Vec<Option<Frame>>> {
        if self.rrows.is_empty() {
            return None;
        }

        let mut pos = Gen::splat(Length::zero());
        let frame = Frame::new(Size::zero(), Length::zero());
        let mut total_cross = Length::zero();
        let mut total_main = Length::zero();
        let mut max_regions = 0;
        let mut collected_frames = if multiregion_sizing.is_some() {
            Some(vec![None; self.rcols.len()])
        } else {
            None
        };

        self.finished.push(frame);

        let frame_len = self.finished.len();

        let total_row_height: Length = self.rrows.iter().filter_map(|(_, x, _)| *x).sum();

        for &(y, h, ref layouted) in self.rrows.iter().as_ref() {
            let last = self.rrows.last().map_or(false, |(o, _, _)| &y == o);
            let available = self.regions.current.get(self.main) - total_row_height;
            let h = if let Some(len) = h {
                len
            } else if let TrackSizing::Fractional(f) = self.rows[y] {
                if total_frs > 0.0 {
                    let res = available * (f.get() / total_frs);
                    if res.is_finite() { res } else { Length::zero() }
                } else {
                    Length::zero()
                }
            } else {
                unreachable!("non-fractional tracks are already resolved");
            };
            total_main += h;

            if let Some(layouted) = layouted {
                for (col_index, frame) in layouted.into_iter().enumerate() {
                    if let Some(frame) = frame {
                        self.finished
                            .get_mut(frame_len - 1)
                            .unwrap()
                            .push_frame(pos.to_point(self.main), frame.clone());
                    }
                    pos.cross += self.rcols[col_index];
                }
            } else {
                let mut overshoot_columns = vec![];
                for (x, &w) in self.rcols.iter().enumerate() {
                    let element = self.get(x, y);

                    if y == 0 {
                        total_cross += w;
                    }

                    if let Cell::Node(n) = element {
                        let region_size = Gen::new(w, h).to_size(self.main);
                        let regions = if last {
                            if let Some(last_sizes) = multiregion_sizing.as_ref() {
                                let mut regions = self.regions.map(|mut s| {
                                    *s.get_mut(self.cross) = w;
                                    s
                                });

                                regions.base = region_size;
                                regions.current = region_size;
                                regions.expand = Spec::splat(true);

                                let (last_region, last_size) = last_sizes[x];
                                regions.unique_regions(last_region + 1);
                                *regions
                                    .nth_mut(last_region)
                                    .unwrap()
                                    .get_mut(self.main) = last_size;
                                regions
                            } else {
                                Regions::one(region_size, Spec::splat(true))
                            }
                        } else {
                            Regions::one(region_size, Spec::splat(true))
                        };
                        let mut items = n.layout(ctx, &regions);
                        let item = items.remove(0);

                        if last && multiregion_sizing.is_some() {
                            max_regions = max_regions.max(items.len());
                            overshoot_columns.push((x, items));
                        } else {
                            assert_eq!(items.len(), 0);
                        }

                        self.finished
                            .get_mut(frame_len - 1)
                            .unwrap()
                            .push_frame(pos.to_point(self.main), item);
                    }

                    pos.cross += w;
                }

                if overshoot_columns.iter().any(|(_, items)| !items.is_empty()) {
                    for (x, col) in overshoot_columns {
                        let mut cross_offset = Length::zero();
                        for col in 0 .. x {
                            cross_offset += self.rcols[col];
                        }


                        let collected_frames = collected_frames.as_mut().unwrap();
                        *collected_frames.get_mut(x).unwrap() =
                            col.get(max_regions - 1).cloned();

                        for (cell_index, subcell) in col.into_iter().enumerate() {
                            if cell_index >= max_regions - 1 {
                                continue;
                            }
                            let frame = if let Some(frame) =
                                self.finished.get_mut(frame_len + cell_index)
                            {
                                frame
                            } else {
                                let frame = Frame::new(Size::zero(), Length::zero());
                                // The previous frame always exists: either the
                                // last iteration created it or it is the normal
                                // frame.
                                self.finished.push(frame);
                                self.finished.last_mut().unwrap()
                            };
                            let pos = Gen::new(cross_offset, Length::zero());
                            frame
                                .size
                                .get_mut(self.cross)
                                .set_max(pos.cross + subcell.size.get(self.cross));
                            frame
                                .size
                                .get_mut(self.main)
                                .set_max(subcell.size.get(self.main));
                            frame.baseline = frame.size.height;
                            frame.push_frame(pos.to_point(self.main), subcell);
                        }
                    }
                }
            }

            pos.cross = Length::zero();
            pos.main += h;
        }

        let frame = self.finished.get_mut(frame_len - 1).unwrap();
        frame.size = Gen::new(total_cross, total_main).to_size(self.main);
        frame.baseline = frame.size.height;

        self.rrows.clear();
        for _ in 0 .. (max_regions.max(1)) {
            self.regions.next();
        }

        if let Some(frames) = collected_frames.as_ref() {
            if frames.iter().all(|i| i.is_none()) {
                collected_frames = None;
            }
        }

        collected_frames
    }

    fn get(&self, x: usize, y: usize) -> &Cell<'a> {
        assert!(x < self.cols.len());
        assert!(y < self.rows.len());
        self.cells.get(y * self.cols.len() + x).unwrap()
    }
}

trait TracksExt {
    /// Get the sizing for the track at the given `idx` or fallback to the
    /// last defined track or `auto`.
    fn get_or_last(&self, idx: usize) -> TrackSizing;
}

impl TracksExt for Vec<TrackSizing> {
    fn get_or_last(&self, idx: usize) -> TrackSizing {
        self.get(idx).or(self.last()).copied().unwrap_or(TrackSizing::Auto)
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
