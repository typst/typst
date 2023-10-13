use typst::model::Resolve;

use super::*;

const DEFAULT_ROW_GAP: Em = Em::new(0.5);
const DEFAULT_COL_GAP: Em = Em::new(0.5);
const VERTICAL_PADDING: Ratio = Ratio::new(0.1);

const DEFAULT_STROKE_THICKNESS: Em = Em::new(0.05);

/// A column vector.
///
/// Content in the vector's elements can be aligned with the `&` symbol.
///
/// # Example
/// ```example
/// $ vec(a, b, c) dot vec(1, 2, 3)
///     = a + 2b + 3c $
/// ```
#[elem(title = "Vector", LayoutMath)]
pub struct VecElem {
    /// The delimiter to use.
    ///
    /// ```example
    /// #set math.vec(delim: "[")
    /// $ vec(1, 2) $
    /// ```
    #[default(Some(Delimiter::Paren))]
    pub delim: Option<Delimiter>,

    /// The gap between elements.
    ///
    /// ```example
    /// #set math.vec(gap: 1em)
    /// $ vec(1, 2) $
    /// ```
    #[resolve]
    #[default(DEFAULT_ROW_GAP.into())]
    pub gap: Rel<Length>,

    /// The elements of the vector.
    #[variadic]
    pub children: Vec<Content>,
}

impl LayoutMath for VecElem {
    #[tracing::instrument(skip(ctx))]
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        let delim = self.delim(ctx.styles());
        let frame = layout_vec_body(
            ctx,
            &self.children(),
            FixedAlign::Center,
            self.gap(ctx.styles()),
        )?;
        layout_delimiters(
            ctx,
            frame,
            delim.map(Delimiter::open),
            delim.map(Delimiter::close),
            self.span(),
        )
    }
}

/// A matrix.
///
/// The elements of a row should be separated by commas, while the rows
/// themselves should be separated by semicolons. The semicolon syntax merges
/// preceding arguments separated by commas into an array. You can also use this
/// special syntax of math function calls to define custom functions that take
/// 2D data.
///
/// Content in cells that are in the same row can be aligned with the `&` symbol.
///
/// # Example
/// ```example
/// $ mat(
///   1, 2, ..., 10;
///   2, 2, ..., 10;
///   dots.v, dots.v, dots.down, dots.v;
///   10, 10, ..., 10;
/// ) $
/// ```
#[elem(title = "Matrix", LayoutMath)]
pub struct MatElem {
    /// The delimiter to use.
    ///
    /// ```example
    /// #set math.mat(delim: "[")
    /// $ mat(1, 2; 3, 4) $
    /// ```
    #[default(Some(Delimiter::Paren))]
    pub delim: Option<Delimiter>,

    /// Draws augmentation lines in a matrix.
    ///
    /// - `{none}`: No lines are drawn.
    /// - A single number: A vertical augmentation line is drawn
    ///   after the specified column number.
    /// - A dictionary: With a dictionary, multiple augmentation lines can be
    ///   drawn both horizontally and vertically. Additionally, the style of the
    ///   lines can be set. The dictionary can contain the following keys:
    ///   - `hline`: The offsets at which horizontal lines should be drawn.
    ///     For example, an offset of `2` would result in a horizontal line
    ///     being drawn after the second row of the matrix. Accepts either an
    ///     integer for a single line, or an array of integers
    ///     for multiple lines.
    ///   - `vline`: The offsets at which vertical lines should be drawn.
    ///     For example, an offset of `2` would result in a vertical line being
    ///     drawn after the second column of the matrix. Accepts either an
    ///     integer for a single line, or an array of integers
    ///     for multiple lines.
    ///   - `stroke`: How to [stroke]($stroke) the line. If set to `{auto}`,
    ///     takes on a thickness of 0.05em and square line caps.
    ///
    /// ```example
    /// $ mat(1, 0, 1; 0, 1, 2; augment: #2) $
    /// ```
    ///
    /// ```example
    /// $ mat(0, 0, 0; 1, 1, 1; augment: #(hline: 1, stroke: 2pt + green)) $
    /// ```
    #[resolve]
    #[fold]
    pub augment: Option<Augment>,

    /// The gap between rows and columns.
    ///
    /// ```example
    /// #set math.mat(gap: 1em)
    /// $ mat(1, 2; 3, 4) $
    /// ```
    #[external]
    pub gap: Rel<Length>,

    /// The gap between rows. Takes precedence over `gap`.
    ///
    /// ```example
    /// #set math.mat(row-gap: 1em)
    /// $ mat(1, 2; 3, 4) $
    /// ```
    #[resolve]
    #[parse(
        let gap = args.named("gap")?;
        args.named("row-gap")?.or(gap)
    )]
    #[default(DEFAULT_ROW_GAP.into())]
    pub row_gap: Rel<Length>,

    /// The gap between columns. Takes precedence over `gap`.
    ///
    /// ```example
    /// #set math.mat(column-gap: 1em)
    /// $ mat(1, 2; 3, 4) $
    /// ```
    #[resolve]
    #[parse(args.named("column-gap")?.or(gap))]
    #[default(DEFAULT_COL_GAP.into())]
    pub column_gap: Rel<Length>,

    /// An array of arrays with the rows of the matrix.
    ///
    /// ```example
    /// #let data = ((1, 2, 3), (4, 5, 6))
    /// #let matrix = math.mat(..data)
    /// $ v := matrix $
    /// ```
    #[variadic]
    #[parse(
        let mut rows = vec![];
        let mut width = 0;

        let values = args.all::<Spanned<Value>>()?;
        if values.iter().any(|spanned| matches!(spanned.v, Value::Array(_))) {
            for Spanned { v, span } in values {
                let array = v.cast::<Array>().at(span)?;
                let row: Vec<_> = array.into_iter().map(Value::display).collect();
                width = width.max(row.len());
                rows.push(row);
            }
        } else {
            rows = vec![values.into_iter().map(|spanned| spanned.v.display()).collect()];
        }

        for row in &mut rows {
            if row.len() < width {
                row.resize(width, Content::empty());
            }
        }

        rows
    )]
    pub rows: Vec<Vec<Content>>,
}

impl LayoutMath for MatElem {
    #[tracing::instrument(skip(ctx))]
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        // validate inputs

        let augment = self.augment(ctx.styles());

        if let Some(aug) = &augment {
            for &offset in &aug.hline.0 {
                if offset == 0 || offset >= self.rows().len() {
                    bail!(
                        self.span(),
                        "cannot draw a horizontal line after row {} of a matrix with {} rows",
                        offset,
                        self.rows().len()
                    );
                }
            }

            let ncols = self.rows().first().map_or(0, |row| row.len());

            for &offset in &aug.vline.0 {
                if offset == 0 || offset >= ncols {
                    bail!(
                        self.span(),
                        "cannot draw a vertical line after column {} of a matrix with {} columns",
                        offset,
                        ncols
                    );
                }
            }
        }

        let delim = self.delim(ctx.styles());
        let frame = layout_mat_body(
            ctx,
            &self.rows(),
            augment,
            Axes::new(self.column_gap(ctx.styles()), self.row_gap(ctx.styles())),
            self.span(),
        )?;

        layout_delimiters(
            ctx,
            frame,
            delim.map(Delimiter::open),
            delim.map(Delimiter::close),
            self.span(),
        )
    }
}

/// A case distinction.
///
/// Content across different branches can be aligned with the `&` symbol.
///
/// # Example
/// ```example
/// $ f(x, y) := cases(
///   1 "if" (x dot y)/2 <= 0,
///   2 "if" x "is even",
///   3 "if" x in NN,
///   4 "else",
/// ) $
/// ```
#[elem(LayoutMath)]
pub struct CasesElem {
    /// The delimiter to use.
    ///
    /// ```example
    /// #set math.cases(delim: "[")
    /// $ x = cases(1, 2) $
    /// ```
    #[default(Delimiter::Brace)]
    pub delim: Delimiter,

    /// The gap between branches.
    ///
    /// ```example
    /// #set math.cases(gap: 1em)
    /// $ x = cases(1, 2) $
    /// ```
    #[resolve]
    #[default(DEFAULT_ROW_GAP.into())]
    pub gap: Rel<Length>,

    /// The branches of the case distinction.
    #[variadic]
    pub children: Vec<Content>,
}

impl LayoutMath for CasesElem {
    #[tracing::instrument(skip(ctx))]
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        let delim = self.delim(ctx.styles());
        let frame = layout_vec_body(
            ctx,
            &self.children(),
            FixedAlign::Start,
            self.gap(ctx.styles()),
        )?;
        layout_delimiters(ctx, frame, Some(delim.open()), None, self.span())
    }
}

/// A vector / matrix delimiter.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum Delimiter {
    /// Delimit with parentheses.
    #[string("(")]
    Paren,
    /// Delimit with brackets.
    #[string("[")]
    Bracket,
    /// Delimit with curly braces.
    #[string("{")]
    Brace,
    /// Delimit with vertical bars.
    #[string("|")]
    Bar,
    /// Delimit with double vertical bars.
    #[string("||")]
    DoubleBar,
}

impl Delimiter {
    /// The delimiter's opening character.
    fn open(self) -> char {
        match self {
            Self::Paren => '(',
            Self::Bracket => '[',
            Self::Brace => '{',
            Self::Bar => '|',
            Self::DoubleBar => '‖',
        }
    }

    /// The delimiter's closing character.
    fn close(self) -> char {
        match self {
            Self::Paren => ')',
            Self::Bracket => ']',
            Self::Brace => '}',
            Self::Bar => '|',
            Self::DoubleBar => '‖',
        }
    }
}

/// Layout the inner contents of a vector.
fn layout_vec_body(
    ctx: &mut MathContext,
    column: &[Content],
    align: FixedAlign,
    row_gap: Rel<Abs>,
) -> SourceResult<Frame> {
    let gap = row_gap.relative_to(ctx.regions.base().y);
    ctx.style(ctx.style.for_denominator());
    let mut flat = vec![];
    for child in column {
        flat.push(ctx.layout_row(child)?);
    }
    ctx.unstyle();
    Ok(stack(ctx, flat, align, gap, 0))
}

/// Layout the inner contents of a matrix.
fn layout_mat_body(
    ctx: &mut MathContext,
    rows: &[Vec<Content>],
    augment: Option<Augment<Abs>>,
    gap: Axes<Rel<Abs>>,
    span: Span,
) -> SourceResult<Frame> {
    let gap = gap.zip_map(ctx.regions.base(), Rel::relative_to);
    let half_gap = gap * 0.5;

    // We provide a default stroke thickness that scales
    // with font size to ensure that augmentation lines
    // look correct by default at all matrix sizes.
    // The line cap is also set to square because it looks more "correct".
    let default_stroke_thickness = DEFAULT_STROKE_THICKNESS.scaled(ctx);
    let default_stroke = FixedStroke {
        thickness: default_stroke_thickness,
        paint: TextElem::fill_in(ctx.styles()),
        line_cap: LineCap::Square,
        ..Default::default()
    };

    let (hline, vline, stroke) = match augment {
        Some(v) => {
            // need to get stroke here for ownership
            let stroke = v.stroke_or(default_stroke);

            (v.hline, v.vline, stroke)
        }
        _ => (Offsets::default(), Offsets::default(), default_stroke),
    };

    let ncols = rows.first().map_or(0, |row| row.len());
    let nrows = rows.len();
    if ncols == 0 || nrows == 0 {
        return Ok(Frame::soft(Size::zero()));
    }

    // Before the full matrix body can be laid out, the
    // individual cells must first be independently laid out
    // so we can ensure alignment across rows and columns.

    // This variable stores the maximum ascent and descent for each row.
    let mut heights = vec![(Abs::zero(), Abs::zero()); nrows];

    // We want to transpose our data layout to columns
    // before final layout. For efficiency, the columns
    // variable is set up here and newly generated
    // individual cells are then added to it.
    let mut cols = vec![vec![]; ncols];

    ctx.style(ctx.style.for_denominator());
    for (row, (ascent, descent)) in rows.iter().zip(&mut heights) {
        for (cell, col) in row.iter().zip(&mut cols) {
            let cell = ctx.layout_row(cell)?;

            ascent.set_max(cell.ascent());
            descent.set_max(cell.descent());

            col.push(cell);
        }
    }
    ctx.unstyle();

    // For each row, combine maximum ascent and descent into a row height.
    // Sum the row heights, then add the total height of the gaps between rows.
    let total_height =
        heights.iter().map(|&(a, b)| a + b).sum::<Abs>() + gap.y * (nrows - 1) as f64;

    // Width starts at zero because it can't be calculated until later
    let mut frame = Frame::soft(Size::new(Abs::zero(), total_height));

    let mut x = Abs::zero();

    for (index, col) in cols.into_iter().enumerate() {
        let AlignmentResult { points, width: rcol } = alignments(&col);

        let mut y = Abs::zero();

        for (cell, &(ascent, descent)) in col.into_iter().zip(&heights) {
            let cell = cell.into_aligned_frame(ctx, &points, FixedAlign::Center);
            let pos = Point::new(
                if points.is_empty() { x + (rcol - cell.width()) / 2.0 } else { x },
                y + ascent - cell.ascent(),
            );

            frame.push_frame(pos, cell);

            y += ascent + descent + gap.y;
        }

        // Advance to the end of the column
        x += rcol;

        // If a vertical line should be inserted after this column
        if vline.0.contains(&(index + 1)) {
            frame.push(
                Point::with_x(x + half_gap.x),
                line_item(total_height, true, stroke.clone(), span),
            );
        }

        // Advance to the start of the next column
        x += gap.x;
    }

    // Once all the columns are laid out, the total width can be calculated
    let total_width = x - gap.x;

    // This allows the horizontal lines to be laid out
    for line in hline.0 {
        let offset = (heights[0..line].iter().map(|&(a, b)| a + b).sum::<Abs>()
            + gap.y * (line - 1) as f64)
            + half_gap.y;

        frame.push(
            Point::with_y(offset),
            line_item(total_width, false, stroke.clone(), span),
        );
    }

    frame.size_mut().x = total_width;

    Ok(frame)
}

fn line_item(length: Abs, vertical: bool, stroke: FixedStroke, span: Span) -> FrameItem {
    let line_geom = if vertical {
        Geometry::Line(Point::with_y(length))
    } else {
        Geometry::Line(Point::with_x(length))
    };

    FrameItem::Shape(
        Shape {
            geometry: line_geom,
            fill: None,
            stroke: Some(stroke),
        },
        span,
    )
}

/// Layout the outer wrapper around the body of a vector or matrix.
fn layout_delimiters(
    ctx: &mut MathContext,
    mut frame: Frame,
    left: Option<char>,
    right: Option<char>,
    span: Span,
) -> SourceResult<()> {
    let axis = scaled!(ctx, axis_height);
    let short_fall = DELIM_SHORT_FALL.scaled(ctx);
    let height = frame.height();
    let target = height + VERTICAL_PADDING.of(height);
    frame.set_baseline(height / 2.0 + axis);

    if let Some(left) = left {
        let mut left =
            GlyphFragment::new(ctx, left, span).stretch_vertical(ctx, target, short_fall);
        left.center_on_axis(ctx);
        ctx.push(left);
    }

    ctx.push(FrameFragment::new(ctx, frame));

    if let Some(right) = right {
        let mut right = GlyphFragment::new(ctx, right, span)
            .stretch_vertical(ctx, target, short_fall);
        right.center_on_axis(ctx);
        ctx.push(right);
    }

    Ok(())
}

/// Parameters specifying how augmentation lines
/// should be drawn on a matrix.
#[derive(Default, Clone, Hash)]
pub struct Augment<T: Numeric = Length> {
    pub hline: Offsets,
    pub vline: Offsets,
    pub stroke: Smart<Stroke<T>>,
}

impl Augment<Abs> {
    fn stroke_or(&self, fallback: FixedStroke) -> FixedStroke {
        match &self.stroke {
            Smart::Custom(v) => v.clone().unwrap_or(fallback),
            _ => fallback,
        }
    }
}

impl Resolve for Augment {
    type Output = Augment<Abs>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        Augment {
            hline: self.hline,
            vline: self.vline,
            stroke: self.stroke.resolve(styles),
        }
    }
}

impl Fold for Augment<Abs> {
    type Output = Augment<Abs>;

    fn fold(mut self, outer: Self::Output) -> Self::Output {
        self.stroke = self.stroke.fold(outer.stroke);
        self
    }
}

cast! {
    Augment,
    self => {
        let stroke = self.stroke.unwrap_or_default();

        let d = dict! {
            "hline" => self.hline.into_value(),
            "vline" => self.vline.into_value(),
            "stroke" => stroke.into_value()
        };

        d.into_value()
    },
    v: usize => Augment {
        hline: Offsets::default(),
        vline: Offsets(vec![v]),
        stroke: Smart::Auto,
    },
    mut dict: Dict => {
        // need the transpose for the defaults to work
        let hline = dict.take("hline").ok().map(Offsets::from_value)
            .transpose().unwrap_or_default().unwrap_or_default();
        let vline = dict.take("vline").ok().map(Offsets::from_value)
            .transpose().unwrap_or_default().unwrap_or_default();

        let stroke = dict.take("stroke").ok().map(Stroke::from_value)
            .transpose()?.map(Smart::Custom).unwrap_or(Smart::Auto);

        Augment { hline, vline, stroke }
    },
}

cast! {
    Augment<Abs>,
    self => self.into_value(),
}

/// The offsets at which augmentation lines
/// should be drawn on a matrix.
#[derive(Debug, Default, Clone, Hash)]
pub struct Offsets(Vec<usize>);

cast! {
    Offsets,
    self => self.0.into_value(),
    v: usize => Self(vec![v]),
    v: Array => Self(v.into_iter().map(Value::cast).collect::<StrResult<_>>()?),
}
