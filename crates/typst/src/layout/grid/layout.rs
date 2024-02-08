use std::fmt::Debug;
use std::hash::Hash;
use std::num::NonZeroUsize;
use std::sync::Arc;

use ecow::eco_format;

use crate::diag::{
    bail, At, Hint, HintedStrResult, HintedString, SourceResult, StrResult,
};
use crate::engine::Engine;
use crate::foundations::{
    Array, CastInfo, Content, Fold, FromValue, Func, IntoValue, Reflect, Resolve, Smart,
    StyleChain, Value,
};
use crate::layout::{
    Abs, Alignment, Axes, Dir, Fr, Fragment, Frame, FrameItem, LayoutMultiple, Length,
    Point, Regions, Rel, Sides, Size, Sizing,
};
use crate::syntax::Span;
use crate::text::TextElem;
use crate::util::{MaybeReverseIter, NonZeroExt, Numeric};
use crate::visualize::{Geometry, Paint, Stroke};

/// Resolved settings for the strokes of cells' lines.
pub enum ResolvedInsideStroke {
    /// Configures all automatic lines spanning the whole grid.
    Auto(Option<Stroke<Abs>>),
    /// Configures the borders of each cell.
    Celled(ResolvedCelled<Sides<Option<Option<Arc<Stroke>>>>>),
}

impl Default for ResolvedInsideStroke {
    fn default() -> Self {
        Self::Auto(None)
    }
}

/// Resolved grid-wide stroke settings.
#[derive(Default)]
pub struct ResolvedGridStroke {
    /// Configures only the grid's border lines.
    #[allow(clippy::type_complexity)] // TODO: Create a type alias or something
    pub outside: Sides<Option<Option<Arc<Stroke<Abs>>>>>,
    /// Configures the cells' lines.
    pub inside: ResolvedInsideStroke,
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

impl<T: Fold> Fold for Celled<T> {
    fn fold(self, outer: Self) -> Self {
        match (self, outer) {
            (Self::Value(inner), Self::Value(outer)) => Self::Value(inner.fold(outer)),
            (self_, _) => self_,
        }
    }
}

impl<T: Resolve> Resolve for Celled<T> {
    type Output = ResolvedCelled<T>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        match self {
            Self::Value(value) => ResolvedCelled::Value(value.resolve(styles)),
            Self::Func(func) => ResolvedCelled::Func(func),
            Self::Array(values) => ResolvedCelled::Array(
                values.into_iter().map(|value| value.resolve(styles)).collect(),
            ),
        }
    }
}

/// The result of resolving a Celled's value according to styles.
/// Holds resolved values which depend on each grid cell's position.
pub enum ResolvedCelled<T: Resolve> {
    /// The resolved value. The same for all cells.
    Value(<T as Resolve>::Output),
    /// A closure mapping cell coordinates to a value.
    /// The value is only resolved upon usage.
    Func(Func),
    /// An array of resolved values corresponding to each column.
    Array(Vec<<T as Resolve>::Output>),
}

impl<T> ResolvedCelled<T>
where
    T: FromValue + Resolve,
    <T as Resolve>::Output: Default + Clone,
{
    /// Resolve the value based on the cell position.
    pub fn resolve(
        &self,
        engine: &mut Engine,
        styles: StyleChain,
        x: usize,
        y: usize,
    ) -> SourceResult<T::Output> {
        Ok(match self {
            Self::Value(value) => value.clone(),
            Self::Func(func) => func
                .call(engine, [x, y])?
                .cast::<T>()
                .at(func.span())?
                .resolve(styles),
            Self::Array(array) => x
                .checked_rem(array.len())
                .and_then(|i| array.get(i))
                .cloned()
                .unwrap_or_default(),
        })
    }
}

impl<T> Default for ResolvedCelled<T>
where
    T: Resolve,
    <T as Resolve>::Output: Default,
{
    fn default() -> Self {
        Self::Value(<T as Resolve>::Output::default())
    }
}

impl<T> Clone for ResolvedCelled<T>
where
    T: Resolve,
    <T as Resolve>::Output: Clone,
{
    fn clone(&self) -> Self {
        match self {
            Self::Value(value) => Self::Value(value.clone()),
            Self::Func(func) => Self::Func(func.clone()),
            Self::Array(values) => Self::Array(values.clone()),
        }
    }
}

/// Represents an explicit grid line (horizontal or vertical) specified by the
/// user.
#[allow(dead_code)]
pub struct Line {
    /// The index of the track after this line. This will be the index of the
    /// row a horizontal line is above of, or of the column right after a
    /// vertical line.
    /// Must be within `0..=tracks.len()` (where `tracks` is either `grid.cols`
    /// or `grid.rows`, as appropriate).
    index: usize,
    /// The index of the track at which this line starts being drawn.
    /// This is the first column a horizontal line appears in, or the first row
    /// a vertical line appears in.
    /// Must be within `0..tracks.len()`.
    start: usize,
    /// The index after the last track through which the line is drawn.
    /// Thus, the line is drawn through tracks `start..end` (note that `end` is
    /// exclusive).
    /// Must be within `1..=tracks.len()`.
    /// None indicates the line should go all the way to the end.
    end: Option<NonZeroUsize>,
    /// The line's stroke. This is `None` when the line is explicitly used to
    /// simply remove an automatic line.
    stroke: Option<Arc<Stroke<Abs>>>,
}

/// Represents a cell in CellGrid, to be laid out by GridLayouter.
#[derive(Clone)]
pub struct Cell {
    /// The cell's body.
    pub body: Content,
    /// The cell's fill.
    pub fill: Option<Paint>,
    /// The amount of columns spanned by the cell.
    pub colspan: NonZeroUsize,
    /// The cell's stroke.
    /// We use an Arc to avoid unnecessary space usage when all sides are the
    /// same, or when the strokes come from a common source.
    pub stroke: Sides<Option<Arc<Stroke<Abs>>>>,
}

impl From<Content> for Cell {
    /// Create a simple cell given its body.
    fn from(body: Content) -> Self {
        Self {
            body,
            fill: None,
            colspan: NonZeroUsize::ONE,
            stroke: Sides::splat(None),
        }
    }
}

impl LayoutMultiple for Cell {
    fn layout(
        &self,
        engine: &mut Engine,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        self.body.layout(engine, styles, regions)
    }
}

/// A grid entry.
#[derive(Clone)]
enum Entry {
    /// An entry which holds a cell.
    Cell(Cell),
    /// An entry which is merged with another cell.
    Merged {
        /// The index of the cell this entry is merged with.
        parent: usize,
    },
}

impl Entry {
    /// Obtains the cell inside this entry, if this is not a merged cell.
    fn as_cell(&self) -> Option<&Cell> {
        match self {
            Self::Cell(cell) => Some(cell),
            Self::Merged { .. } => None,
        }
    }
}

/// A grid item, possibly affected by automatic cell positioning. Can be either
/// a line or a cell.
pub enum GridItem<T: ResolvableCell> {
    /// A horizontal line in the grid.
    HLine {
        /// The row above which the horizontal line is drawn.
        y: Smart<usize>,
        start: usize,
        end: Option<NonZeroUsize>,
        stroke: Option<Arc<Stroke<Abs>>>,
        /// The span of the corresponding line element.
        span: Span,
    },
    /// A vertical line in the grid.
    VLine {
        /// The column before which the vertical line is drawn.
        x: Smart<usize>,
        start: usize,
        end: Option<NonZeroUsize>,
        stroke: Option<Arc<Stroke<Abs>>>,
        /// The span of the corresponding line element.
        span: Span,
    },
    /// A cell in the grid.
    Cell(T),
}

/// Used for cell-like elements which are aware of their final properties in
/// the table, and may have property overrides.
pub trait ResolvableCell {
    /// Resolves the cell's fields, given its coordinates and default grid-wide
    /// fill, align, inset and stroke properties.
    /// Returns a final Cell.
    #[allow(clippy::too_many_arguments)]
    fn resolve_cell(
        self,
        x: usize,
        y: usize,
        fill: &Option<Paint>,
        align: Smart<Alignment>,
        inset: Sides<Option<Rel<Length>>>,
        stroke: Sides<Option<Option<Arc<Stroke<Abs>>>>>,
        styles: StyleChain,
    ) -> Cell;

    /// Returns this cell's column override.
    fn x(&self, styles: StyleChain) -> Smart<usize>;

    /// Returns this cell's row override.
    fn y(&self, styles: StyleChain) -> Smart<usize>;

    /// The amount of columns spanned by this cell.
    fn colspan(&self, styles: StyleChain) -> NonZeroUsize;

    /// The cell's span, for errors.
    fn span(&self) -> Span;
}

/// A grid of cells, including the columns, rows, and cell data.
pub struct CellGrid {
    /// The grid cells.
    entries: Vec<Entry>,
    /// The column tracks including gutter tracks.
    cols: Vec<Sizing>,
    /// The row tracks including gutter tracks.
    rows: Vec<Sizing>,
    /// The global grid stroke options.
    stroke: ResolvedGridStroke,
    /// The vertical lines before each column, or on the right border.
    /// Contains up to 'cols.len() + 1' vectors of lines.
    #[allow(dead_code)]
    vlines: Vec<Vec<Line>>,
    /// The horizontal lines on top of each row, or on the bottom border.
    /// Contains up to 'rows.len() + 1' vectors of lines.
    #[allow(dead_code)]
    hlines: Vec<Vec<Line>>,
    /// Whether this grid has gutters.
    has_gutter: bool,
}

impl CellGrid {
    /// Generates the cell grid, given the tracks and cells.
    pub fn new(
        tracks: Axes<&[Sizing]>,
        gutter: Axes<&[Sizing]>,
        cells: impl IntoIterator<Item = Cell>,
    ) -> Self {
        let entries = cells.into_iter().map(Entry::Cell).collect();
        Self::new_internal(
            tracks,
            gutter,
            ResolvedGridStroke::default(),
            vec![],
            vec![],
            entries,
        )
    }

    /// Resolves and positions all cells in the grid before creating it.
    /// Allows them to keep track of their final properties and positions
    /// and adjust their fields accordingly.
    /// Cells must implement Clone as they will be owned. Additionally, they
    /// must implement Default in order to fill positions in the grid which
    /// weren't explicitly specified by the user with empty cells.
    #[allow(clippy::too_many_arguments)]
    pub fn resolve<T, I>(
        tracks: Axes<&[Sizing]>,
        gutter: Axes<&[Sizing]>,
        items: I,
        fill: &Celled<Option<Paint>>,
        align: &Celled<Smart<Alignment>>,
        inset: Sides<Option<Rel<Length>>>,
        stroke: ResolvedGridStroke,
        engine: &mut Engine,
        styles: StyleChain,
        span: Span,
    ) -> SourceResult<Self>
    where
        T: ResolvableCell + Default,
        I: IntoIterator<Item = GridItem<T>>,
        I::IntoIter: ExactSizeIterator,
    {
        // Number of content columns: Always at least one.
        let c = tracks.x.len().max(1);

        // Lists of lines.
        let mut vlines: Vec<Vec<Line>> = Vec::with_capacity(c);
        let mut hlines: Vec<Vec<Line>> = Vec::new();
        // Horizontal lines are only pushed later to be able to check for row
        // validity.
        // We keep their spans so we can report errors later.
        let mut pending_hlines: Vec<(Span, Line)> = Vec::new();

        // We can't just use the cell's index in the 'cells' vector to
        // determine its automatic position, since cells could have arbitrary
        // positions, so the position of a cell in 'cells' can differ from its
        // final position in 'resolved_cells' (see below).
        // Therefore, we use a counter, 'auto_index', to determine the position
        // of the next cell with (x: auto, y: auto). It is only stepped when
        // a cell with (x: auto, y: auto), usually the vast majority, is found.
        let mut auto_index: usize = 0;

        // We have to rebuild the grid to account for arbitrary positions.
        // Create at least 'items.len()' positions, since there could be at
        // least 'items.len()' cells, even though some of them might be placed
        // in arbitrary positions and thus cause the grid to expand.
        // Additionally, make sure we allocate up to the next multiple of 'c',
        // since each row will have 'c' cells, even if the last few cells
        // weren't explicitly specified by the user.
        // We apply '% c' twice so that the amount of cells potentially missing
        // is zero when 'items.len()' is already a multiple of 'c' (thus
        // 'items.len() % c' would be zero).
        let items = items.into_iter();
        let Some(item_count) = items.len().checked_add((c - items.len() % c) % c) else {
            bail!(span, "too many cells or lines were given")
        };
        let mut resolved_cells: Vec<Option<Entry>> = Vec::with_capacity(item_count);
        for item in items {
            let cell = match item {
                GridItem::HLine { y, start, end, stroke, span } => {
                    let y = y.as_custom().unwrap_or_else(|| {
                        // When no 'y' is specified for the hline, we place it
                        // under the latest automatically positioned cell.
                        // The current value of the auto index is always the
                        // index of the latest automatically positioned cell
                        // placed plus one (that's what we do in
                        // 'resolve_cell_position'), so we subtract 1 to get
                        // that cell's index, and place the hline below its
                        // row. The exception is when the auto_index is 0,
                        // meaning no automatically positioned cell was placed
                        // yet. In that case, we place the hline at the top of
                        // the table.
                        auto_index
                            .checked_sub(1)
                            .map_or(0, |last_auto_index| last_auto_index / c + 1)
                    });
                    // Since the amount of rows is dynamic, delay placing
                    // hlines until after all cells were placed so we can
                    // properly verify if they are valid. Note that we can't
                    // place hlines even if we already know they would be in a
                    // valid row, since it's possible that we pushed pending
                    // hlines in the same row as this one in previous
                    // iterations, and we need to ensure that hlines from
                    // previous iterations are pushed to the final vector of
                    // hlines first - the order of hlines must be kept, as this
                    // matters when determining which one "wins" in case of
                    // conflict. Pushing the current hline before we push
                    // pending hlines later would change their order!
                    pending_hlines.push((span, Line { index: y, start, end, stroke }));
                    continue;
                }
                GridItem::VLine { x, start, end, stroke, span } => {
                    let x = x.as_custom().unwrap_or_else(|| {
                        // When no 'x' is specified for the vline, we place it
                        // after the latest automatically positioned cell.
                        // The current value of the auto index is always the
                        // index of the latest automatically positioned cell
                        // placed plus one (that's what we do in
                        // 'resolve_cell_position'), so we subtract 1 to get
                        // that cell's index, and place the vline after its
                        // column. The exception is when the auto_index is 0,
                        // meaning no automatically positioned cell was placed
                        // yet. In that case, we place the vline to the left of
                        // the table.
                        auto_index
                            .checked_sub(1)
                            .map_or(0, |last_auto_index| last_auto_index % c + 1)
                    });
                    if x > c {
                        bail!(span, "cannot place vertical line at invalid column");
                    }
                    if vlines.len() <= x {
                        vlines.resize_with(x + 1, Vec::new);
                    }
                    vlines[x].push(Line { index: x, start, end, stroke });
                    continue;
                }
                GridItem::Cell(cell) => cell,
            };
            let cell_span = cell.span();
            // Let's calculate the cell's final position based on its
            // requested position.
            let resolved_index = {
                let cell_x = cell.x(styles);
                let cell_y = cell.y(styles);
                resolve_cell_position(cell_x, cell_y, &resolved_cells, &mut auto_index, c)
                    .at(cell_span)?
            };
            let x = resolved_index % c;
            let y = resolved_index / c;
            let colspan = cell.colspan(styles).get();

            if colspan > c - x {
                bail!(
                    cell_span,
                    "cell's colspan would cause it to exceed the available column(s)";
                    hint: "try placing the cell in another position or reducing its colspan"
                )
            }

            let Some(largest_index) = resolved_index.checked_add(colspan - 1) else {
                bail!(
                    cell_span,
                    "cell would span an exceedingly large position";
                    hint: "try reducing the cell's colspan"
                )
            };

            let stroke = match &stroke.inside {
                ResolvedInsideStroke::Auto(_) => Sides::default(),
                ResolvedInsideStroke::Celled(stroke) => {
                    stroke.clone().resolve(engine, styles, x, y)?
                }
            };

            // Let's resolve the cell so it can determine its own fields
            // based on its final position.
            let cell = cell.resolve_cell(
                x,
                y,
                &fill.resolve(engine, x, y)?,
                align.resolve(engine, x, y)?,
                inset,
                stroke,
                styles,
            );

            if largest_index >= resolved_cells.len() {
                // Ensure the length of the vector of resolved cells is always
                // a multiple of 'c' by pushing full rows every time. Here, we
                // add enough absent positions (later converted to empty cells)
                // to ensure the last row in the new vector length is
                // completely filled. This is necessary so that those
                // positions, even if not explicitly used at the end, are
                // eventually susceptible to show rules and receive grid
                // styling, as they will be resolved as empty cells in a second
                // loop below.
                let Some(new_len) = largest_index
                    .checked_add(1)
                    .and_then(|new_len| new_len.checked_add((c - new_len % c) % c))
                else {
                    bail!(cell_span, "cell position too large")
                };

                // Here, the cell needs to be placed in a position which
                // doesn't exist yet in the grid (out of bounds). We will add
                // enough absent positions for this to be possible. They must
                // be absent as no cells actually occupy them (they can be
                // overridden later); however, if no cells occupy them as we
                // finish building the grid, then such positions will be
                // replaced by empty cells.
                resolved_cells.resize(new_len, None);
            }

            // The vector is large enough to contain the cell, so we can just
            // index it directly to access the position it will be placed in.
            // However, we still need to ensure we won't try to place a cell
            // where there already is one.
            let slot = &mut resolved_cells[resolved_index];
            if slot.is_some() {
                bail!(
                    cell_span,
                    "attempted to place a second cell at column {x}, row {y}";
                    hint: "try specifying your cells in a different order"
                );
            }

            *slot = Some(Entry::Cell(cell));

            // Now, if the cell spans more than one column, we fill the spanned
            // positions in the grid with Entry::Merged pointing to the
            // original cell as its parent.
            for (offset, slot) in resolved_cells[resolved_index..][..colspan]
                .iter_mut()
                .enumerate()
                .skip(1)
            {
                if slot.is_some() {
                    let spanned_x = x + offset;
                    bail!(
                        cell_span,
                        "cell would span a previously placed cell at column {spanned_x}, row {y}";
                        hint: "try specifying your cells in a different order or reducing the cell's colspan"
                    )
                }
                *slot = Some(Entry::Merged { parent: resolved_index });
            }
        }

        // Replace absent entries by resolved empty cells, and produce a vector
        // of 'Entry' from 'Option<Entry>' (final step).
        let resolved_cells = resolved_cells
            .into_iter()
            .enumerate()
            .map(|(i, cell)| {
                if let Some(cell) = cell {
                    Ok(cell)
                } else {
                    let x = i % c;
                    let y = i / c;
                    let stroke = match &stroke.inside {
                        ResolvedInsideStroke::Auto(_) => Sides::default(),
                        ResolvedInsideStroke::Celled(stroke) => {
                            stroke.clone().resolve(engine, styles, x, y)?
                        }
                    };

                    // Ensure all absent entries are affected by show rules and
                    // grid styling by turning them into resolved empty cells.
                    let new_cell = T::default().resolve_cell(
                        x,
                        y,
                        &fill.resolve(engine, x, y)?,
                        align.resolve(engine, x, y)?,
                        inset,
                        stroke,
                        styles,
                    );
                    Ok(Entry::Cell(new_cell))
                }
            })
            .collect::<SourceResult<Vec<Entry>>>()?;

        for (span, line) in pending_hlines {
            let y = line.index;
            if resolved_cells.len().div_ceil(c) < y {
                bail!(span, "cannot place horizontal line at invalid row");
            }
            if hlines.len() <= y {
                hlines.resize_with(y + 1, Vec::new);
            }
            hlines[y].push(line);
        }

        Ok(Self::new_internal(tracks, gutter, stroke, vlines, hlines, resolved_cells))
    }

    /// Generates the cell grid, given the tracks and resolved entries.
    fn new_internal(
        tracks: Axes<&[Sizing]>,
        gutter: Axes<&[Sizing]>,
        stroke: ResolvedGridStroke,
        vlines: Vec<Vec<Line>>,
        hlines: Vec<Vec<Line>>,
        entries: Vec<Entry>,
    ) -> Self {
        let mut cols = vec![];
        let mut rows = vec![];

        // Number of content columns: Always at least one.
        let c = tracks.x.len().max(1);

        // Number of content rows: At least as many as given, but also at least
        // as many as needed to place each item.
        let r = {
            let len = entries.len();
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

        Self {
            cols,
            rows,
            entries,
            stroke,
            vlines,
            hlines,
            has_gutter,
        }
    }

    /// Get the grid entry in column `x` and row `y`.
    ///
    /// Returns `None` if it's a gutter cell.
    #[track_caller]
    fn entry(&self, x: usize, y: usize) -> Option<&Entry> {
        assert!(x < self.cols.len());
        assert!(y < self.rows.len());

        if self.has_gutter {
            // Even columns and rows are children, odd ones are gutter.
            if x % 2 == 0 && y % 2 == 0 {
                let c = 1 + self.cols.len() / 2;
                self.entries.get((y / 2) * c + x / 2)
            } else {
                None
            }
        } else {
            let c = self.cols.len();
            self.entries.get(y * c + x)
        }
    }

    /// Get the content of the cell in column `x` and row `y`.
    ///
    /// Returns `None` if it's a gutter cell or merged position.
    #[track_caller]
    fn cell(&self, x: usize, y: usize) -> Option<&Cell> {
        self.entry(x, y).and_then(Entry::as_cell)
    }

    /// Returns the parent cell of the grid entry at the given position.
    /// If the entry at the given position is a cell, returns it.
    /// If it is a merged cell, returns the parent cell.
    /// If it is a gutter cell, returns None.
    #[track_caller]
    fn parent_cell(&self, x: usize, y: usize) -> Option<&Cell> {
        self.parent_cell_position(x, y)
            .and_then(|Axes { x, y }| self.cell(x, y))
    }

    /// Returns the position of the parent cell of the grid entry at the given
    /// position. It is guaranteed to have a non-gutter, non-merged cell at
    /// the returned position, due to how the grid is built.
    /// If the entry at the given position is a cell, returns the given
    /// position.
    /// If it is a merged cell, returns the parent cell's position.
    /// If it is a gutter cell, returns None.
    #[track_caller]
    fn parent_cell_position(&self, x: usize, y: usize) -> Option<Axes<usize>> {
        self.entry(x, y).map(|entry| match entry {
            Entry::Cell(_) => Axes::new(x, y),
            Entry::Merged { parent } => {
                let c = if self.has_gutter {
                    1 + self.cols.len() / 2
                } else {
                    self.cols.len()
                };
                let factor = if self.has_gutter { 2 } else { 1 };
                Axes::new(factor * (*parent % c), factor * (*parent / c))
            }
        })
    }
}

/// Given a cell's requested x and y, the vector with the resolved cell
/// positions, the `auto_index` counter (determines the position of the next
/// `(auto, auto)` cell) and the amount of columns in the grid, returns the
/// final index of this cell in the vector of resolved cells.
fn resolve_cell_position(
    cell_x: Smart<usize>,
    cell_y: Smart<usize>,
    resolved_cells: &[Option<Entry>],
    auto_index: &mut usize,
    columns: usize,
) -> HintedStrResult<usize> {
    // Translates a (x, y) position to the equivalent index in the final cell vector.
    // Errors if the position would be too large.
    let cell_index = |x, y: usize| {
        y.checked_mul(columns)
            .and_then(|row_index| row_index.checked_add(x))
            .ok_or_else(|| HintedString::from(eco_format!("cell position too large")))
    };
    match (cell_x, cell_y) {
        // Fully automatic cell positioning. The cell did not
        // request a coordinate.
        (Smart::Auto, Smart::Auto) => {
            // Let's find the first available position starting from the
            // automatic position counter, searching in row-major order.
            let mut resolved_index = *auto_index;
            while let Some(Some(_)) = resolved_cells.get(resolved_index) {
                // Skip any non-absent cell positions (`Some(None)`) to
                // determine where this cell will be placed. An out of bounds
                // position (thus `None`) is also a valid new position (only
                // requires expanding the vector).
                resolved_index += 1;
            }

            // Ensure the next cell with automatic position will be
            // placed after this one (maybe not immediately after).
            *auto_index = resolved_index + 1;

            Ok(resolved_index)
        }
        // Cell has chosen at least its column.
        (Smart::Custom(cell_x), cell_y) => {
            if cell_x >= columns {
                return Err(HintedString::from(eco_format!(
                    "cell could not be placed at invalid column {cell_x}"
                )));
            }
            if let Smart::Custom(cell_y) = cell_y {
                // Cell has chosen its exact position.
                cell_index(cell_x, cell_y)
            } else {
                // Cell has only chosen its column.
                // Let's find the first row which has that column available.
                let mut resolved_y = 0;
                while let Some(Some(_)) =
                    resolved_cells.get(cell_index(cell_x, resolved_y)?)
                {
                    // Try each row until either we reach an absent position
                    // (`Some(None)`) or an out of bounds position (`None`),
                    // in which case we'd create a new row to place this cell in.
                    resolved_y += 1;
                }
                cell_index(cell_x, resolved_y)
            }
        }
        // Cell has only chosen its row, not its column.
        (Smart::Auto, Smart::Custom(cell_y)) => {
            // Let's find the first column which has that row available.
            let first_row_pos = cell_index(0, cell_y)?;
            let last_row_pos = first_row_pos
                .checked_add(columns)
                .ok_or_else(|| eco_format!("cell position too large"))?;

            (first_row_pos..last_row_pos)
                .find(|possible_index| {
                    // Much like in the previous cases, we skip any occupied
                    // positions until we either reach an absent position
                    // (`Some(None)`) or an out of bounds position (`None`),
                    // in which case we can just expand the vector enough to
                    // place this cell. In either case, we found an available
                    // position.
                    !matches!(resolved_cells.get(*possible_index), Some(Some(_)))
                })
                .ok_or_else(|| {
                    eco_format!(
                        "cell could not be placed in row {cell_y} because it was full"
                    )
                })
                .hint("try specifying your cells in a different order")
        }
    }
}

/// Performs grid layout.
pub struct GridLayouter<'a> {
    /// The grid of cells.
    grid: &'a CellGrid,
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
    /// Whether this is an RTL grid.
    is_rtl: bool,
    /// The span of the grid element.
    span: Span,
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
        grid: &'a CellGrid,
        regions: Regions<'a>,
        styles: StyleChain<'a>,
        span: Span,
    ) -> Self {
        // We use these regions for auto row measurement. Since at that moment,
        // columns are already sized, we can enable horizontal expansion.
        let mut regions = regions;
        regions.expand = Axes::new(true, false);

        Self {
            grid,
            regions,
            styles,
            rcols: vec![Abs::zero(); grid.cols.len()],
            width: Abs::zero(),
            rrows: vec![],
            lrows: vec![],
            initial: regions.size,
            finished: vec![],
            is_rtl: TextElem::dir_in(styles) == Dir::RTL,
            span,
        }
    }

    /// Determines the columns sizes and then layouts the grid row-by-row.
    pub fn layout(mut self, engine: &mut Engine) -> SourceResult<Fragment> {
        self.measure_columns(engine)?;

        for y in 0..self.grid.rows.len() {
            // Skip to next region if current one is full, but only for content
            // rows, not for gutter rows.
            if self.regions.is_full() && (!self.grid.has_gutter || y % 2 == 0) {
                self.finish_region(engine)?;
            }

            match self.grid.rows[y] {
                Sizing::Auto => self.layout_auto_row(engine, y)?,
                Sizing::Rel(v) => self.layout_relative_row(engine, v, y)?,
                Sizing::Fr(v) => self.lrows.push(Row::Fr(v, y)),
            }
        }

        self.finish_region(engine)?;

        self.render_fills_strokes()
    }

    /// Add lines and backgrounds.
    fn render_fills_strokes(mut self) -> SourceResult<Fragment> {
        let mut finished = std::mem::take(&mut self.finished);
        let stroke = match &self.grid.stroke.inside {
            ResolvedInsideStroke::Auto(Some(stroke)) => Some(Arc::new(stroke.clone())),
            _ => None,
        };

        for (frame, rows) in finished.iter_mut().zip(&self.rrows) {
            if self.rcols.is_empty() || rows.is_empty() {
                continue;
            }

            // Render table lines.

            // Render horizontal lines.
            // First, calculate their offsets from the top of the frame.
            let hline_offsets = points(rows.iter().map(|piece| piece.height));
            // Additionally, determine their indices (the indices of the
            // rows they are drawn on top of). In principle, this will
            // correspond to the rows' indices directly, except for the
            // first and last hlines, which must be 0 and (amount of rows)
            // respectively, as they are always drawn (due to being part of
            // the table's border).
            let hline_indices = std::iter::once(0)
                .chain(rows.iter().map(|piece| piece.y).skip(1))
                .chain(std::iter::once(self.grid.rows.len()));
            for (y, dy) in hline_indices.zip(hline_offsets) {
                let hlines_at_row =
                    self.grid.hlines.get(y).map(|hlines| &**hlines).unwrap_or(&[]);
                let tracks = self.rcols.iter().copied().enumerate();

                // Apply top / bottom border stroke overrides.
                let stroke = match y {
                    0 => self
                        .grid
                        .stroke
                        .outside
                        .top
                        .as_ref()
                        .map(|border_stroke| border_stroke.clone().fold(stroke.clone()))
                        .unwrap_or_else(|| stroke.clone()),
                    y if y == self.grid.rows.len() => self
                        .grid
                        .stroke
                        .outside
                        .bottom
                        .as_ref()
                        .map(|border_stroke| border_stroke.clone().fold(stroke.clone()))
                        .unwrap_or_else(|| stroke.clone()),
                    _ => stroke.clone(),
                };

                // Determine all different line segments we have to draw in
                // this row.
                for (stroke, dx, length) in generate_line_segments(
                    self.grid,
                    tracks,
                    y,
                    stroke.as_ref(),
                    hlines_at_row,
                    hline_stroke_at_column,
                ) {
                    let stroke = (*stroke).clone().unwrap_or_default();
                    let thickness = stroke.thickness;
                    let half = thickness / 2.0;
                    let dx = if self.is_rtl { self.width - dx - length } else { dx };
                    let target = Point::with_x(length + thickness);
                    let hline = Geometry::Line(target).stroked(stroke);
                    frame.prepend(
                        Point::new(dx - half, dy),
                        FrameItem::Shape(hline, self.span),
                    );
                }
            }

            // Render vertical lines.
            for (x, dx) in points(self.rcols.iter().copied()).enumerate() {
                let dx = if self.is_rtl { self.width - dx } else { dx };
                let vlines_at_column =
                    self.grid.vlines.get(x).map(|vlines| &**vlines).unwrap_or(&[]);
                let tracks = rows.iter().map(|row| (row.y, row.height));

                // Apply left / right border stroke overrides.
                let stroke = match x {
                    0 => self
                        .grid
                        .stroke
                        .outside
                        .left
                        .as_ref()
                        .map(|border_stroke| border_stroke.clone().fold(stroke.clone()))
                        .unwrap_or_else(|| stroke.clone()),
                    x if x == self.grid.cols.len() => self
                        .grid
                        .stroke
                        .outside
                        .right
                        .as_ref()
                        .map(|border_stroke| border_stroke.clone().fold(stroke.clone()))
                        .unwrap_or_else(|| stroke.clone()),
                    _ => stroke.clone(),
                };

                // Determine all different line segments we have to draw in
                // this column.
                // Even a single line might generate more than one segment,
                // if it happens to cross a colspan (over which it must not be
                // drawn).
                for (stroke, dy, length) in generate_line_segments(
                    self.grid,
                    tracks,
                    x,
                    stroke.as_ref(),
                    vlines_at_column,
                    vline_stroke_at_row,
                ) {
                    let stroke = (*stroke).clone().unwrap_or_default();
                    let thickness = stroke.thickness;
                    let half = thickness / 2.0;
                    let target = Point::with_y(length + thickness);
                    let vline = Geometry::Line(target).stroked(stroke);
                    frame.prepend(
                        Point::new(dx, dy - half),
                        FrameItem::Shape(vline, self.span),
                    );
                }
            }

            // Render cell backgrounds.
            // Reverse with RTL so that later columns start first.
            let mut dx = Abs::zero();
            for (x, &col) in self.rcols.iter().enumerate().rev_if(self.is_rtl) {
                let mut dy = Abs::zero();
                for row in rows {
                    if let Some(cell) = self.grid.cell(x, row.y) {
                        let fill = cell.fill.clone();
                        if let Some(fill) = fill {
                            let width = self.cell_spanned_width(x, cell.colspan.get());
                            // In the grid, cell colspans expand to the right,
                            // so we're at the leftmost (lowest 'x') column
                            // spanned by the cell. However, in RTL, cells
                            // expand to the left. Therefore, without the
                            // offset below, cell fills would start at the
                            // rightmost visual position of a cell and extend
                            // over to unrelated columns to the right in RTL.
                            // We avoid this by ensuring the fill starts at the
                            // very left of the cell, even with colspan > 1.
                            let offset =
                                if self.is_rtl { -width + col } else { Abs::zero() };
                            let pos = Point::new(dx + offset, dy);
                            let size = Size::new(width, row.height);
                            let rect = Geometry::Rect(size).filled(fill);
                            frame.prepend(pos, FrameItem::Shape(rect, self.span));
                        }
                    }
                    dy += row.height;
                }
                dx += col;
            }
        }

        Ok(Fragment::frames(finished))
    }

    /// Determine all column sizes.
    fn measure_columns(&mut self, engine: &mut Engine) -> SourceResult<()> {
        // Sum of sizes of resolved relative tracks.
        let mut rel = Abs::zero();

        // Sum of fractions of all fractional tracks.
        let mut fr = Fr::zero();

        // Resolve the size of all relative columns and compute the sum of all
        // fractional tracks.
        for (&col, rcol) in self.grid.cols.iter().zip(&mut self.rcols) {
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

    /// Total width spanned by the cell (among resolved columns).
    /// Includes spanned gutter columns.
    fn cell_spanned_width(&self, x: usize, colspan: usize) -> Abs {
        self.rcols
            .iter()
            .skip(x)
            .take(if self.grid.has_gutter { 2 * colspan - 1 } else { colspan })
            .sum()
    }

    /// Measure the size that is available to auto columns.
    fn measure_auto_columns(
        &mut self,
        engine: &mut Engine,
        available: Abs,
    ) -> SourceResult<(Abs, usize)> {
        let mut auto = Abs::zero();
        let mut count = 0;
        let all_frac_cols = self
            .grid
            .cols
            .iter()
            .enumerate()
            .filter(|(_, col)| col.is_fractional())
            .map(|(x, _)| x)
            .collect::<Vec<_>>();

        // Determine size of auto columns by laying out all cells in those
        // columns, measuring them and finding the largest one.
        for (x, &col) in self.grid.cols.iter().enumerate() {
            if col != Sizing::Auto {
                continue;
            }

            let mut resolved = Abs::zero();
            for y in 0..self.grid.rows.len() {
                // We get the parent cell in case this is a merged position.
                let Some(Axes { x: parent_x, y: parent_y }) =
                    self.grid.parent_cell_position(x, y)
                else {
                    continue;
                };
                let cell = self.grid.cell(parent_x, parent_y).unwrap();
                let colspan = cell.colspan.get();
                if colspan > 1 {
                    let last_spanned_auto_col = self
                        .grid
                        .cols
                        .iter()
                        .enumerate()
                        .skip(parent_x)
                        .take(if self.grid.has_gutter {
                            2 * colspan - 1
                        } else {
                            colspan
                        })
                        .rev()
                        .find(|(_, col)| **col == Sizing::Auto)
                        .map(|(x, _)| x);

                    if last_spanned_auto_col != Some(x) {
                        // A colspan only affects the size of the last spanned
                        // auto column.
                        continue;
                    }
                }

                if colspan > 1
                    && self.regions.size.x.is_finite()
                    && !all_frac_cols.is_empty()
                    && all_frac_cols
                        .iter()
                        .all(|x| (parent_x..parent_x + colspan).contains(x))
                {
                    // Additionally, as a heuristic, a colspan won't affect the
                    // size of auto columns if it already spans all fractional
                    // columns, since those would already expand to provide all
                    // remaining available after auto column sizing to that
                    // cell. However, this heuristic is only valid in finite
                    // regions (pages without 'auto' width), since otherwise
                    // the fractional columns don't expand at all.
                    continue;
                }

                // For relative rows, we can already resolve the correct
                // base and for auto and fr we could only guess anyway.
                let height = match self.grid.rows[y] {
                    Sizing::Rel(v) => {
                        v.resolve(self.styles).relative_to(self.regions.base().y)
                    }
                    _ => self.regions.base().y,
                };
                // Don't expand this auto column more than the cell actually
                // needs. To do this, we check how much the other, previously
                // resolved columns provide to the cell in terms of width
                // (if it is a colspan), and subtract this from its expected
                // width when comparing with other cells in this column. Note
                // that, since this is the last auto column spanned by this
                // cell, all other auto columns will already have been resolved
                // and will be considered.
                // Only fractional columns will be excluded from this
                // calculation, which can lead to auto columns being expanded
                // unnecessarily when cells span both a fractional column and
                // an auto column. One mitigation for this is the heuristic
                // used above to not expand the last auto column spanned by a
                // cell if it spans all fractional columns in a finite region.
                let already_covered_width = self.cell_spanned_width(parent_x, colspan);

                let size = Size::new(available, height);
                let pod = Regions::one(size, Axes::splat(false));
                let frame = cell.measure(engine, self.styles, pod)?.into_frame();
                resolved.set_max(frame.width() - already_covered_width);
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

        for (&col, rcol) in self.grid.cols.iter().zip(&mut self.rcols) {
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

            for (&col, &rcol) in self.grid.cols.iter().zip(&self.rcols) {
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
        for (&col, rcol) in self.grid.cols.iter().zip(&mut self.rcols) {
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

        for x in 0..self.rcols.len() {
            if let Some(cell) = self.grid.cell(x, y) {
                let mut pod = self.regions;
                pod.size.x = self.cell_spanned_width(x, cell.colspan.get());

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
            if self.grid.has_gutter && y % 2 == 1 {
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

        // Reverse the column order when using RTL.
        for (x, &rcol) in self.rcols.iter().enumerate().rev_if(self.is_rtl) {
            if let Some(cell) = self.grid.cell(x, y) {
                let width = self.cell_spanned_width(x, cell.colspan.get());
                let size = Size::new(width, height);
                let mut pod = Regions::one(size, Axes::splat(true));
                if self.grid.rows[y] == Sizing::Auto {
                    pod.full = self.regions.full;
                }
                let mut frame = cell.layout(engine, self.styles, pod)?.into_frame();
                if self.is_rtl {
                    // In the grid, cell colspans expand to the right,
                    // so we're at the leftmost (lowest 'x') column
                    // spanned by the cell. However, in RTL, cells
                    // expand to the left. Therefore, without the
                    // offset below, the cell's contents would be laid out
                    // starting at its rightmost visual position and extend
                    // over to unrelated cells to its right in RTL.
                    // We avoid this by ensuring the rendered cell starts at
                    // the very left of the cell, even with colspan > 1.
                    let offset = Point::with_x(-width + rcol);
                    frame.translate(offset);
                }
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
        for (x, &rcol) in self.rcols.iter().enumerate().rev_if(self.is_rtl) {
            if let Some(cell) = self.grid.cell(x, y) {
                let width = self.cell_spanned_width(x, cell.colspan.get());
                pod.size.x = width;

                // Push the layouted frames into the individual output frames.
                let fragment = cell.layout(engine, self.styles, pod)?;
                for (output, mut frame) in outputs.iter_mut().zip(fragment) {
                    if self.is_rtl {
                        let offset = Point::with_x(-width + rcol);
                        frame.translate(offset);
                    }
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

/// Generates the segments of lines that should be drawn alongside a certain
/// axis in the grid, going through the given tracks (orthogonal to the lines).
/// Each returned segment contains its stroke, its offset from the start, and
/// its length.
/// Accepts, as parameters, the index of the lines that should be produced
/// (for example, the column at which vertical lines will be drawn); the
/// default stroke of lines at this index (used if there aren't any overrides
/// by intersecting cells or user-specified lines); a list of user-specified
/// lines with the same index (the `lines` parameter); and a function
/// which returns the final stroke that should be used for each track the line
/// goes through (its parameters are the grid, the track number, the index of
/// the line to be drawn and the default stroke at this index).
/// Contiguous segments with the same stroke are joined together automatically.
/// The function should return 'None' for positions at which the line would
/// otherwise cross a merged cell (for example, a vline could cross a colspan),
/// in which case a new segment should be drawn after the merged cell(s), even
/// if it would have the same stroke as the previous one.
/// Note that we assume that the tracks are sorted according to ascending
/// number, and they must be iterable over pairs of (number, size). For
/// vertical lines, for instance, 'tracks' would describe the rows in the
/// current region, as pairs (row index, row height).
fn generate_line_segments<F>(
    grid: &CellGrid,
    tracks: impl IntoIterator<Item = (usize, Abs)>,
    index: usize,
    stroke: Option<&Arc<Stroke<Abs>>>,
    lines: &[Line],
    line_stroke_at_track: F,
) -> impl IntoIterator<Item = (Arc<Stroke<Abs>>, Abs, Abs)>
where
    F: Fn(&CellGrid, usize, usize, Option<Arc<Stroke<Abs>>>) -> Option<Arc<Stroke<Abs>>>,
{
    // Each line segment that should be drawn.
    // The last element in the vector below is the currently drawn segment.
    // That is, the last segment will be expanded until interrupted.
    let mut drawn_lines = vec![];
    // Whether the latest line segment is complete, because we hit a row we
    // should skip while drawing the line. Starts at true so we push the first
    // segment to the vector.
    let mut interrupted = true;
    // How far from the start (before the first track) have we gone so far.
    // Used to determine the positions at which to draw each segment.
    let mut offset = Abs::zero();

    // We start drawing at the first suitable track, and keep going through
    // tracks (of increasing numbers) expanding the last segment until we hit
    // a track next to which we shouldn't draw, which is skipped, or we find a
    // stroke override (either by a cell or by a user-specified line),
    // requiring us to use a different stroke. Both cases lead to the creation
    // of a new line segment later if a suitable track is found, restarting the
    // cycle.
    for (track, size) in tracks {
        // Get the expected stroke at this track by folding the strokes of each
        // user-specified line going through the current position, with
        // priority to the line specified last, and then folding the resulting
        // stroke with the default stroke at this position (with priority to
        // the stroke resulting from user-specified lines).
        let stroke = lines
            .iter()
            .filter(|line| {
                line.end
                    .map(|end| (line.start..end.get()).contains(&track))
                    .unwrap_or_else(|| track >= line.start)
            })
            .fold(stroke.cloned(), |stroke, line| {
                match (stroke, line.stroke.as_ref().cloned()) {
                    // Fold with priority to the line specified last.
                    (Some(stroke), Some(line_stroke)) => Some(line_stroke.fold(stroke)),
                    (stroke, line_stroke) => stroke.or(line_stroke),
                }
            });

        // The function shall determine if it is appropriate to draw the line
        // at this position or not (i.e. whether or not it would cross a merged
        // cell), and, if so, the final stroke it should have (because cells
        // near this position could have stroke overrides, which have priority
        // and should be folded with the stroke obtained above).
        if let Some(stroke) = line_stroke_at_track(grid, index, track, stroke) {
            if interrupted {
                // Last segment was interrupted by a merged cell, had a stroke
                // of 'None', or there are no segments yet.
                // Create a new segment to draw. We start spanning this track.
                drawn_lines.push((stroke, offset, size));
                interrupted = false;
            } else {
                // The vector can't be empty if interrupted is false.
                let current_segment = drawn_lines.last_mut().unwrap();
                if current_segment.0 == stroke {
                    // Extend the current segment so it covers at least this
                    // track as well, since it has the same stroke in this
                    // track.
                    current_segment.2 += size;
                } else {
                    // We got a different stroke now, so create a new segment.
                    drawn_lines.push((stroke, offset, size));
                }
            }
        } else {
            interrupted = true;
        }
        offset += size;
    }

    drawn_lines
}

/// Returns the correct stroke with which to draw a vline right before column
/// 'x' when going through row 'y', given its initial stroke.
/// If the vline would go through a colspan, returns None (shouldn't be drawn).
/// If the one (when at the border) or two (otherwise) cells to the left and
/// right of the vline have right and left stroke overrides, respectively,
/// then the cells' stroke overrides are folded together with the vline's
/// stroke (with priority to the right cell's stroke, followed by the left
/// cell's) and returned. If, however, the cells around the vline at this row
/// do not have any stroke overrides, then the vline's own stroke is returned.
fn vline_stroke_at_row(
    grid: &CellGrid,
    x: usize,
    y: usize,
    stroke: Option<Arc<Stroke<Abs>>>,
) -> Option<Arc<Stroke<Abs>>> {
    if x != 0 && x != grid.cols.len() {
        // When the vline isn't at the border, we need to check if a colspan would
        // be present between columns 'x' and 'x-1' at row 'y', and thus overlap
        // with the line.
        // To do so, we analyze the cell right after this vline. If it is merged
        // with a cell before this line (parent_x < x) which is at this row or
        // above it (parent_y <= y), this means it would overlap with the vline,
        // so the vline must not be drawn at this row.
        let first_adjacent_cell = if grid.has_gutter {
            // Skip the gutters, if x or y represent gutter tracks.
            // We would then analyze the cell one column after (if at a gutter
            // column), and/or one row below (if at a gutter row), in order to
            // check if it would be merged with a cell before the vline.
            (x + x % 2, y + y % 2)
        } else {
            (x, y)
        };
        let Axes { x: parent_x, y: parent_y } = grid
            .parent_cell_position(first_adjacent_cell.0, first_adjacent_cell.1)
            .unwrap();

        if parent_x < x && parent_y <= y {
            // There is a colspan cell going through this vline's position,
            // so don't draw it here.
            return None;
        }
    }

    let left_cell_stroke = x
        .checked_sub(1)
        .and_then(|left_x| grid.parent_cell(left_x, y))
        .and_then(|left_cell| left_cell.stroke.right.as_ref());
    let right_cell_stroke = if x < grid.cols.len() {
        grid.parent_cell(x, y)
            .and_then(|right_cell| right_cell.stroke.left.as_ref())
    } else {
        None
    };

    let cell_stroke = match (left_cell_stroke.cloned(), right_cell_stroke.cloned()) {
        (Some(left_cell_stroke), Some(right_cell_stroke)) => {
            // When both cells specify a stroke for this line segment, fold
            // both strokes, with priority to the right cell's left stroke.
            Some(right_cell_stroke.fold(left_cell_stroke))
        }
        // When one of the cells doesn't specify a stroke, the other cell's
        // stroke should be used.
        (left_cell_stroke, right_cell_stroke) => left_cell_stroke.or(right_cell_stroke),
    };

    // Fold the line stroke and folded cell strokes, if possible.
    // Otherwise, use whichever of the two isn't 'none' or unspecified.
    match (cell_stroke, stroke) {
        (Some(cell_stroke), Some(stroke)) => Some(cell_stroke.fold(stroke.clone())),
        (cell_stroke, stroke) => cell_stroke.or_else(|| stroke.clone()),
    }
}

/// Returns the correct stroke with which to draw a hline on top of row 'y'
/// when going through column 'x', given its initial stroke.
/// If the one (when at the border) or two (otherwise) cells above and below
/// the hline have bottom and top stroke overrides, respectively, then the
/// cells' stroke overrides are folded together with the hline's stroke (with
/// priority to the bottom cell's stroke, followed by the top cell's) and
/// returned. If, however, the cells around the hline at this column do not
/// have any stroke overrides, then the hline's own stroke is returned.
fn hline_stroke_at_column(
    grid: &CellGrid,
    y: usize,
    x: usize,
    stroke: Option<Arc<Stroke<Abs>>>,
) -> Option<Arc<Stroke<Abs>>> {
    // There are no rowspans yet, so no need to add a check here. The line will
    // always be drawn, if it has a stroke.
    let top_cell_stroke = y
        .checked_sub(1)
        .and_then(|top_y| grid.parent_cell(x, top_y))
        .and_then(|top_cell| top_cell.stroke.bottom.as_ref());
    let bottom_cell_stroke = if y < grid.rows.len() {
        grid.parent_cell(x, y)
            .and_then(|bottom_cell| bottom_cell.stroke.top.as_ref())
    } else {
        None
    };

    let cell_stroke = match (top_cell_stroke.cloned(), bottom_cell_stroke.cloned()) {
        (Some(top_cell_stroke), Some(bottom_cell_stroke)) => {
            // When both cells specify a stroke for this line segment, fold
            // both strokes, with priority to the bottom cell's top stroke.
            Some(bottom_cell_stroke.fold(top_cell_stroke))
        }
        // When one of the cells doesn't specify a stroke, the other cell's
        // stroke should be used.
        (top_cell_stroke, bottom_cell_stroke) => top_cell_stroke.or(bottom_cell_stroke),
    };

    // Fold the line stroke and folded cell strokes, if possible.
    // Otherwise, use whichever of the two isn't 'none' or unspecified.
    match (cell_stroke, stroke) {
        (Some(cell_stroke), Some(stroke)) => Some(cell_stroke.fold(stroke.clone())),
        (cell_stroke, stroke) => cell_stroke.or_else(|| stroke.clone()),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn sample_cell() -> Cell {
        Cell {
            body: Content::default(),
            fill: None,
            colspan: NonZeroUsize::ONE,
            stroke: Sides::default(),
        }
    }

    fn cell_with_colspan(colspan: usize) -> Cell {
        Cell {
            body: Content::default(),
            fill: None,
            colspan: NonZeroUsize::try_from(colspan).unwrap(),
            stroke: Sides::default(),
        }
    }

    fn sample_grid(gutters: bool) -> CellGrid {
        const COLS: usize = 4;
        const ROWS: usize = 6;
        let entries = vec![
            // row 0
            Entry::Cell(sample_cell()),
            Entry::Cell(sample_cell()),
            Entry::Cell(cell_with_colspan(2)),
            Entry::Merged { parent: 2 },
            // row 1
            Entry::Cell(sample_cell()),
            Entry::Cell(cell_with_colspan(3)),
            Entry::Merged { parent: 5 },
            Entry::Merged { parent: 5 },
            // row 2
            Entry::Merged { parent: 4 },
            Entry::Cell(sample_cell()),
            Entry::Cell(cell_with_colspan(2)),
            Entry::Merged { parent: 10 },
            // row 3
            Entry::Cell(sample_cell()),
            Entry::Cell(cell_with_colspan(3)),
            Entry::Merged { parent: 13 },
            Entry::Merged { parent: 13 },
            // row 4
            Entry::Cell(sample_cell()),
            Entry::Merged { parent: 13 },
            Entry::Merged { parent: 13 },
            Entry::Merged { parent: 13 },
            // row 5
            Entry::Cell(sample_cell()),
            Entry::Cell(sample_cell()),
            Entry::Cell(cell_with_colspan(2)),
            Entry::Merged { parent: 22 },
        ];
        CellGrid::new_internal(
            Axes::with_x(&[Sizing::Auto; COLS]),
            if gutters {
                Axes::new(&[Sizing::Auto; COLS - 1], &[Sizing::Auto; ROWS - 1])
            } else {
                Axes::default()
            },
            ResolvedGridStroke::default(),
            vec![],
            vec![],
            entries,
        )
    }

    #[test]
    fn test_vline_splitting_without_gutter() {
        let stroke = Arc::new(Stroke::default());
        let grid = sample_grid(false);
        let rows = &[
            RowPiece { height: Abs::pt(1.0), y: 0 },
            RowPiece { height: Abs::pt(2.0), y: 1 },
            RowPiece { height: Abs::pt(4.0), y: 2 },
            RowPiece { height: Abs::pt(8.0), y: 3 },
            RowPiece { height: Abs::pt(16.0), y: 4 },
            RowPiece { height: Abs::pt(32.0), y: 5 },
        ];
        let expected_vline_splits = &[
            vec![(stroke.clone(), Abs::pt(0.), Abs::pt(1. + 2. + 4. + 8. + 16. + 32.))],
            vec![(stroke.clone(), Abs::pt(0.), Abs::pt(1. + 2. + 4. + 8. + 16. + 32.))],
            // interrupted a few times by colspans
            vec![
                (stroke.clone(), Abs::pt(0.), Abs::pt(1.)),
                (stroke.clone(), Abs::pt(1. + 2.), Abs::pt(4.)),
                (stroke.clone(), Abs::pt(1. + 2. + 4. + 8. + 16.), Abs::pt(32.)),
            ],
            // interrupted every time by colspans
            vec![],
            vec![(stroke.clone(), Abs::pt(0.), Abs::pt(1. + 2. + 4. + 8. + 16. + 32.))],
        ];
        for (x, expected_splits) in expected_vline_splits.iter().enumerate() {
            let tracks = rows.iter().map(|row| (row.y, row.height));
            assert_eq!(
                expected_splits,
                &generate_line_segments(
                    &grid,
                    tracks,
                    x,
                    Some(&stroke),
                    &[],
                    vline_stroke_at_row
                )
                .into_iter()
                .collect::<Vec<_>>(),
            );
        }
    }

    #[test]
    fn test_vline_splitting_with_gutter() {
        let stroke = Arc::new(Stroke::default());
        let grid = sample_grid(true);
        let rows = &[
            RowPiece { height: Abs::pt(1.0), y: 0 },
            RowPiece { height: Abs::pt(2.0), y: 1 },
            RowPiece { height: Abs::pt(4.0), y: 2 },
            RowPiece { height: Abs::pt(8.0), y: 3 },
            RowPiece { height: Abs::pt(16.0), y: 4 },
            RowPiece { height: Abs::pt(32.0), y: 5 },
            RowPiece { height: Abs::pt(64.0), y: 6 },
            RowPiece { height: Abs::pt(128.0), y: 7 },
            RowPiece { height: Abs::pt(256.0), y: 8 },
            RowPiece { height: Abs::pt(512.0), y: 9 },
            RowPiece { height: Abs::pt(1024.0), y: 10 },
        ];
        let expected_vline_splits = &[
            // left border
            vec![(
                stroke.clone(),
                Abs::pt(0.),
                Abs::pt(1. + 2. + 4. + 8. + 16. + 32. + 64. + 128. + 256. + 512. + 1024.),
            )],
            // gutter line below
            vec![(
                stroke.clone(),
                Abs::pt(0.),
                Abs::pt(1. + 2. + 4. + 8. + 16. + 32. + 64. + 128. + 256. + 512. + 1024.),
            )],
            vec![(
                stroke.clone(),
                Abs::pt(0.),
                Abs::pt(1. + 2. + 4. + 8. + 16. + 32. + 64. + 128. + 256. + 512. + 1024.),
            )],
            // gutter line below
            // the two lines below are interrupted multiple times by colspans
            vec![
                (stroke.clone(), Abs::pt(0.), Abs::pt(1. + 2.)),
                (stroke.clone(), Abs::pt(1. + 2. + 4.), Abs::pt(8. + 16. + 32.)),
                (
                    stroke.clone(),
                    Abs::pt(1. + 2. + 4. + 8. + 16. + 32. + 64. + 128. + 256.),
                    Abs::pt(512. + 1024.),
                ),
            ],
            vec![
                (stroke.clone(), Abs::pt(0.), Abs::pt(1. + 2.)),
                (stroke.clone(), Abs::pt(1. + 2. + 4.), Abs::pt(8. + 16. + 32.)),
                (
                    stroke.clone(),
                    Abs::pt(1. + 2. + 4. + 8. + 16. + 32. + 64. + 128. + 256.),
                    Abs::pt(512. + 1024.),
                ),
            ],
            // gutter line below
            // the two lines below can only cross certain gutter rows, because
            // all non-gutter cells in the following column are merged with
            // cells from the previous column.
            vec![
                (stroke.clone(), Abs::pt(1.), Abs::pt(2.)),
                (stroke.clone(), Abs::pt(1. + 2. + 4.), Abs::pt(8.)),
                (stroke.clone(), Abs::pt(1. + 2. + 4. + 8. + 16.), Abs::pt(32.)),
                (
                    stroke.clone(),
                    Abs::pt(1. + 2. + 4. + 8. + 16. + 32. + 64. + 128. + 256.),
                    Abs::pt(512.),
                ),
            ],
            vec![
                (stroke.clone(), Abs::pt(1.), Abs::pt(2.)),
                (stroke.clone(), Abs::pt(1. + 2. + 4.), Abs::pt(8.)),
                (stroke.clone(), Abs::pt(1. + 2. + 4. + 8. + 16.), Abs::pt(32.)),
                (
                    stroke.clone(),
                    Abs::pt(1. + 2. + 4. + 8. + 16. + 32. + 64. + 128. + 256.),
                    Abs::pt(512.),
                ),
            ],
            // right border
            vec![(
                stroke.clone(),
                Abs::pt(0.),
                Abs::pt(1. + 2. + 4. + 8. + 16. + 32. + 64. + 128. + 256. + 512. + 1024.),
            )],
        ];
        for (x, expected_splits) in expected_vline_splits.iter().enumerate() {
            let tracks = rows.iter().map(|row| (row.y, row.height));
            assert_eq!(
                expected_splits,
                &generate_line_segments(
                    &grid,
                    tracks,
                    x,
                    Some(&stroke),
                    &[],
                    vline_stroke_at_row
                )
                .into_iter()
                .collect::<Vec<_>>(),
            );
        }
    }
}
