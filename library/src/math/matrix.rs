use super::*;

const ROW_GAP: Em = Em::new(0.5);
const COL_GAP: Em = Em::new(0.75);
const VERTICAL_PADDING: Ratio = Ratio::new(0.1);

/// # Vector
/// A column vector.
///
/// Content in the vector's elements can be aligned with the `&` symbol.
///
/// ## Example
/// ```
/// $ vec(a, b, c) dot vec(1, 2, 3)
///     = a + 2b + 3c $
/// ```
///
/// ## Parameters
/// - elements: Content (positional, variadic)
///   The elements of the vector.
///
/// ## Category
/// math
#[func]
#[capable(LayoutMath)]
#[derive(Debug, Hash)]
pub struct VecNode(Vec<Content>);

#[node]
impl VecNode {
    /// The delimiter to use.
    ///
    /// # Example
    /// ```
    /// #set math.vec(delim: "[")
    /// $ vec(1, 2) $
    /// ```
    pub const DELIM: Delimiter = Delimiter::Paren;

    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.all()?).pack())
    }
}

impl LayoutMath for VecNode {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        let delim = ctx.styles().get(Self::DELIM);
        let frame = layout_vec_body(ctx, &self.0, Align::Center)?;
        layout_delimiters(ctx, frame, Some(delim.open()), Some(delim.close()))
    }
}

/// # Matrix
/// A matrix.
///
/// The elements of a row should be separated by commas, while the rows
/// themselves should be separated by semicolons. The semicolon syntax merges
/// preceding arguments separated by commas into a array arguments. You
/// can also use this special syntax of math function calls to define custom
/// functions that take 2D data.
///
/// Content in cells that are in the same row can be aligned with the `&` symbol.
///
/// ## Example
/// ```
/// $ mat(
///   1, 2, ..., 10;
///   2, 2, ..., 10;
///   dots.v, dots.v, dots.down, dots.v;
///   10, 10, ..., 10;
/// ) $
/// ```
///
/// ## Parameters
/// - rows: Array (positional, variadic)
///   An array of arrays with the rows of the matrix.
///
///   ### Example
///   ```
///   #let data = ((1, 2, 3), (4, 5, 6))
///   #let matrix = math.mat(..data)
///   $ v := matrix $
///   ```
///
/// ## Category
/// math
#[func]
#[capable(LayoutMath)]
#[derive(Debug, Hash)]
pub struct MatNode(Vec<Vec<Content>>);

#[node]
impl MatNode {
    /// The delimiter to use.
    ///
    /// # Example
    /// ```
    /// #set math.mat(delim: "[")
    /// $ mat(1, 2; 3, 4) $
    /// ```
    pub const DELIM: Delimiter = Delimiter::Paren;

    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
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

        Ok(Self(rows).pack())
    }
}

impl LayoutMath for MatNode {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        let delim = ctx.styles().get(Self::DELIM);
        let frame = layout_mat_body(ctx, &self.0)?;
        layout_delimiters(ctx, frame, Some(delim.open()), Some(delim.close()))
    }
}

/// # Cases
/// A case distinction.
///
/// Content across different branches can be aligned with the `&` symbol.
///
/// ## Example
/// ```
/// $ f(x, y) := cases(
///   1 "if" (x dot y)/2 <= 0,
///   2 "if" x "is even",
///   3 "if" x in NN,
///   4 "else",
/// ) $
/// ```
///
/// ## Parameters
/// - branches: Content (positional, variadic)
///   The branches of the case distinction.
///
/// ## Category
/// math
#[func]
#[capable(LayoutMath)]
#[derive(Debug, Hash)]
pub struct CasesNode(Vec<Content>);

#[node]
impl CasesNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.all()?).pack())
    }
}

impl LayoutMath for CasesNode {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        let frame = layout_vec_body(ctx, &self.0, Align::Left)?;
        layout_delimiters(ctx, frame, Some('{'), None)
    }
}

/// A vector / matrix delimiter.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Delimiter {
    Paren,
    Bracket,
    Brace,
    Bar,
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

castable! {
    Delimiter,
    /// Delimit with parentheses.
    "(" => Self::Paren,
    /// Delimit with brackets.
    "[" => Self::Bracket,
    /// Delimit with curly braces.
    "{" => Self::Brace,
    /// Delimit with vertical bars.
    "|" => Self::Bar,
    /// Delimit with double vertical bars.
    "||" => Self::DoubleBar,
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
    for element in column {
        flat.push(ctx.layout_row(element)?);
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

    let mut rcols = vec![Abs::zero(); ncols];
    let mut rrows = vec![Abs::zero(); nrows];

    ctx.style(ctx.style.for_denominator());
    let mut cols = vec![vec![]; ncols];
    for (row, rrow) in rows.iter().zip(&mut rrows) {
        for ((cell, rcol), col) in row.iter().zip(&mut rcols).zip(&mut cols) {
            let cell = ctx.layout_row(cell)?;
            rcol.set_max(cell.width());
            rrow.set_max(cell.height());
            col.push(cell);
        }
    }
    ctx.unstyle();

    let width = rcols.iter().sum::<Abs>() + col_gap * (ncols - 1) as f64;
    let height = rrows.iter().sum::<Abs>() + row_gap * (nrows - 1) as f64;
    let size = Size::new(width, height);

    let mut frame = Frame::new(size);
    let mut x = Abs::zero();
    for (col, &rcol) in cols.into_iter().zip(&rcols) {
        let points = alignments(&col);
        let mut y = Abs::zero();
        for (cell, &rrow) in col.into_iter().zip(&rrows) {
            let cell = cell.to_aligned_frame(ctx, &points, Align::Center);
            let pos = Point::new(
                x + (rcol - cell.width()) / 2.0,
                y + (rrow - cell.height()) / 2.0,
            );
            frame.push_frame(pos, cell);
            y += rrow + row_gap;
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
) -> SourceResult<()> {
    let axis = scaled!(ctx, axis_height);
    let short_fall = DELIM_SHORT_FALL.scaled(ctx);
    let height = frame.height();
    let target = height + VERTICAL_PADDING.of(height);
    frame.set_baseline(height / 2.0 + axis);

    if let Some(left) = left {
        ctx.push(GlyphFragment::new(ctx, left).stretch_vertical(ctx, target, short_fall));
    }

    ctx.push(frame);

    if let Some(right) = right {
        ctx.push(
            GlyphFragment::new(ctx, right).stretch_vertical(ctx, target, short_fall),
        );
    }

    Ok(())
}
