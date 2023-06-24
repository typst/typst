use super::*;

const LINE_GAP: Em = Em::new(0.15);
const BRACE_GAP: Em = Em::new(0.25);
const BRACKET_GAP: Em = Em::new(0.25);

/// A horizontal line under content.
///
/// ## Example { #example }
/// ```example
/// $ underline(1 + 2 + ... + 5) $
/// ```
///
/// Display: Underline
/// Category: math
#[element(LayoutMath)]
pub struct UnderlineElem {
    /// The content above the line.
    #[required]
    pub body: Content,
}

impl LayoutMath for UnderlineElem {
    #[tracing::instrument(skip(ctx))]
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        layout(ctx, &self.body(), &None, '\u{305}', LINE_GAP, false, self.span())
    }
}

/// A horizontal line over content.
///
/// ## Example { #example }
/// ```example
/// $ overline(1 + 2 + ... + 5) $
/// ```
///
/// Display: Overline
/// Category: math
#[element(LayoutMath)]
pub struct OverlineElem {
    /// The content below the line.
    #[required]
    pub body: Content,
}

impl LayoutMath for OverlineElem {
    #[tracing::instrument(skip(ctx))]
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        layout(ctx, &self.body(), &None, '\u{332}', LINE_GAP, true, self.span())
    }
}

/// A horizontal brace under content, with an optional annotation below.
///
/// ## Example { #example }
/// ```example
/// $ underbrace(1 + 2 + ... + 5, "numbers") $
/// ```
///
/// Display: Underbrace
/// Category: math
#[element(LayoutMath)]
pub struct UnderbraceElem {
    /// The content above the brace.
    #[required]
    pub body: Content,

    /// The optional content below the brace.
    #[positional]
    pub annotation: Option<Content>,
}

impl LayoutMath for UnderbraceElem {
    #[tracing::instrument(skip(ctx))]
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        layout(
            ctx,
            &self.body(),
            &self.annotation(ctx.styles()),
            '⏟',
            BRACE_GAP,
            false,
            self.span(),
        )
    }
}

/// A horizontal brace over content, with an optional annotation above.
///
/// ## Example { #example }
/// ```example
/// $ overbrace(1 + 2 + ... + 5, "numbers") $
/// ```
///
/// Display: Overbrace
/// Category: math
#[element(LayoutMath)]
pub struct OverbraceElem {
    /// The content below the brace.
    #[required]
    pub body: Content,

    /// The optional content above the brace.
    #[positional]
    pub annotation: Option<Content>,
}

impl LayoutMath for OverbraceElem {
    #[tracing::instrument(skip(ctx))]
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        layout(
            ctx,
            &self.body(),
            &self.annotation(ctx.styles()),
            '⏞',
            BRACE_GAP,
            true,
            self.span(),
        )
    }
}

/// A horizontal bracket under content, with an optional annotation below.
///
/// ## Example { #example }
/// ```example
/// $ underbracket(1 + 2 + ... + 5, "numbers") $
/// ```
///
/// Display: Underbracket
/// Category: math
#[element(LayoutMath)]
pub struct UnderbracketElem {
    /// The content above the bracket.
    #[required]
    pub body: Content,

    /// The optional content below the bracket.
    #[positional]
    pub annotation: Option<Content>,
}

impl LayoutMath for UnderbracketElem {
    #[tracing::instrument(skip(ctx))]
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        layout(
            ctx,
            &self.body(),
            &self.annotation(ctx.styles()),
            '⎵',
            BRACKET_GAP,
            false,
            self.span(),
        )
    }
}

/// A horizontal bracket over content, with an optional annotation above.
///
/// ## Example { #example }
/// ```example
/// $ overbracket(1 + 2 + ... + 5, "numbers") $
/// ```
///
/// Display: Overbracket
/// Category: math
#[element(LayoutMath)]
pub struct OverbracketElem {
    /// The content below the bracket.
    #[required]
    pub body: Content,

    /// The optional content above the bracket.
    #[positional]
    pub annotation: Option<Content>,
}

impl LayoutMath for OverbracketElem {
    #[tracing::instrument(skip(ctx))]
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        layout(
            ctx,
            &self.body(),
            &self.annotation(ctx.styles()),
            '⎴',
            BRACKET_GAP,
            true,
            self.span(),
        )
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
    span: Span,
) -> SourceResult<()> {
    let gap = gap.scaled(ctx);
    let body = ctx.layout_row(body)?;
    let body_class = body.class();
    let body = body.into_fragment(ctx);
    let glyph = GlyphFragment::new(ctx, c, span);
    let stretched = glyph.stretch_horizontal(ctx, body.width(), Abs::zero());

    let mut rows = vec![MathRow::new(vec![body]), stretched.into()];
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
    ctx.push(FrameFragment::new(ctx, frame).with_class(body_class));

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
    let rows: Vec<_> = rows.into_iter().flat_map(|r| r.rows()).collect();
    let AlignmentResult { points, width } = alignments(&rows);
    let rows: Vec<_> = rows
        .into_iter()
        .map(|row| row.into_aligned_frame(ctx, &points, align))
        .collect();

    let mut y = Abs::zero();
    let mut frame = Frame::new(Size::new(
        width,
        rows.iter().map(|row| row.height()).sum::<Abs>()
            + rows.len().saturating_sub(1) as f64 * gap,
    ));

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
