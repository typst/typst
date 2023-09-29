use super::*;

const BRACE_GAP: Em = Em::new(0.25);
const BRACKET_GAP: Em = Em::new(0.25);

/// A marker to distinguish under- vs. overlines.
enum LineKind {
    Over,
    Under,
}

/// A horizontal line under content.
///
/// ```example
/// $ underline(1 + 2 + ... + 5) $
/// ```
#[elem(LayoutMath)]
pub struct UnderlineElem {
    /// The content above the line.
    #[required]
    pub body: Content,
}

impl LayoutMath for UnderlineElem {
    #[tracing::instrument(skip(ctx))]
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        layout_underoverline(ctx, &self.body(), self.span(), LineKind::Under)
    }
}

/// A horizontal line over content.
///
/// ```example
/// $ overline(1 + 2 + ... + 5) $
/// ```
#[elem(LayoutMath)]
pub struct OverlineElem {
    /// The content below the line.
    #[required]
    pub body: Content,
}

impl LayoutMath for OverlineElem {
    #[tracing::instrument(skip(ctx))]
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        layout_underoverline(ctx, &self.body(), self.span(), LineKind::Over)
    }
}

/// layout under- or overlined content
fn layout_underoverline(
    ctx: &mut MathContext,
    body: &Content,
    span: Span,
    line: LineKind,
) -> SourceResult<()> {
    let (extra_height, content, line_pos, content_pos, baseline, bar_height);
    match line {
        LineKind::Under => {
            let sep = scaled!(ctx, underbar_extra_descender);
            bar_height = scaled!(ctx, underbar_rule_thickness);
            let gap = scaled!(ctx, underbar_vertical_gap);
            extra_height = sep + bar_height + gap;

            content = ctx.layout_fragment(body)?;

            line_pos = Point::with_y(content.height() + gap + bar_height / 2.0);
            content_pos = Point::zero();
            baseline = content.ascent()
        }
        LineKind::Over => {
            let sep = scaled!(ctx, overbar_extra_ascender);
            bar_height = scaled!(ctx, overbar_rule_thickness);
            let gap = scaled!(ctx, overbar_vertical_gap);
            extra_height = sep + bar_height + gap;

            ctx.style(ctx.style.with_cramped(true));
            content = ctx.layout_fragment(body)?;
            ctx.unstyle();

            line_pos = Point::with_y(sep + bar_height / 2.0);
            content_pos = Point::with_y(extra_height);
            baseline = content.ascent() + extra_height;
        }
    }

    let width = content.width();
    let height = content.height() + extra_height;
    let size = Size::new(width, height);

    let content_class = content.class().unwrap_or(MathClass::Normal);
    let mut frame = Frame::soft(size);
    frame.set_baseline(baseline);
    frame.push_frame(content_pos, content.into_frame());
    frame.push(
        line_pos,
        FrameItem::Shape(
            Geometry::Line(Point::with_x(width)).stroked(FixedStroke {
                paint: TextElem::fill_in(ctx.styles()),
                thickness: bar_height,
                ..FixedStroke::default()
            }),
            span,
        ),
    );

    ctx.push(FrameFragment::new(ctx, frame).with_class(content_class));

    Ok(())
}

/// A horizontal brace under content, with an optional annotation below.
///
/// ```example
/// $ underbrace(1 + 2 + ... + 5, "numbers") $
/// ```
#[elem(LayoutMath)]
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
        layout_underoverspreader(
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
/// ```example
/// $ overbrace(1 + 2 + ... + 5, "numbers") $
/// ```
#[elem(LayoutMath)]
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
        layout_underoverspreader(
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
/// ```example
/// $ underbracket(1 + 2 + ... + 5, "numbers") $
/// ```
#[elem(LayoutMath)]
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
        layout_underoverspreader(
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
/// ```example
/// $ overbracket(1 + 2 + ... + 5, "numbers") $
/// ```
#[elem(LayoutMath)]
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
        layout_underoverspreader(
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

/// Layout an over- or underbrace-like object.
fn layout_underoverspreader(
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

    let frame = stack(ctx, rows, FixedAlign::Center, gap, baseline);
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
    align: FixedAlign,
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
    let mut frame = Frame::soft(Size::new(
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
