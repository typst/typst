use std::num::NonZeroUsize;

use smallvec::{smallvec, SmallVec};

use crate::diag::{bail, At, SourceResult, StrResult};
use crate::engine::Engine;
use crate::foundations::{
    cast, elem, Array, CastInfo, Content, FromValue, Func, IntoValue, NativeElement,
    Reflect, Resolve, Smart, StyleChain, Value,
};
use crate::layout::{
    Abs, Align, AlignElem, Axes, Dir, Fr, Fragment, Frame, FrameItem, Layout, Length,
    Point, Regions, Rel, Sides, Size, Sizing,
};
use crate::syntax::Span;
use crate::text::TextElem;
use crate::util::Numeric;
use crate::visualize::{FixedStroke, Geometry, Paint, Stroke};

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
/// # Examples
/// The example below demonstrates the different track sizing options.
///
/// ```example
/// // We use `rect` to emphasize the
/// // area of cells.
/// #set rect(
///   inset: 8pt,
///   fill: rgb("e4e5ea"),
///   width: 100%,
/// )
///
/// #grid(
///   columns: (60pt, 1fr, 2fr),
///   rows: (auto, 60pt),
///   gutter: 3pt,
///   rect[Fixed width, auto height],
///   rect[1/3 of the remains],
///   rect[2/3 of the remains],
///   rect(height: 100%)[Fixed height],
///   image("tiger.jpg", height: 100%),
///   image("tiger.jpg", height: 100%),
/// )
/// ```
///
/// You can also [spread]($arguments/#spreading) an array of strings or content
/// into a grid to populate its cells.
///
/// ```example
/// #grid(
///   columns: 5,
///   gutter: 5pt,
///   ..range(25).map(str)
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
    #[borrowed]
    pub columns: TrackSizings,

    /// The row sizes.
    ///
    /// If there are more cells than fit the defined rows, the last row is
    /// repeated until there are no more cells.
    #[borrowed]
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
    #[borrowed]
    pub column_gutter: TrackSizings,

    /// The gaps between rows. Takes precedence over `gutter`.
    #[parse(args.named("row-gutter")?.or_else(|| gutter.clone()))]
    #[borrowed]
    pub row_gutter: TrackSizings,

    /// How to fill the cells.
    ///
    /// This can be a color or a function that returns a color. The function is
    /// passed the cells' column and row index, starting at zero. This can be
    /// used to implement striped grids.
    ///
    /// ```example
    /// #grid(
    ///   fill: (col, row) => if calc.even(col + row) { luma(240) } else { white },
    ///   align: center + horizon,
    ///   columns: 4,
    ///   [X], [O], [X], [O],
    ///   [O], [X], [O], [X],
    ///   [X], [O], [X], [O],
    ///   [O], [X], [O], [X]
    /// )
    /// ```
    #[borrowed]
    pub fill: Celled<Option<Paint>>,

    /// How to align the cells' content.
    ///
    /// This can either be a single alignment, an array of alignments
    /// (corresponding to each column) or a function that returns an alignment.
    /// The function is passed the cells' column and row index, starting at zero.
    /// If set to `{auto}`, the outer alignment is used.
    ///
    /// ```example
    /// #grid(
    ///   columns: 3,
    ///   align: (x, y) => (left, center, right).at(x),
    ///   [Hello], [Hello], [Hello],
    ///   [A], [B], [C],
    /// )
    /// ```
    #[borrowed]
    pub align: Celled<Smart<Align>>,

    /// How to [stroke]($stroke) the cells.
    ///
    /// Grids have no strokes by default, which can be changed by setting this
    /// option to the desired stroke.
    ///
    /// _Note:_ Richer stroke customization for individual cells is not yet
    /// implemented, but will be in the future. In the meantime, you can use the
    /// third-party [tablex library](https://github.com/PgBiel/typst-tablex/).
    #[resolve]
    #[fold]
    pub stroke: Option<Stroke>,

    /// How much to pad the cells' content.
    ///
    /// ```example
    /// #grid(
    ///   inset: 10pt,
    ///   fill: (_, row) => (red, blue).at(row),
    ///   [Hello],
    ///   [World],
    /// )
    ///
    /// #grid(
    ///   columns: 2,
    ///   inset: (
    ///     x: 20pt,
    ///     y: 10pt,
    ///   ),
    ///   fill: (col, _) => (red, blue).at(col),
    ///   [Hello],
    ///   [World],
    /// )
    /// ```
    #[fold]
    #[default(Sides::splat(Abs::pt(0.0).into()))]
    pub inset: Sides<Option<Rel<Length>>>,

    /// The contents of the grid cells.
    ///
    /// The cells are populated in row-major order.
    #[variadic]
    pub children: Vec<Content>,
}

impl Layout for GridElem {
    #[typst_macros::time(name = "grid", span = self.span())]
    fn layout(
        &self,
        engine: &mut Engine,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let inset = self.inset(styles);
        let align = self.align(styles);
        let columns = self.columns(styles);
        let rows = self.rows(styles);
        let column_gutter = self.column_gutter(styles);
        let row_gutter = self.row_gutter(styles);
        let fill = self.fill(styles);
        let stroke = self.stroke(styles).map(Stroke::unwrap_or_default);

        let tracks = Axes::new(columns.0.as_slice(), rows.0.as_slice());
        let gutter = Axes::new(column_gutter.0.as_slice(), row_gutter.0.as_slice());
        let cells =
            apply_align_inset_to_cells(engine, &tracks, &self.children, align, inset)?;

        // Prepare grid layout by unifying content and gutter tracks.
        let layouter = GridLayouter::new(
            tracks,
            gutter,
            &cells,
            fill,
            &stroke,
            regions,
            styles,
            self.span(),
        );

        // Measure the columns and layout the grid row-by-row.
        Ok(layouter.layout(engine)?.fragment)
    }
}

pub fn apply_align_inset_to_cells(
    engine: &mut Engine,
    tracks: &Axes<&[Sizing]>,
    cells: &[Content],
    align: &Celled<Smart<Align>>,
    inset: Sides<Rel<Length>>,
) -> SourceResult<Vec<Content>> {
    let cols = tracks.x.len().max(1);
    cells
        .iter()
        .enumerate()
        .map(|(i, child)| {
            let mut child = child.clone().padded(inset);

            let x = i % cols;
            let y = i / cols;
            if let Smart::Custom(alignment) = align.resolve(engine, x, y)? {
                child = child.styled(AlignElem::set_alignment(alignment));
            }

            Ok(child)
        })
        .collect()
}

/// Track sizing definitions.
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct TrackSizings(pub SmallVec<[Sizing; 4]>);

cast! {
    TrackSizings,
    self => self.0.into_value(),
    sizing: Sizing => Self(smallvec![sizing]),
    count: NonZeroUsize => Self(smallvec![Sizing::Auto; count.get()]),
    values: Array => Self(values.into_iter().map(Value::cast).collect::<StrResult<_>>()?),
}

/// A value that can be configured per cell.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Celled<T> {
    /// A bare value, the same for all cells.
    Value(T),
    /// A closure mapping from cell coordinates to a value.
    Func(Func),
    /// An array of alignment values corresponding to each column.
    Array(Vec<T>),
}

impl<T: Default + Clone + FromValue> Celled<T> {
    /// Resolve the value based on the cell position.
    pub fn resolve(&self, engine: &mut Engine, x: usize, y: usize) -> SourceResult<T> {
        Ok(match self {
            Self::Value(value) => value.clone(),
            Self::Func(func) => func.call(engine, [x, y])?.cast().at(func.span())?,
            Self::Array(array) => x
                .checked_rem(array.len())
                .and_then(|i| array.get(i))
                .cloned()
                .unwrap_or_default(),
        })
    }
}

impl<T: Default> Default for Celled<T> {
    fn default() -> Self {
        Self::Value(T::default())
    }
}

impl<T: Reflect> Reflect for Celled<T> {
    fn input() -> CastInfo {
        T::input() + Array::input() + Func::input()
    }

    fn output() -> CastInfo {
        T::output() + Array::output() + Func::output()
    }

    fn castable(value: &Value) -> bool {
        Array::castable(value) || Func::castable(value) || T::castable(value)
    }
}

impl<T: IntoValue> IntoValue for Celled<T> {
    fn into_value(self) -> Value {
        match self {
            Self::Value(value) => value.into_value(),
            Self::Func(func) => func.into_value(),
            Self::Array(arr) => arr.into_value(),
        }
    }
}

impl<T: FromValue> FromValue for Celled<T> {
    fn from_value(value: Value) -> StrResult<Self> {
        match value {
            Value::Func(v) => Ok(Self::Func(v)),
            Value::Array(array) => Ok(Self::Array(
                array.into_iter().map(T::from_value).collect::<StrResult<_>>()?,
            )),
            v if T::castable(&v) => Ok(Self::Value(T::from_value(v)?)),
            v => Err(Self::error(&v)),
        }
    }
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
    // How to fill the cells.
    #[allow(dead_code)]
    fill: &'a Celled<Option<Paint>>,
    // How to stroke the cells.
    #[allow(dead_code)]
    stroke: &'a Option<FixedStroke>,
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
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tracks: Axes<&[Sizing]>,
        gutter: Axes<&[Sizing]>,
        cells: &'a [Content],
        fill: &'a Celled<Option<Paint>>,
        stroke: &'a Option<FixedStroke>,
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
            fill,
            stroke,
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
    pub fn layout(mut self, engine: &mut Engine) -> SourceResult<GridLayout> {
        self.measure_columns(engine)?;

        for y in 0..self.rows.len() {
            // Skip to next region if current one is full, but only for content
            // rows, not for gutter rows.
            if self.regions.is_full() && (!self.has_gutter || y % 2 == 0) {
                self.finish_region(engine)?;
            }

            match self.rows[y] {
                Sizing::Auto => self.layout_auto_row(engine, y)?,
                Sizing::Rel(v) => self.layout_relative_row(engine, v, y)?,
                Sizing::Fr(v) => self.lrows.push(Row::Fr(v, y)),
            }
        }

        self.finish_region(engine)?;

        if self.stroke.is_some() || !matches!(self.fill, Celled::Value(None)) {
            self.render_fills_strokes(engine)?;
        }

        Ok(GridLayout {
            fragment: Fragment::frames(self.finished),
            cols: self.rcols,
            rows: self.rrows,
        })
    }

    /// Add lines and backgrounds.
    fn render_fills_strokes(&mut self, engine: &mut Engine) -> SourceResult<()> {
        for (frame, rows) in self.finished.iter_mut().zip(&self.rrows) {
            if self.rcols.is_empty() || rows.is_empty() {
                continue;
            }

            // Render table lines.
            if let Some(stroke) = self.stroke {
                let thickness = stroke.thickness;
                let half = thickness / 2.0;

                // Render horizontal lines.
                for offset in points(rows.iter().map(|piece| piece.height)) {
                    let target = Point::with_x(frame.width() + thickness);
                    let hline = Geometry::Line(target).stroked(stroke.clone());
                    frame.prepend(
                        Point::new(-half, offset),
                        FrameItem::Shape(hline, self.span),
                    );
                }

                // Render vertical lines.
                for offset in points(self.rcols.iter().copied()) {
                    let target = Point::with_y(frame.height() + thickness);
                    let vline = Geometry::Line(target).stroked(stroke.clone());
                    frame.prepend(
                        Point::new(offset, -half),
                        FrameItem::Shape(vline, self.span),
                    );
                }
            }

            // Render cell backgrounds.
            let mut dx = Abs::zero();
            for (x, &col) in self.rcols.iter().enumerate() {
                let mut dy = Abs::zero();
                for row in rows {
                    if let Some(fill) = self.fill.resolve(engine, x, row.y)? {
                        let pos = Point::new(dx, dy);
                        let size = Size::new(col, row.height);
                        let rect = Geometry::Rect(size).filled(fill);
                        frame.prepend(pos, FrameItem::Shape(rect, self.span));
                    }
                    dy += row.height;
                }
                dx += col;
            }
        }

        Ok(())
    }

    /// Determine all column sizes.
    fn measure_columns(&mut self, engine: &mut Engine) -> SourceResult<()> {
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
            let (auto, count) = self.measure_auto_columns(engine, available)?;

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
        engine: &mut Engine,
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
                    let frame = cell.measure(engine, self.styles, pod)?.into_frame();
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
    fn layout_auto_row(&mut self, engine: &mut Engine, y: usize) -> SourceResult<()> {
        // Determine the size for each region of the row. If the first region
        // ends up empty for some column, skip the region and remeasure.
        let mut resolved = match self.measure_auto_row(engine, y, true)? {
            Some(resolved) => resolved,
            None => {
                self.finish_region(engine)?;
                self.measure_auto_row(engine, y, false)?.unwrap()
            }
        };

        // Nothing to layout.
        if resolved.is_empty() {
            return Ok(());
        }

        // Layout into a single region.
        if let &[first] = resolved.as_slice() {
            let frame = self.layout_single_row(engine, first, y)?;
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
        let fragment = self.layout_multi_row(engine, &resolved, y)?;
        let len = fragment.len();
        for (i, frame) in fragment.into_iter().enumerate() {
            self.push_row(frame, y);
            if i + 1 < len {
                self.finish_region(engine)?;
            }
        }

        Ok(())
    }

    /// Measure the regions sizes of an auto row. The option is always `Some(_)`
    /// if `can_skip` is false.
    fn measure_auto_row(
        &mut self,
        engine: &mut Engine,
        y: usize,
        can_skip: bool,
    ) -> SourceResult<Option<Vec<Abs>>> {
        let mut resolved: Vec<Abs> = vec![];

        for (x, &rcol) in self.rcols.iter().enumerate() {
            if let Some(cell) = self.cell(x, y) {
                let mut pod = self.regions;
                pod.size.x = rcol;

                let frames = cell.measure(engine, self.styles, pod)?.into_frames();

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
        engine: &mut Engine,
        v: Rel<Length>,
        y: usize,
    ) -> SourceResult<()> {
        let resolved = v.resolve(self.styles).relative_to(self.regions.base().y);
        let frame = self.layout_single_row(engine, resolved, y)?;

        // Skip to fitting region.
        let height = frame.height();
        while !self.regions.size.y.fits(height) && !self.regions.in_last() {
            self.finish_region(engine)?;

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
        engine: &mut Engine,
        height: Abs,
        y: usize,
    ) -> SourceResult<Frame> {
        if !height.is_finite() {
            bail!(self.span, "cannot create grid with infinite height");
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
                let frame = cell.layout(engine, self.styles, pod)?.into_frame();
                output.push_frame(pos, frame);
            }

            pos.x += rcol;
        }

        Ok(output)
    }

    /// Layout a row spanning multiple regions.
    fn layout_multi_row(
        &mut self,
        engine: &mut Engine,
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
                let fragment = cell.layout(engine, self.styles, pod)?;
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
    fn finish_region(&mut self, engine: &mut Engine) -> SourceResult<()> {
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
                    (self.layout_single_row(engine, height, y)?, y)
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

/// Turn an iterator of extents into an iterator of offsets before, in between,
/// and after the extents, e.g. [10mm, 5mm] -> [0mm, 10mm, 15mm].
fn points(extents: impl IntoIterator<Item = Abs>) -> impl Iterator<Item = Abs> {
    let mut offset = Abs::zero();
    std::iter::once(Abs::zero()).chain(extents).map(move |extent| {
        offset += extent;
        offset
    })
}
