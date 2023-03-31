use crate::layout::{AlignElem, GridLayouter, TrackSizings};
use crate::meta::LocalName;
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
/// Display: Table
/// Category: layout
#[element(Layout, LocalName)]
pub struct TableElem {
    /// Defines the column sizes. See the [grid documentation]($func/grid) for
    /// more information on track sizing.
    pub columns: TrackSizings,

    /// Defines the row sizes. See the [grid documentation]($func/grid) for more
    /// information on track sizing.
    pub rows: TrackSizings,

    /// Defines the gaps between rows & columns. See the [grid
    /// documentation]($func/grid) for more information on gutters.
    #[external]
    pub gutter: TrackSizings,

    /// Defines the gaps between columns. Takes precedence over `gutter`. See
    /// the [grid documentation]($func/grid) for more information on gutters.
    #[parse(
        let gutter = args.named("gutter")?;
        args.named("column-gutter")?.or_else(|| gutter.clone())
    )]
    pub column_gutter: TrackSizings,

    /// Defines the gaps between rows. Takes precedence over `gutter`. See the
    /// [grid documentation]($func/grid) for more information on gutters.
    #[parse(args.named("row-gutter")?.or_else(|| gutter.clone()))]
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
    pub fill: Celled<Option<Paint>>,

    /// How to align the cell's content.
    ///
    /// This can either be a single alignment or a function that returns an
    /// alignment. The function is passed the cell's column and row index,
    /// starting at zero. If set to `{auto}`, the outer alignment is used.
    ///
    /// ```example
    /// #table(
    ///   columns: 3,
    ///   align: (x, y) => (left, center, right).at(x),
    ///   [Hello], [Hello], [Hello],
    ///   [A], [B], [C],
    /// )
    /// ```
    pub align: Celled<Smart<Axes<Option<GenAlign>>>>,

    /// How to stroke the cells.
    ///
    /// This can be a color, a stroke width, both, or `{none}` to disable
    /// the stroke.
    #[resolve]
    #[fold]
    #[default(Some(PartialStroke::default()))]
    pub stroke: Option<PartialStroke>,

    /// How much to pad the cells's content.
    ///
    /// The default value is `{5pt}`.
    #[default(Abs::pt(5.0).into())]
    pub inset: Rel<Length>,

    /// The contents of the table cells.
    #[variadic]
    pub children: Vec<Content>,
}

impl Layout for TableElem {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let inset = self.inset(styles);
        let align = self.align(styles);

        let tracks = Axes::new(self.columns(styles).0, self.rows(styles).0);
        let gutter = Axes::new(self.column_gutter(styles).0, self.row_gutter(styles).0);
        let cols = tracks.x.len().max(1);
        let cells: Vec<_> = self
            .children()
            .into_iter()
            .enumerate()
            .map(|(i, child)| {
                let mut child = child.padded(Sides::splat(inset));

                let x = i % cols;
                let y = i / cols;
                if let Smart::Custom(alignment) = align.resolve(vt, x, y)? {
                    child = child.styled(AlignElem::set_alignment(alignment));
                }

                Ok(child)
            })
            .collect::<SourceResult<_>>()?;

        let fill = self.fill(styles);
        let stroke = self.stroke(styles).map(PartialStroke::unwrap_or_default);

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
            if let Some(stroke) = &stroke {
                let thickness = stroke.thickness;
                let half = thickness / 2.0;

                // Render horizontal lines.
                for offset in points(rows.iter().map(|piece| piece.height)) {
                    let target = Point::with_x(frame.width() + thickness);
                    let hline = Geometry::Line(target).stroked(stroke.clone());
                    frame.prepend(
                        Point::new(-half, offset),
                        FrameItem::Shape(hline, self.span()),
                    );
                }

                // Render vertical lines.
                for offset in points(layout.cols.iter().copied()) {
                    let target = Point::with_y(frame.height() + thickness);
                    let vline = Geometry::Line(target).stroked(stroke.clone());
                    frame.prepend(
                        Point::new(offset, -half),
                        FrameItem::Shape(vline, self.span()),
                    );
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
                        frame.prepend(pos, FrameItem::Shape(rect, self.span()));
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
    pub fn resolve(&self, vt: &mut Vt, x: usize, y: usize) -> SourceResult<T> {
        Ok(match self {
            Self::Value(value) => value.clone(),
            Self::Func(func) => func
                .call_vt(vt, [Value::Int(x as i64), Value::Int(y as i64)])?
                .cast()
                .at(func.span())?,
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

impl LocalName for TableElem {
    fn local_name(&self, lang: Lang) -> &'static str {
        match lang {
<<<<<<< HEAD
            Lang::FRENCH => "Tableau",
=======
            Lang::CHINESE => "表",
>>>>>>> 631ba40e57b4f121fe9335f334a76dd9c81de088
            Lang::GERMAN => "Tabelle",
            Lang::ITALIAN => "Tabella",
            Lang::RUSSIAN => "Таблица",
            Lang::ENGLISH | _ => "Table",
        }
    }
}
