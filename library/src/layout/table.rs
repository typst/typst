use crate::layout::{AlignNode, GridLayouter, TrackSizings};
use crate::prelude::*;

/// A table of items.
///
/// Tables are used to arrange content in cells. Cells can contain arbitrary
/// content, including multiple paragraphs and are specified in row-major order.
/// Because tables are just grids with configurable cell properties, refer to
/// the [grid documentation]($func/grid) for more information on how to size the
/// table tracks.
///
/// ## Example
/// ```example
/// #table(
///   columns: (1fr, auto, auto),
///   inset: 10pt,
///   align: horizon,
///   [], [*Area*], [*Parameters*],
///   image("cylinder.svg"),
///   $ pi h (D^2 - d^2) / 4 $,
///   [
///     $h$: height \
///     $D$: outer radius \
///     $d$: inner radius
///   ],
///   image("tetrahedron.svg"),
///   $ sqrt(2) / 12 a^3 $,
///   [$a$: edge length]
/// )
/// ```
///
/// ## Parameters
/// - gutter: `TrackSizings` (named)
///   Defines the gaps between rows & columns.
///   See the [grid documentation]($func/grid) for more information on gutters.
///
/// Display: Table
/// Category: layout
#[node(Layout)]
pub struct TableNode {
    /// The contents of the table cells.
    #[variadic]
    pub cells: Vec<Content>,

    /// Defines the column sizes.
    /// See the [grid documentation]($func/grid) for more information on track
    /// sizing.
    #[named]
    #[default]
    pub columns: TrackSizings,

    /// Defines the row sizes.
    /// See the [grid documentation]($func/grid) for more information on track
    /// sizing.
    #[named]
    #[default]
    pub rows: TrackSizings,

    /// Defines the gaps between columns. Takes precedence over `gutter`.
    /// See the [grid documentation]($func/grid) for more information on gutters.
    #[named]
    #[shorthand(gutter)]
    #[default]
    pub column_gutter: TrackSizings,

    /// Defines the gaps between rows. Takes precedence over `gutter`.
    /// See the [grid documentation]($func/grid) for more information on gutters.
    #[named]
    #[shorthand(gutter)]
    #[default]
    pub row_gutter: TrackSizings,

    /// How to fill the cells.
    ///
    /// This can be a color or a function that returns a color. The function is
    /// passed the cell's column and row index, starting at zero. This can be
    /// used to implement striped tables.
    ///
    /// ```example
    /// #table(
    ///   fill: (col, _) => if calc.odd(col) { luma(240) } else { white },
    ///   align: (col, row) =>
    ///     if row == 0 { center }
    ///     else if col == 0 { left }
    ///     else { right },
    ///   columns: 4,
    ///   [], [*Q1*], [*Q2*], [*Q3*],
    ///   [Revenue:], [1000 €], [2000 €], [3000 €],
    ///   [Expenses:], [500 €], [1000 €], [1500 €],
    ///   [Profit:], [500 €], [1000 €], [1500 €],
    /// )
    /// ```
    #[settable]
    #[default]
    pub fill: Celled<Option<Paint>>,

    /// How to align the cell's content.
    ///
    /// This can either be a single alignment or a function that returns an
    /// alignment. The function is passed the cell's column and row index,
    /// starting at zero. If set to `{auto}`, the outer alignment is used.
    #[settable]
    #[default]
    pub align: Celled<Smart<Axes<Option<GenAlign>>>>,

    /// How to stroke the cells.
    ///
    /// This can be a color, a stroke width, both, or `{none}` to disable
    /// the stroke.
    #[settable]
    #[resolve]
    #[fold]
    #[default(Some(PartialStroke::default()))]
    pub stroke: Option<PartialStroke>,

    /// How much to pad the cells's content.
    ///
    /// The default value is `{5pt}`.
    #[settable]
    #[default(Abs::pt(5.0).into())]
    pub inset: Rel<Length>,
}

impl Layout for TableNode {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let inset = styles.get(Self::INSET);
        let align = styles.get(Self::ALIGN);

        let tracks = Axes::new(self.columns().0, self.rows().0);
        let gutter = Axes::new(self.column_gutter().0, self.row_gutter().0);
        let cols = tracks.x.len().max(1);
        let cells: Vec<_> = self
            .cells()
            .into_iter()
            .enumerate()
            .map(|(i, child)| {
                let mut child = child.padded(Sides::splat(inset));

                let x = i % cols;
                let y = i / cols;
                if let Smart::Custom(alignment) = align.resolve(vt, x, y)? {
                    child = child.styled(AlignNode::ALIGNMENT, alignment)
                }

                Ok(child)
            })
            .collect::<SourceResult<_>>()?;

        let fill = styles.get(Self::FILL);
        let stroke = styles.get(Self::STROKE).map(PartialStroke::unwrap_or_default);

        // Prepare grid layout by unifying content and gutter tracks.
        let layouter = GridLayouter::new(
            vt,
            tracks.as_deref(),
            gutter.as_deref(),
            &cells,
            regions,
            styles,
        );

        // Measure the columns and layout the grid row-by-row.
        let mut layout = layouter.layout()?;

        // Add lines and backgrounds.
        for (frame, rows) in layout.fragment.iter_mut().zip(&layout.rows) {
            // Render table lines.
            if let Some(stroke) = stroke {
                let thickness = stroke.thickness;
                let half = thickness / 2.0;

                // Render horizontal lines.
                for offset in points(rows.iter().map(|piece| piece.height)) {
                    let target = Point::with_x(frame.width() + thickness);
                    let hline = Geometry::Line(target).stroked(stroke);
                    frame.prepend(Point::new(-half, offset), Element::Shape(hline));
                }

                // Render vertical lines.
                for offset in points(layout.cols.iter().copied()) {
                    let target = Point::with_y(frame.height() + thickness);
                    let vline = Geometry::Line(target).stroked(stroke);
                    frame.prepend(Point::new(offset, -half), Element::Shape(vline));
                }
            }

            // Render cell backgrounds.
            let mut dx = Abs::zero();
            for (x, &col) in layout.cols.iter().enumerate() {
                let mut dy = Abs::zero();
                for row in rows {
                    if let Some(fill) = fill.resolve(vt, x, row.y)? {
                        let pos = Point::new(dx, dy);
                        let size = Size::new(col, row.height);
                        let rect = Geometry::Rect(size).filled(fill);
                        frame.prepend(pos, Element::Shape(rect));
                    }
                    dy += row.height;
                }
                dx += col;
            }
        }

        Ok(layout.fragment)
    }
}

/// Turn an iterator extents into an iterator of offsets before, in between, and
/// after the extents, e.g. [10mm, 5mm] -> [0mm, 10mm, 15mm].
fn points(extents: impl IntoIterator<Item = Abs>) -> impl Iterator<Item = Abs> {
    let mut offset = Abs::zero();
    std::iter::once(Abs::zero())
        .chain(extents.into_iter())
        .map(move |extent| {
            offset += extent;
            offset
        })
}

/// A value that can be configured per cell.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Celled<T> {
    /// A bare value, the same for all cells.
    Value(T),
    /// A closure mapping from cell coordinates to a value.
    Func(Func),
}

impl<T: Cast + Clone> Celled<T> {
    /// Resolve the value based on the cell position.
    pub fn resolve(&self, vt: &Vt, x: usize, y: usize) -> SourceResult<T> {
        Ok(match self {
            Self::Value(value) => value.clone(),
            Self::Func(func) => {
                let args =
                    Args::new(func.span(), [Value::Int(x as i64), Value::Int(y as i64)]);
                func.call_detached(vt.world(), args)?.cast().at(func.span())?
            }
        })
    }
}

impl<T: Default> Default for Celled<T> {
    fn default() -> Self {
        Self::Value(T::default())
    }
}

impl<T: Cast> Cast for Celled<T> {
    fn is(value: &Value) -> bool {
        matches!(value, Value::Func(_)) || T::is(value)
    }

    fn cast(value: Value) -> StrResult<Self> {
        match value {
            Value::Func(v) => Ok(Self::Func(v)),
            v if T::is(&v) => Ok(Self::Value(T::cast(v)?)),
            v => <Self as Cast>::error(v),
        }
    }

    fn describe() -> CastInfo {
        T::describe() + CastInfo::Type("function")
    }
}

impl<T: Into<Value>> From<Celled<T>> for Value {
    fn from(celled: Celled<T>) -> Self {
        match celled {
            Celled::Value(value) => value.into(),
            Celled::Func(func) => func.into(),
        }
    }
}
