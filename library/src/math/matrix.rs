use super::*;

const ROW_GAP: Em = Em::new(0.5);
const VERTICAL_PADDING: Ratio = Ratio::new(0.1);

/// # Vector
/// A column vector.
///
/// _Note:_ Matrices are not yet supported.
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
    /// #set vec(delim: "[")
    /// $ vec(1, 2) $
    /// ```
    pub const DELIM: Delimiter = Delimiter::Paren;

    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.all()?).pack())
    }

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "elements" => {
                Some(Value::Array(self.0.iter().cloned().map(Value::Content).collect()))
            }
            _ => None,
        }
    }
}

impl LayoutMath for VecNode {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        let delim = ctx.outer.get(Self::DELIM);
        layout(ctx, &self.0, Align::Center, Some(delim.open()), Some(delim.close()))
    }
}

/// # Cases
/// A case distinction.
///
/// ## Example
/// ```
/// $ f(x, y) := cases(
///   1 "if" (x dot y)/2 <= 0,
///   2 "if" x in NN,
///   3 "if" x "is even",
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

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "branches" => {
                Some(Value::Array(self.0.iter().cloned().map(Value::Content).collect()))
            }
            _ => None,
        }
    }
}

impl LayoutMath for CasesNode {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        layout(ctx, &self.0, Align::Left, Some('{'), None)
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
    /// Delimit the vector with parentheses.
    "(" => Self::Paren,
    /// Delimit the vector with brackets.
    "[" => Self::Bracket,
    /// Delimit the vector with curly braces.
    "{" => Self::Brace,
    /// Delimit the vector with vertical bars.
    "|" => Self::Bar,
    /// Delimit the vector with double vertical bars.
    "||" => Self::DoubleBar,
}

/// Layout a matrix.
fn layout(
    ctx: &mut MathContext,
    elements: &[Content],
    align: Align,
    left: Option<char>,
    right: Option<char>,
) -> SourceResult<()> {
    let axis = scaled!(ctx, axis_height);
    let gap = ROW_GAP.scaled(ctx);

    ctx.style(ctx.style.for_denominator());
    let mut rows = vec![];
    for element in elements {
        rows.push(ctx.layout_row(element)?);
    }
    ctx.unstyle();

    if let Some(left) = left {
        ctx.push(GlyphFragment::new(ctx, left));
    }

    let mut frame = stack(ctx, rows, align, gap, 0);
    frame.set_baseline(frame.height() / 2.0 + axis);

    ctx.push(frame);

    if let Some(right) = right {
        ctx.push(GlyphFragment::new(ctx, right));
    }

    Ok(())
}

/// Stack rows on top of each other.
///
/// Add a `gap` between each row and uses the baseline of the `baseline`th
/// row for the whole frame.
pub(super) fn stack(
    ctx: &MathContext,
    rows: Vec<MathRow>,
    align: Align,
    gap: Abs,
    baseline: usize,
) -> Frame {
    let mut width = Abs::zero();
    let mut height = rows.len().saturating_sub(1) as f64 * gap;

    let points = alignments(&rows);
    let rows: Vec<_> =
        rows.into_iter().map(|row| row.to_line_frame(ctx, &points)).collect();

    for row in &rows {
        height += row.height();
        width.set_max(row.width());
    }

    let extra = VERTICAL_PADDING.of(height);
    height += extra;

    let mut y = extra / 2.0;
    let mut frame = Frame::new(Size::new(width, height));

    for (i, row) in rows.into_iter().enumerate() {
        let x = align.position(width - row.width());
        let pos = Point::new(x, y);
        if i == baseline {
            frame.set_baseline(y + row.baseline());
        }
        y += row.height() + gap;
        frame.push_frame(pos, row);
    }

    frame
}
