use std::num::NonZeroUsize;

use ecow::eco_format;

use crate::diag::{
    bail, At, Hint, HintedStrResult, HintedString, SourceResult, StrResult,
};
use crate::engine::Engine;
use crate::foundations::{
    Array, CastInfo, Content, FromValue, Func, IntoValue, Reflect, Resolve, Smart,
    StyleChain, Value,
};
use crate::layout::{
    Abs, Alignment, Axes, Dir, Fr, Fragment, Frame, FrameItem, LayoutMultiple, Length,
    Point, Regions, Rel, Sides, Size, Sizing,
};
use crate::syntax::Span;
use crate::text::TextElem;
use crate::util::{MaybeReverseIter, NonZeroExt, Numeric};
use crate::visualize::{FixedStroke, Geometry, Paint};

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

/// Represents a cell in CellGrid, to be laid out by GridLayouter.
#[derive(Clone)]
pub struct Cell {
    /// The cell's body.
    pub body: Content,
    /// The cell's fill.
    pub fill: Option<Paint>,
    /// The amount of columns spanned by the cell.
    pub colspan: NonZeroUsize,
}

impl From<Content> for Cell {
    /// Create a simple cell given its body.
    fn from(body: Content) -> Self {
        Self { body, fill: None, colspan: NonZeroUsize::ONE }
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

/// Used for cell-like elements which are aware of their final properties in
/// the table, and may have property overrides.
pub trait ResolvableCell {
    /// Resolves the cell's fields, given its coordinates and default grid-wide
    /// fill, align and inset properties.
    /// Returns a final Cell.
    fn resolve_cell(
        self,
        x: usize,
        y: usize,
        fill: &Option<Paint>,
        align: Smart<Alignment>,
        inset: Sides<Option<Rel<Length>>>,
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
        Self::new_internal(tracks, gutter, entries)
    }

    /// Resolves and positions all cells in the grid before creating it.
    /// Allows them to keep track of their final properties and positions
    /// and adjust their fields accordingly.
    /// Cells must implement Clone as they will be owned. Additionally, they
    /// must implement Default in order to fill positions in the grid which
    /// weren't explicitly specified by the user with empty cells.
    #[allow(clippy::too_many_arguments)]
    pub fn resolve<T: ResolvableCell + Clone + Default>(
        tracks: Axes<&[Sizing]>,
        gutter: Axes<&[Sizing]>,
        cells: &[T],
        fill: &Celled<Option<Paint>>,
        align: &Celled<Smart<Alignment>>,
        inset: Sides<Option<Rel<Length>>>,
        engine: &mut Engine,
        styles: StyleChain,
        span: Span,
    ) -> SourceResult<Self> {
        // Number of content columns: Always at least one.
        let c = tracks.x.len().max(1);

        // We can't just use the cell's index in the 'cells' vector to
        // determine its automatic position, since cells could have arbitrary
        // positions, so the position of a cell in 'cells' can differ from its
        // final position in 'resolved_cells' (see below).
        // Therefore, we use a counter, 'auto_index', to determine the position
        // of the next cell with (x: auto, y: auto). It is only stepped when
        // a cell with (x: auto, y: auto), usually the vast majority, is found.
        let mut auto_index = 0;

        // We have to rebuild the grid to account for arbitrary positions.
        // Create at least 'cells.len()' positions, since there will be at
        // least 'cells.len()' cells, even though some of them might be placed
        // in arbitrary positions and thus cause the grid to expand.
        // Additionally, make sure we allocate up to the next multiple of 'c',
        // since each row will have 'c' cells, even if the last few cells
        // weren't explicitly specified by the user.
        // We apply '% c' twice so that the amount of cells potentially missing
        // is zero when 'cells.len()' is already a multiple of 'c' (thus
        // 'cells.len() % c' would be zero).
        let Some(cell_count) = cells.len().checked_add((c - cells.len() % c) % c) else {
            bail!(span, "too many cells were given")
        };
        let mut resolved_cells: Vec<Option<Entry>> = Vec::with_capacity(cell_count);
        for cell in cells.iter().cloned() {
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

            // Let's resolve the cell so it can determine its own fields
            // based on its final position.
            let cell = cell.resolve_cell(
                x,
                y,
                &fill.resolve(engine, x, y)?,
                align.resolve(engine, x, y)?,
                inset,
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

                    // Ensure all absent entries are affected by show rules and
                    // grid styling by turning them into resolved empty cells.
                    let new_cell = T::default().resolve_cell(
                        x,
                        y,
                        &fill.resolve(engine, x, y)?,
                        align.resolve(engine, x, y)?,
                        inset,
                        styles,
                    );
                    Ok(Entry::Cell(new_cell))
                }
            })
            .collect::<SourceResult<Vec<Entry>>>()?;

        Ok(Self::new_internal(tracks, gutter, resolved_cells))
    }

    /// Generates the cell grid, given the tracks and resolved entries.
    fn new_internal(
        tracks: Axes<&[Sizing]>,
        gutter: Axes<&[Sizing]>,
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

        Self { cols, rows, entries, has_gutter }
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
    // How to stroke the cells.
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
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        grid: &'a CellGrid,
        stroke: &'a Option<FixedStroke>,
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
            stroke,
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
        for (frame, rows) in finished.iter_mut().zip(&self.rrows) {
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
                for (x, dx) in points(self.rcols.iter().copied()).enumerate() {
                    let dx = if self.is_rtl { self.width - dx } else { dx };
                    // We want each vline to span the entire table (start
                    // at y = 0, end after all rows).
                    // We use 'split_vline' to split the vline such that it
                    // is not drawn above colspans.
                    for (dy, length) in
                        split_vline(self.grid, rows, x, 0, self.grid.rows.len())
                    {
                        let target = Point::with_y(length + thickness);
                        let vline = Geometry::Line(target).stroked(stroke.clone());
                        frame.prepend(
                            Point::new(dx, dy - half),
                            FrameItem::Shape(vline, self.span),
                        );
                    }
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

/// Given the 'x' of the column right after the vline (or cols.len() at the
/// border) and its start..end range of rows, alongside the rows for the
/// current region, splits the vline into contiguous parts to draw, including
/// the height of the vline in each part. This will go through each row and
/// interrupt the current vline to be drawn when a colspan is detected, or the
/// end of the row range (or of the region) is reached.
/// The idea is to not draw vlines over colspans.
/// This will return the start offsets and lengths of each final segment of
/// this vline. The offsets are relative to the top of the first row.
/// Note that this assumes that rows are sorted according to ascending 'y'.
fn split_vline(
    grid: &CellGrid,
    rows: &[RowPiece],
    x: usize,
    start: usize,
    end: usize,
) -> impl IntoIterator<Item = (Abs, Abs)> {
    // Each segment of this vline that should be drawn.
    // The last element in the vector below is the currently drawn segment.
    // That is, the last segment will be expanded until interrupted.
    let mut drawn_vlines = vec![];
    // Whether the latest vline segment is complete, because we hit a row we
    // should skip while drawing the vline. Starts at true so we push
    // the first segment to the vector.
    let mut interrupted = true;
    // How far down from the first row have we gone so far.
    // Used to determine the positions at which to draw each segment.
    let mut offset = Abs::zero();

    // We start drawing at the first suitable row, and keep going down
    // (increasing y) expanding the last segment until we hit a row on top of
    // which we shouldn't draw, which is skipped, leading to the creation of a
    // new vline segment later if a suitable row is found, restarting the
    // cycle.
    for row in rows.iter().take_while(|row| row.y < end) {
        if should_draw_vline_at_row(grid, x, row.y, start, end) {
            if interrupted {
                // Last segment was interrupted by a colspan, or there are no
                // segments yet.
                // Create a new segment to draw. We start spanning this row.
                drawn_vlines.push((offset, row.height));
                interrupted = false;
            } else {
                // Extend the current segment so it covers at least this row
                // as well.
                // The vector can't be empty if interrupted is false.
                let current_segment = drawn_vlines.last_mut().unwrap();
                current_segment.1 += row.height;
            }
        } else {
            interrupted = true;
        }
        offset += row.height;
    }

    drawn_vlines
}

/// Returns 'true' if the vline right before column 'x', given its start..end
/// range of rows, should be drawn when going through row 'y'.
/// That only occurs if the row is within its start..end range, and if it
/// wouldn't go through a colspan.
fn should_draw_vline_at_row(
    grid: &CellGrid,
    x: usize,
    y: usize,
    start: usize,
    end: usize,
) -> bool {
    if !(start..end).contains(&y) {
        // Row is out of range for this line
        return false;
    }
    if x == 0 || x == grid.cols.len() {
        // Border vline. Always drawn.
        return true;
    }
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

    parent_x >= x || parent_y > y
}

#[cfg(test)]
mod test {
    use super::*;

    fn sample_cell() -> Cell {
        Cell {
            body: Content::default(),
            fill: None,
            colspan: NonZeroUsize::ONE,
        }
    }

    fn cell_with_colspan(colspan: usize) -> Cell {
        Cell {
            body: Content::default(),
            fill: None,
            colspan: NonZeroUsize::try_from(colspan).unwrap(),
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
            entries,
        )
    }

    #[test]
    fn test_vline_splitting_without_gutter() {
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
            vec![(Abs::pt(0.), Abs::pt(1. + 2. + 4. + 8. + 16. + 32.))],
            vec![(Abs::pt(0.), Abs::pt(1. + 2. + 4. + 8. + 16. + 32.))],
            // interrupted a few times by colspans
            vec![
                (Abs::pt(0.), Abs::pt(1.)),
                (Abs::pt(1. + 2.), Abs::pt(4.)),
                (Abs::pt(1. + 2. + 4. + 8. + 16.), Abs::pt(32.)),
            ],
            // interrupted every time by colspans
            vec![],
            vec![(Abs::pt(0.), Abs::pt(1. + 2. + 4. + 8. + 16. + 32.))],
        ];
        for (x, expected_splits) in expected_vline_splits.iter().enumerate() {
            assert_eq!(
                expected_splits,
                &split_vline(&grid, rows, x, 0, 6).into_iter().collect::<Vec<_>>(),
            );
        }
    }

    #[test]
    fn test_vline_splitting_with_gutter() {
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
                Abs::pt(0.),
                Abs::pt(1. + 2. + 4. + 8. + 16. + 32. + 64. + 128. + 256. + 512. + 1024.),
            )],
            // gutter line below
            vec![(
                Abs::pt(0.),
                Abs::pt(1. + 2. + 4. + 8. + 16. + 32. + 64. + 128. + 256. + 512. + 1024.),
            )],
            vec![(
                Abs::pt(0.),
                Abs::pt(1. + 2. + 4. + 8. + 16. + 32. + 64. + 128. + 256. + 512. + 1024.),
            )],
            // gutter line below
            // the two lines below are interrupted multiple times by colspans
            vec![
                (Abs::pt(0.), Abs::pt(1. + 2.)),
                (Abs::pt(1. + 2. + 4.), Abs::pt(8. + 16. + 32.)),
                (
                    Abs::pt(1. + 2. + 4. + 8. + 16. + 32. + 64. + 128. + 256.),
                    Abs::pt(512. + 1024.),
                ),
            ],
            vec![
                (Abs::pt(0.), Abs::pt(1. + 2.)),
                (Abs::pt(1. + 2. + 4.), Abs::pt(8. + 16. + 32.)),
                (
                    Abs::pt(1. + 2. + 4. + 8. + 16. + 32. + 64. + 128. + 256.),
                    Abs::pt(512. + 1024.),
                ),
            ],
            // gutter line below
            // the two lines below can only cross certain gutter rows, because
            // all non-gutter cells in the following column are merged with
            // cells from the previous column.
            vec![
                (Abs::pt(1.), Abs::pt(2.)),
                (Abs::pt(1. + 2. + 4.), Abs::pt(8.)),
                (Abs::pt(1. + 2. + 4. + 8. + 16.), Abs::pt(32.)),
                (
                    Abs::pt(1. + 2. + 4. + 8. + 16. + 32. + 64. + 128. + 256.),
                    Abs::pt(512.),
                ),
            ],
            vec![
                (Abs::pt(1.), Abs::pt(2.)),
                (Abs::pt(1. + 2. + 4.), Abs::pt(8.)),
                (Abs::pt(1. + 2. + 4. + 8. + 16.), Abs::pt(32.)),
                (
                    Abs::pt(1. + 2. + 4. + 8. + 16. + 32. + 64. + 128. + 256.),
                    Abs::pt(512.),
                ),
            ],
            // right border
            vec![(
                Abs::pt(0.),
                Abs::pt(1. + 2. + 4. + 8. + 16. + 32. + 64. + 128. + 256. + 512. + 1024.),
            )],
        ];
        for (x, expected_splits) in expected_vline_splits.iter().enumerate() {
            assert_eq!(
                expected_splits,
                &split_vline(&grid, rows, x, 0, 11).into_iter().collect::<Vec<_>>(),
            );
        }
    }
}
