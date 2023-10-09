use crate::prelude::*;
use crate::text::TextElem;

use super::Sizing;

/// Arranges content in a grid.
///
/// The grid element allows you to arrange content in a grid. You can define the
/// number of rows and columns, as well as the size of the gutters between them.
/// There are multiple sizing modes for columns and rows that can be used to
/// create complex layouts.
///
/// The sizing of the grid is determined by the track sizes specified in the
/// arguments. Because each of the sizing parameters accepts the same values, we
/// will explain them just once, here. Each sizing argument accepts an array of
/// individual track sizes. A track size is either:
///
/// - `{auto}`: The track will be sized to fit its contents. It will be at most
///   as large as the remaining space. If there is more than one `{auto}` track
///   which, and together they claim more than the available space, the `{auto}`
///   tracks will fairly distribute the available space among themselves.
///
/// - A fixed or relative length (e.g. `{10pt}` or `{20% - 1cm}`): The track
///   will be exactly of this size.
///
/// - A fractional length (e.g. `{1fr}`): Once all other tracks have been sized,
///   the remaining space will be divided among the fractional tracks according
///   to their fractions. For example, if there are two fractional tracks, each
///   with a fraction of `{1fr}`, they will each take up half of the remaining
///   space.
///
/// To specify a single track, the array can be omitted in favor of a single
/// value. To specify multiple `{auto}` tracks, enter the number of tracks
/// instead of an array. For example, `columns:` `{3}` is equivalent to
/// `columns:` `{(auto, auto, auto)}`.
///
/// # Example
/// ```example
/// #set text(10pt, style: "italic")
/// #let cell = rect.with(
///   inset: 8pt,
///   fill: rgb("e4e5ea"),
///   width: 100%,
///   radius: 6pt
/// )
/// #grid(
///   columns: (60pt, 1fr, 60pt),
///   rows: (60pt, auto),
///   gutter: 3pt,
///   cell(height: 100%)[Easy to learn],
///   cell(height: 100%)[Great output],
///   cell(height: 100%)[Intuitive],
///   cell[Our best Typst yet],
///   cell[
///     Responsive design in print
///     for everyone
///   ],
///   cell[One more thing...],
/// )
/// ```
#[elem(Layout)]
pub struct GridElem {
    /// The column sizes.
    ///
    /// Either specify a track size array or provide an integer to create a grid
    /// with that many `{auto}`-sized columns. Note that opposed to rows and
    /// gutters, providing a single track size will only ever create a single
    /// column.
    pub columns: TrackSizings,

    /// The row sizes.
    ///
    /// If there are more cells than fit the defined rows, the last row is
    /// repeated until there are no more cells.
    pub rows: TrackSizings,

    /// The gaps between rows & columns.
    ///
    /// If there are more gutters than defined sizes, the last gutter is repeated.
    #[external]
    pub gutter: TrackSizings,

    /// The gaps between columns. Takes precedence over `gutter`.
    #[parse(
        let gutter = args.named("gutter")?;
        args.named("column-gutter")?.or_else(|| gutter.clone())
    )]
    pub column_gutter: TrackSizings,

    /// The gaps between rows. Takes precedence over `gutter`.
    #[parse(args.named("row-gutter")?.or_else(|| gutter.clone()))]
    pub row_gutter: TrackSizings,

    /// The contents of the grid cells.
    ///
    /// The cells are populated in row-major order.
    #[variadic]
    pub children: Vec<Content>,
}

impl Layout for GridElem {
    #[tracing::instrument(name = "GridElem::layout", skip_all)]
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        // Prepare grid layout by unifying content and gutter tracks.
        let cells = self.children();
        let layouter = GridLayouter::new(
            Axes::new(&self.columns(styles).0, &self.rows(styles).0),
            Axes::new(&self.column_gutter(styles).0, &self.row_gutter(styles).0),
            &cells,
            regions,
            styles,
            self.span(),
        );

        // Measure the columns and layout the grid row-by-row.
        Ok(layouter.layout(vt)?.fragment)
    }
}

/// Track sizing definitions.
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct TrackSizings(pub Vec<Sizing>);

cast! {
    TrackSizings,
    self => self.0.into_value(),
    sizing: Sizing => Self(vec![sizing]),
    count: NonZeroUsize => Self(vec![Sizing::Auto; count.get()]),
    values: Array => Self(values.into_iter().map(Value::cast).collect::<StrResult<_>>()?),
}

/// Performs grid layout.
pub struct GridLayouter<'a> {
    /// The grid cells.
    cells: &'a [Content],
    /// Whether this is an RTL grid.
    is_rtl: bool,
    /// Whether this grid has gutters.
    has_gutter: bool,
    /// The column tracks including gutter tracks.
    cols: Vec<Sizing>,
    /// The row tracks including gutter tracks.
    rows: Vec<Sizing>,
    /// The regions to layout children into.
    regions: Regions<'a>,
    /// The inherited styles.
    styles: StyleChain<'a>,
    /// Resolved column sizes.
    rcols: Vec<Abs>,
    /// The sum of `rcols`.
    width: Abs,
    /// Resolve row sizes, by region.
    rrows: Vec<Vec<RowPiece>>,
    /// Rows in the current region.
    lrows: Vec<Row>,
    /// The initial size of the current region before we started subtracting.
    initial: Size,
    /// Frames for finished regions.
    finished: Vec<Frame>,
    /// The span of the grid element.
    span: Span,
}

/// The resulting sizes of columns and rows in a grid.
#[derive(Debug)]
pub struct GridLayout {
    /// The fragment.
    pub fragment: Fragment,
    /// The column widths.
    pub cols: Vec<Abs>,
    /// The heights of the resulting rows segments, by region.
    pub rows: Vec<Vec<RowPiece>>,
}

/// Details about a resulting row piece.
#[derive(Debug)]
pub struct RowPiece {
    /// The height of the segment.
    pub height: Abs,
    /// The index of the row.
    pub y: usize,
}

/// Produced by initial row layout, auto and relative rows are already finished,
/// fractional rows not yet.
enum Row {
    /// Finished row frame of auto or relative row with y index.
    Frame(Frame, usize),
    /// Fractional row with y index.
    Fr(Fr, usize),
}

impl<'a> GridLayouter<'a> {
    /// Create a new grid layouter.
    ///
    /// This prepares grid layout by unifying content and gutter tracks.
    pub fn new(
        tracks: Axes<&[Sizing]>,
        gutter: Axes<&[Sizing]>,
        cells: &'a [Content],
        regions: Regions<'a>,
        styles: StyleChain<'a>,
        span: Span,
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

        let has_gutter = gutter.any(|tracks| !tracks.is_empty());
        let auto = Sizing::Auto;
        let zero = Sizing::Rel(Rel::zero());
        let get_or = |tracks: &[_], idx, default| {
            tracks.get(idx).or(tracks.last()).copied().unwrap_or(default)
        };

        // Collect content and gutter columns.
        for x in 0..c {
            cols.push(get_or(tracks.x, x, auto));
            if has_gutter {
                cols.push(get_or(gutter.x, x, zero));
            }
        }

        // Collect content and gutter rows.
        for y in 0..r {
            rows.push(get_or(tracks.y, y, auto));
            if has_gutter {
                rows.push(get_or(gutter.y, y, zero));
            }
        }

        // Remove superfluous gutter tracks.
        if has_gutter {
            cols.pop();
            rows.pop();
        }

        // Reverse for RTL.
        let is_rtl = TextElem::dir_in(styles) == Dir::RTL;
        if is_rtl {
            cols.reverse();
        }

        // We use these regions for auto row measurement. Since at that moment,
        // columns are already sized, we can enable horizontal expansion.
        let mut regions = regions;
        regions.expand = Axes::new(true, false);

        Self {
            cells,
            is_rtl,
            has_gutter,
            rows,
            regions,
            styles,
            rcols: vec![Abs::zero(); cols.len()],
            cols,
            width: Abs::zero(),
            rrows: vec![],
            lrows: vec![],
            initial: regions.size,
            finished: vec![],
            span,
        }
    }

    /// Determines the columns sizes and then layouts the grid row-by-row.
    pub fn layout(mut self, vt: &mut Vt) -> SourceResult<GridLayout> {
        self.measure_columns(vt)?;

        for y in 0..self.rows.len() {
            // Skip to next region if current one is full, but only for content
            // rows, not for gutter rows.
            if self.regions.is_full() && (!self.has_gutter || y % 2 == 0) {
                self.finish_region(vt)?;
            }

            match self.rows[y] {
                Sizing::Auto => self.layout_auto_row(vt, y)?,
                Sizing::Rel(v) => self.layout_relative_row(vt, v, y)?,
                Sizing::Fr(v) => self.lrows.push(Row::Fr(v, y)),
            }
        }

        self.finish_region(vt)?;

        Ok(GridLayout {
            fragment: Fragment::frames(self.finished),
            cols: self.rcols,
            rows: self.rrows,
        })
    }

    /// Determine all column sizes.
    #[tracing::instrument(name = "GridLayouter::measure_columns", skip_all)]
    fn measure_columns(&mut self, vt: &mut Vt) -> SourceResult<()> {
        // Sum of sizes of resolved relative tracks.
        let mut rel = Abs::zero();

        // Sum of fractions of all fractional tracks.
        let mut fr = Fr::zero();

        // Resolve the size of all relative columns and compute the sum of all
        // fractional tracks.
        for (&col, rcol) in self.cols.iter().zip(&mut self.rcols) {
            match col {
                Sizing::Auto => {}
                Sizing::Rel(v) => {
                    let resolved =
                        v.resolve(self.styles).relative_to(self.regions.base().x);
                    *rcol = resolved;
                    rel += resolved;
                }
                Sizing::Fr(v) => fr += v,
            }
        }

        // Size that is not used by fixed-size columns.
        let available = self.regions.size.x - rel;
        if available >= Abs::zero() {
            // Determine size of auto columns.
            let (auto, count) = self.measure_auto_columns(vt, available)?;

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
        self.width = self.rcols.iter().sum();

        Ok(())
    }

    /// Measure the size that is available to auto columns.
    fn measure_auto_columns(
        &mut self,
        vt: &mut Vt,
        available: Abs,
    ) -> SourceResult<(Abs, usize)> {
        let mut auto = Abs::zero();
        let mut count = 0;

        // Determine size of auto columns by laying out all cells in those
        // columns, measuring them and finding the largest one.
        for (x, &col) in self.cols.iter().enumerate() {
            if col != Sizing::Auto {
                continue;
            }

            let mut resolved = Abs::zero();
            for y in 0..self.rows.len() {
                if let Some(cell) = self.cell(x, y) {
                    // For relative rows, we can already resolve the correct
                    // base and for auto and fr we could only guess anyway.
                    let height = match self.rows[y] {
                        Sizing::Rel(v) => {
                            v.resolve(self.styles).relative_to(self.regions.base().y)
                        }
                        _ => self.regions.base().y,
                    };

                    let size = Size::new(available, height);
                    let pod = Regions::one(size, Axes::splat(false));
                    let frame = cell.measure(vt, self.styles, pod)?.into_frame();
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
            if let Sizing::Fr(v) = col {
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
                if col == Sizing::Auto && rcol <= fair && rcol > last {
                    redistribute -= rcol;
                    overlarge -= 1;
                    changed = true;
                }
            }
        }

        // Redistribute space fairly among overlarge columns.
        for (&col, rcol) in self.cols.iter().zip(&mut self.rcols) {
            if col == Sizing::Auto && *rcol > fair {
                *rcol = fair;
            }
        }
    }

    /// Layout a row with automatic height. Such a row may break across multiple
    /// regions.
    fn layout_auto_row(&mut self, vt: &mut Vt, y: usize) -> SourceResult<()> {
        // Determine the size for each region of the row. If the first region
        // ends up empty for some column, skip the region and remeasure.
        let mut resolved = match self.measure_auto_row(vt, y, true)? {
            Some(resolved) => resolved,
            None => {
                self.finish_region(vt)?;
                self.measure_auto_row(vt, y, false)?.unwrap()
            }
        };

        // Nothing to layout.
        if resolved.is_empty() {
            return Ok(());
        }

        // Layout into a single region.
        if let &[first] = resolved.as_slice() {
            let frame = self.layout_single_row(vt, first, y)?;
            self.push_row(frame, y);
            return Ok(());
        }

        // Expand all but the last region.
        // Skip the first region if the space is eaten up by an fr row.
        let len = resolved.len();
        for (region, target) in self
            .regions
            .iter()
            .zip(&mut resolved[..len - 1])
            .skip(self.lrows.iter().any(|row| matches!(row, Row::Fr(..))) as usize)
        {
            target.set_max(region.y);
        }

        // Layout into multiple regions.
        let fragment = self.layout_multi_row(vt, &resolved, y)?;
        let len = fragment.len();
        for (i, frame) in fragment.into_iter().enumerate() {
            self.push_row(frame, y);
            if i + 1 < len {
                self.finish_region(vt)?;
            }
        }

        Ok(())
    }

    /// Measure the regions sizes of an auto row. The option is always `Some(_)`
    /// if `can_skip` is false.
    fn measure_auto_row(
        &mut self,
        vt: &mut Vt,
        y: usize,
        can_skip: bool,
    ) -> SourceResult<Option<Vec<Abs>>> {
        let mut resolved: Vec<Abs> = vec![];

        for (x, &rcol) in self.rcols.iter().enumerate() {
            if let Some(cell) = self.cell(x, y) {
                let mut pod = self.regions;
                pod.size.x = rcol;

                let frames = cell.measure(vt, self.styles, pod)?.into_frames();

                // Skip the first region if one cell in it is empty. Then,
                // remeasure.
                if let [first, rest @ ..] = frames.as_slice() {
                    if can_skip
                        && first.is_empty()
                        && rest.iter().any(|frame| !frame.is_empty())
                    {
                        return Ok(None);
                    }
                }

                let mut sizes = frames.iter().map(|frame| frame.height());
                for (target, size) in resolved.iter_mut().zip(&mut sizes) {
                    target.set_max(size);
                }

                // New heights are maximal by virtue of being new. Note that
                // this extend only uses the rest of the sizes iterator.
                resolved.extend(sizes);
            }
        }

        Ok(Some(resolved))
    }

    /// Layout a row with relative height. Such a row cannot break across
    /// multiple regions, but it may force a region break.
    fn layout_relative_row(
        &mut self,
        vt: &mut Vt,
        v: Rel<Length>,
        y: usize,
    ) -> SourceResult<()> {
        let resolved = v.resolve(self.styles).relative_to(self.regions.base().y);
        let frame = self.layout_single_row(vt, resolved, y)?;

        // Skip to fitting region.
        let height = frame.height();
        while !self.regions.size.y.fits(height) && !self.regions.in_last() {
            self.finish_region(vt)?;

            // Don't skip multiple regions for gutter and don't push a row.
            if self.has_gutter && y % 2 == 1 {
                return Ok(());
            }
        }

        self.push_row(frame, y);

        Ok(())
    }

    /// Layout a row with fixed height and return its frame.
    fn layout_single_row(
        &mut self,
        vt: &mut Vt,
        height: Abs,
        y: usize,
    ) -> SourceResult<Frame> {
        if !height.is_finite() {
            bail!(error!(self.span, "cannot create grid with infinite height"));
        }

        let mut output = Frame::soft(Size::new(self.width, height));
        let mut pos = Point::zero();

        for (x, &rcol) in self.rcols.iter().enumerate() {
            if let Some(cell) = self.cell(x, y) {
                let size = Size::new(rcol, height);
                let mut pod = Regions::one(size, Axes::splat(true));
                if self.rows[y] == Sizing::Auto {
                    pod.full = self.regions.full;
                }
                let frame = cell.layout(vt, self.styles, pod)?.into_frame();
                output.push_frame(pos, frame);
            }

            pos.x += rcol;
        }

        Ok(output)
    }

    /// Layout a row spanning multiple regions.
    fn layout_multi_row(
        &mut self,
        vt: &mut Vt,
        heights: &[Abs],
        y: usize,
    ) -> SourceResult<Fragment> {
        // Prepare frames.
        let mut outputs: Vec<_> = heights
            .iter()
            .map(|&h| Frame::soft(Size::new(self.width, h)))
            .collect();

        // Prepare regions.
        let size = Size::new(self.width, heights[0]);
        let mut pod = Regions::one(size, Axes::splat(true));
        pod.full = self.regions.full;
        pod.backlog = &heights[1..];

        // Layout the row.
        let mut pos = Point::zero();
        for (x, &rcol) in self.rcols.iter().enumerate() {
            if let Some(cell) = self.cell(x, y) {
                pod.size.x = rcol;

                // Push the layouted frames into the individual output frames.
                let fragment = cell.layout(vt, self.styles, pod)?;
                for (output, frame) in outputs.iter_mut().zip(fragment) {
                    output.push_frame(pos, frame);
                }
            }

            pos.x += rcol;
        }

        Ok(Fragment::frames(outputs))
    }

    /// Push a row frame into the current region.
    fn push_row(&mut self, frame: Frame, y: usize) {
        self.regions.size.y -= frame.height();
        self.lrows.push(Row::Frame(frame, y));
    }

    /// Finish rows for one region.
    fn finish_region(&mut self, vt: &mut Vt) -> SourceResult<()> {
        // Determine the height of existing rows in the region.
        let mut used = Abs::zero();
        let mut fr = Fr::zero();
        for row in &self.lrows {
            match row {
                Row::Frame(frame, _) => used += frame.height(),
                Row::Fr(v, _) => fr += *v,
            }
        }

        // Determine the size of the grid in this region, expanding fully if
        // there are fr rows.
        let mut size = Size::new(self.width, used).min(self.initial);
        if fr.get() > 0.0 && self.initial.y.is_finite() {
            size.y = self.initial.y;
        }

        // The frame for the region.
        let mut output = Frame::soft(size);
        let mut pos = Point::zero();
        let mut rrows = vec![];

        // Place finished rows and layout fractional rows.
        for row in std::mem::take(&mut self.lrows) {
            let (frame, y) = match row {
                Row::Frame(frame, y) => (frame, y),
                Row::Fr(v, y) => {
                    let remaining = self.regions.full - used;
                    let height = v.share(fr, remaining);
                    (self.layout_single_row(vt, height, y)?, y)
                }
            };

            let height = frame.height();
            output.push_frame(pos, frame);
            rrows.push(RowPiece { height, y });
            pos.y += height;
        }

        self.finished.push(output);
        self.rrows.push(rrows);
        self.regions.next();
        self.initial = self.regions.size;

        Ok(())
    }

    /// Get the content of the cell in column `x` and row `y`.
    ///
    /// Returns `None` if it's a gutter cell.
    #[track_caller]
    fn cell(&self, mut x: usize, y: usize) -> Option<&'a Content> {
        assert!(x < self.cols.len());
        assert!(y < self.rows.len());

        // Columns are reorder, but the cell slice is not.
        if self.is_rtl {
            x = self.cols.len() - 1 - x;
        }

        if self.has_gutter {
            // Even columns and rows are children, odd ones are gutter.
            if x % 2 == 0 && y % 2 == 0 {
                let c = 1 + self.cols.len() / 2;
                self.cells.get((y / 2) * c + x / 2)
            } else {
                None
            }
        } else {
            let c = self.cols.len();
            self.cells.get(y * c + x)
        }
    }
}
