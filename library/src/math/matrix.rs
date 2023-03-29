use super::*;

const ROW_GAP: Em = Em::new(0.5);
const COL_GAP: Em = Em::new(0.5);
const VERTICAL_PADDING: Ratio = Ratio::new(0.1);

/// A column vector.
///
/// Content in the vector's elements can be aligned with the `&` symbol.
///
/// ## Example
/// ```example
/// $ vec(a, b, c) dot.op vec(1, 2, 3)
///     = a + 2b + 3c $
/// ```
///
/// Display: Vector
/// Category: math
#[element(LayoutMath)]
pub struct VecElem {
    /// The delimiter to use.
    ///
    /// ```example
    /// #set math.vec(delim: "[")
    /// $ vec(1, 2) $
    /// ```
    #[default(Some(Delimiter::Paren))]
    pub delim: Option<Delimiter>,

    /// The elements of the vector.
    #[variadic]
    pub children: Vec<Content>,
}

impl LayoutMath for VecElem {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        let delim = self.delim(ctx.styles());
        let frame = layout_vec_body(ctx, &self.children(), Align::Center)?;
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
/// ## Example
/// ```example
/// $ mat(
///   1, 2, ..., 10;
///   2, 2, ..., 10;
///   dots.v, dots.v, dots.down, dots.v;
///   10, 10, ..., 10;
/// ) $
/// ```
///
/// Display: Matrix
/// Category: math
#[element(LayoutMath)]
pub struct MatElem {
    /// The delimiter to use.
    ///
    /// ```example
    /// #set math.mat(delim: "[")
    /// $ mat(1, 2; 3, 4) $
    /// ```
    #[default(Some(Delimiter::Paren))]
    pub delim: Option<Delimiter>,

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
        if values.iter().all(|spanned| matches!(spanned.v, Value::Content(_))) {
            rows = vec![values.into_iter().map(|spanned| spanned.v.display()).collect()];
        } else {
            for Spanned { v, span } in values {
                let array = v.cast::<Array>().at(span)?;
                let row: Vec<_> = array.into_iter().map(Value::display).collect();
                width = width.max(row.len());
                rows.push(row);
            }
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
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        let delim = self.delim(ctx.styles());
        let frame = layout_mat_body(ctx, &self.rows())?;
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
/// ## Example
/// ```example
/// $ f(x, y) := cases(
///   1 "if" (x dot.op y)/2 <= 0,
///   2 "if" x "is even",
///   3 "if" x in NN,
///   4 "else",
/// ) $
/// ```
///
/// Display: Cases
/// Category: math
#[element(LayoutMath)]
pub struct CasesElem {
    /// The delimiter to use.
    ///
    /// ```example
    /// #set math.cases(delim: "[")
    /// $ x = cases(1, 2) $
    /// ```
    #[default(Delimiter::Brace)]
    pub delim: Delimiter,

    /// The branches of the case distinction.
    #[variadic]
    pub children: Vec<Content>,
}

impl LayoutMath for CasesElem {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        let delim = self.delim(ctx.styles());
        let frame = layout_vec_body(ctx, &self.children(), Align::Left)?;
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
    align: Align,
) -> SourceResult<Frame> {
    let gap = ROW_GAP.scaled(ctx);
    ctx.style(ctx.style.for_denominator());
    let mut flat = vec![];
    for child in column {
        flat.push(ctx.layout_row(child)?);
    }
    ctx.unstyle();
    Ok(stack(ctx, flat, align, gap, 0))
}

/// Layout the inner contents of a matrix.
fn layout_mat_body(ctx: &mut MathContext, rows: &[Vec<Content>]) -> SourceResult<Frame> {
    let row_gap = ROW_GAP.scaled(ctx);
    let col_gap = COL_GAP.scaled(ctx);

    let ncols = rows.first().map_or(0, |row| row.len());
    let nrows = rows.len();
    if ncols == 0 || nrows == 0 {
        return Ok(Frame::new(Size::zero()));
    }

    let mut widths = vec![Abs::zero(); ncols];
    let mut ascents = vec![Abs::zero(); nrows];
    let mut descents = vec![Abs::zero(); nrows];

    ctx.style(ctx.style.for_denominator());
    let mut cols = vec![vec![]; ncols];
    for ((row, ascent), descent) in rows.iter().zip(&mut ascents).zip(&mut descents) {
        for ((cell, rcol), col) in row.iter().zip(&mut widths).zip(&mut cols) {
            let cell = ctx.layout_row(cell)?;
            rcol.set_max(cell.width());
            ascent.set_max(cell.ascent());
            descent.set_max(cell.descent());
            col.push(cell);
        }
    }
    ctx.unstyle();

    let width = widths.iter().sum::<Abs>() + col_gap * (ncols - 1) as f64;
    let height = ascents.iter().sum::<Abs>()
        + descents.iter().sum::<Abs>()
        + row_gap * (nrows - 1) as f64;
    let size = Size::new(width, height);

    let mut frame = Frame::new(size);
    let mut x = Abs::zero();
    for (col, &rcol) in cols.into_iter().zip(&widths) {
        let points = alignments(&col);
        let mut y = Abs::zero();
        for ((cell, &ascent), &descent) in col.into_iter().zip(&ascents).zip(&descents) {
            let cell = cell.to_aligned_frame(ctx, &points, Align::Center);
            let pos =
                Point::new(x + (rcol - cell.width()) / 2.0, y + ascent - cell.ascent());
            frame.push_frame(pos, cell);
            y += ascent + descent + row_gap;
        }
        x += rcol + col_gap;
    }

    Ok(frame)
}

/// Layout the outer wrapper around a vector's or matrices' body.
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
        ctx.push(
            GlyphFragment::new(ctx, left, span).stretch_vertical(ctx, target, short_fall),
        );
    }

    ctx.push(FrameFragment::new(ctx, frame));

    if let Some(right) = right {
        ctx.push(
            GlyphFragment::new(ctx, right, span)
                .stretch_vertical(ctx, target, short_fall),
        );
    }

    Ok(())
}
