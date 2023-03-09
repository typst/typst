use super::*;

const LINE_GAP: Em = Em::new(0.15);
const BRACE_GAP: Em = Em::new(0.25);
const BRACKET_GAP: Em = Em::new(0.25);

/// A horizontal line under content.
///
/// ## Example
/// ```example
/// $ underline(1 + 2 + ... + 5) $
/// ```
///
/// Display: Underline
/// Category: math
#[node(LayoutMath)]
pub struct UnderlineNode {
    /// The content above the line.
    #[positional]
    #[required]
    pub body: Content,
}

impl LayoutMath for UnderlineNode {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        layout(ctx, &self.body(), &None, '\u{305}', LINE_GAP, false)
    }
}

/// A horizontal line over content.
///
/// ## Example
/// ```example
/// $ overline(1 + 2 + ... + 5) $
/// ```
///
/// Display: Overline
/// Category: math
#[node(LayoutMath)]
pub struct OverlineNode {
    /// The content below the line.
    #[positional]
    #[required]
    pub body: Content,
}

impl LayoutMath for OverlineNode {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        layout(ctx, &self.body(), &None, '\u{332}', LINE_GAP, true)
    }
}

/// A horizontal brace under content, with an optional annotation below.
///
/// ## Example
/// ```example
/// $ underbrace(1 + 2 + ... + 5, "numbers") $
/// ```
///
/// Display: Underbrace
/// Category: math
#[node(LayoutMath)]
pub struct UnderbraceNode {
    /// The content above the brace.
    #[positional]
    #[required]
    pub body: Content,

    /// The optional content below the brace.
    #[positional]
    pub annotation: Option<Content>,
}

impl LayoutMath for UnderbraceNode {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        layout(ctx, &self.body(), &self.annotation(ctx.styles()), '⏟', BRACE_GAP, false)
    }
}

/// A horizontal brace over content, with an optional annotation above.
///
/// ## Example
/// ```example
/// $ overbrace(1 + 2 + ... + 5, "numbers") $
/// ```
///
/// Display: Overbrace
/// Category: math
#[node(LayoutMath)]
pub struct OverbraceNode {
    /// The content below the brace.
    #[positional]
    #[required]
    pub body: Content,

    /// The optional content above the brace.
    #[positional]
    pub annotation: Option<Content>,
}

impl LayoutMath for OverbraceNode {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        layout(ctx, &self.body(), &self.annotation(ctx.styles()), '⏞', BRACE_GAP, true)
    }
}

/// A horizontal bracket under content, with an optional annotation below.
///
/// ## Example
/// ```example
/// $ underbracket(1 + 2 + ... + 5, "numbers") $
/// ```
///
/// Display: Underbracket
/// Category: math
#[node(LayoutMath)]
pub struct UnderbracketNode {
    /// The content above the bracket.
    #[positional]
    #[required]
    pub body: Content,

    /// The optional content below the bracket.
    #[positional]
    pub annotation: Option<Content>,
}

impl LayoutMath for UnderbracketNode {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        layout(ctx, &self.body(), &self.annotation(ctx.styles()), '⎵', BRACKET_GAP, false)
    }
}

/// A horizontal bracket over content, with an optional annotation above.
///
/// ## Example
/// ```example
/// $ overbracket(1 + 2 + ... + 5, "numbers") $
/// ```
///
/// Display: Overbracket
/// Category: math
#[node(LayoutMath)]
pub struct OverbracketNode {
    /// The content below the bracket.
    #[positional]
    #[required]
    pub body: Content,

    /// The optional content above the bracket.
    #[positional]
    pub annotation: Option<Content>,
}

impl LayoutMath for OverbracketNode {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        layout(ctx, &self.body(), &self.annotation(ctx.styles()), '⎴', BRACKET_GAP, true)
    }
}

/// Layout an over- or underthing.
fn layout(
    ctx: &mut MathContext,
    body: &Content,
    annotation: &Option<Content>,
    c: char,
    gap: Em,
    reverse: bool,
) -> SourceResult<()> {
    let gap = gap.scaled(ctx);
    let body = ctx.layout_row(body)?;
    let glyph = GlyphFragment::new(ctx, c);
    let stretched = glyph.stretch_horizontal(ctx, body.width(), Abs::zero());

    let mut rows = vec![body, stretched.into()];
    ctx.style(if reverse {
        ctx.style.for_subscript()
    } else {
        ctx.style.for_superscript()
    });
    rows.extend(
        annotation
            .as_ref()
            .map(|annotation| ctx.layout_row(annotation))
            .transpose()?,
    );
    ctx.unstyle();

    let mut baseline = 0;
    if reverse {
        rows.reverse();
        baseline = rows.len() - 1;
    }

    let frame = stack(ctx, rows, Align::Center, gap, baseline);
    ctx.push(FrameFragment::new(ctx, frame));

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
    let rows: Vec<_> = rows
        .into_iter()
        .map(|row| row.to_aligned_frame(ctx, &points, align))
        .collect();

    for row in &rows {
        height += row.height();
        width.set_max(row.width());
    }

    let mut y = Abs::zero();
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
