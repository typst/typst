use super::*;

/// A square root.
///
/// ## Example
/// ```example
/// $ sqrt(x^2) = x = sqrt(x)^2 $
/// ```
///
/// Display: Square Root
/// Category: math
#[node(LayoutMath)]
pub struct SqrtNode {
    /// The expression to take the square root of.
    #[required]
    pub radicand: Content,
}

impl LayoutMath for SqrtNode {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        layout(ctx, None, &self.radicand(), self.span())
    }
}

/// A general root.
///
/// ## Example
/// ```example
/// $ root(3, x) $
/// ```
///
/// Display: Root
/// Category: math
#[node(LayoutMath)]
pub struct RootNode {
    /// Which root of the radicand to take.
    #[required]
    index: Content,

    /// The expression to take the root of.
    #[required]
    radicand: Content,
}

impl LayoutMath for RootNode {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        layout(ctx, Some(&self.index()), &self.radicand(), self.span())
    }
}

/// Layout a root.
///
/// https://www.w3.org/TR/mathml-core/#radicals-msqrt-mroot
fn layout(
    ctx: &mut MathContext,
    mut index: Option<&Content>,
    radicand: &Content,
    span: Span,
) -> SourceResult<()> {
    let gap = scaled!(
        ctx,
        text: radical_vertical_gap,
        display: radical_display_style_vertical_gap,
    );
    let thickness = scaled!(ctx, radical_rule_thickness);
    let extra_ascender = scaled!(ctx, radical_extra_ascender);
    let kern_before = scaled!(ctx, radical_kern_before_degree);
    let kern_after = scaled!(ctx, radical_kern_after_degree);
    let raise_factor = percent!(ctx, radical_degree_bottom_raise_percent);

    // Layout radicand.
    ctx.style(ctx.style.with_cramped(true));
    let radicand = ctx.layout_frame(radicand)?;
    ctx.unstyle();

    // Layout root symbol.
    let target = radicand.height() + thickness + gap;
    let sqrt = precomposed(ctx, index, target)
        .map(|frame| {
            index = None;
            frame
        })
        .unwrap_or_else(|| {
            let glyph = GlyphFragment::new(ctx, '√', span);
            glyph.stretch_vertical(ctx, target, Abs::zero()).frame
        });

    // Layout the index.
    // Script-script style looks too small, we use Script style instead.
    ctx.style(ctx.style.with_size(MathSize::Script));
    let index = index.map(|node| ctx.layout_frame(node)).transpose()?;
    ctx.unstyle();

    let gap = gap.max((sqrt.height() - radicand.height() - thickness) / 2.0);
    let descent = radicand.descent() + gap;
    let inner_ascent = extra_ascender + thickness + gap + radicand.ascent();

    let mut sqrt_offset = Abs::zero();
    let mut shift_up = Abs::zero();
    let mut ascent = inner_ascent;

    if let Some(index) = &index {
        sqrt_offset = kern_before + index.width() + kern_after;
        shift_up = raise_factor * sqrt.height() - descent + index.descent();
        ascent.set_max(shift_up + index.ascent());
    }

    let radicant_offset = sqrt_offset + sqrt.width();
    let width = radicant_offset + radicand.width();
    let size = Size::new(width, ascent + descent);

    let sqrt_pos = Point::new(sqrt_offset, ascent - inner_ascent);
    let line_pos = Point::new(radicant_offset, ascent - inner_ascent + thickness / 2.0);
    let radicand_pos = Point::new(radicant_offset, ascent - radicand.ascent());

    let mut frame = Frame::new(size);
    frame.set_baseline(ascent);

    if let Some(index) = index {
        let index_pos = Point::new(kern_before, ascent - shift_up - index.ascent());
        frame.push_frame(index_pos, index);
    }

    frame.push_frame(sqrt_pos, sqrt);
    frame.push(
        line_pos,
        Element::Shape(
            Geometry::Line(Point::with_x(radicand.width()))
                .stroked(Stroke { paint: TextNode::fill_in(ctx.styles()), thickness }),
            span,
        ),
    );

    frame.push_frame(radicand_pos, radicand);
    ctx.push(FrameFragment::new(ctx, frame));

    Ok(())
}

/// Select a precomposed radical, if the font has it.
fn precomposed(ctx: &MathContext, index: Option<&Content>, target: Abs) -> Option<Frame> {
    let node = index?.to::<TextNode>()?;
    let c = match node.text().as_str() {
        "3" => '∛',
        "4" => '∜',
        _ => return None,
    };

    ctx.ttf.glyph_index(c)?;
    let glyph = GlyphFragment::new(ctx, c, node.span());
    let variant = glyph.stretch_vertical(ctx, target, Abs::zero()).frame;
    if variant.height() < target {
        return None;
    }

    Some(variant)
}
