use super::*;

/// # Square Root
/// A square root.
///
/// ## Example
/// ```
/// $ sqrt(x^2) = x = sqrt(x)^2 $
/// ```
///
/// ## Parameters
/// - radicand: Content (positional, required)
///   The expression to take the square root of.
///
/// ## Category
/// math
#[func]
#[capable(LayoutMath)]
#[derive(Debug, Hash)]
pub struct SqrtNode(pub Content);

#[node]
impl SqrtNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("radicand")?).pack())
    }
}

impl LayoutMath for SqrtNode {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        layout(ctx, None, &self.0)
    }
}

/// # Root
/// A general root.
///
/// ## Example
/// ```
/// $ radical(3, x) $
/// ```
///
/// ## Parameters
/// - index: Content (positional, required)
///   Which root of the radicand to take.
///
/// - radicand: Content (positional, required)
///   The expression to take the root of.
///
/// ## Category
/// math
#[func]
#[capable(LayoutMath)]
#[derive(Debug, Hash)]
pub struct RootNode {
    index: Content,
    radicand: Content,
}

#[node]
impl RootNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self {
            index: args.expect("index")?,
            radicand: args.expect("radicand")?,
        }
        .pack())
    }
}

impl LayoutMath for RootNode {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        layout(ctx, Some(&self.index), &self.radicand)
    }
}

/// Layout a root.
///
/// https://www.w3.org/TR/mathml-core/#radicals-msqrt-mroot
fn layout(
    ctx: &mut MathContext,
    mut index: Option<&Content>,
    radicand: &Content,
) -> SourceResult<()> {
    let gap = scaled!(
        ctx,
        text: radical_vertical_gap,
        display: radical_display_style_vertical_gap,
    );
    let thickness = scaled!(ctx, radical_rule_thickness);
    let ascender = scaled!(ctx, radical_extra_ascender);
    let kern_before = scaled!(ctx, radical_kern_before_degree);
    let kern_after = scaled!(ctx, radical_kern_after_degree);
    let raise = percent!(ctx, radical_degree_bottom_raise_percent);

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
            let glyph = GlyphFragment::new(ctx, '√');
            glyph.stretch_vertical(ctx, target, Abs::zero()).frame
        });

    // Layout the index.
    let mut offset = Abs::zero();
    let index = if let Some(index) = index {
        // Script-script style looks too small, we use Script style instead.
        ctx.style(ctx.style.with_size(MathSize::Script));
        let frame = ctx.layout_frame(index)?;
        offset = kern_before + frame.width() + kern_after;
        ctx.unstyle();
        frame
    } else {
        Frame::new(Size::zero())
    };

    let width = offset + sqrt.width() + radicand.width();
    let height = sqrt.height() + ascender;
    let size = Size::new(width, height);
    let remains = (sqrt.height() - radicand.height() - thickness) / 2.0;
    let index_pos =
        Point::new(kern_before, height - index.ascent() - raise * sqrt.height());
    let sqrt_pos = Point::new(offset, ascender);
    let line_pos = Point::new(offset + sqrt.width(), ascender + thickness / 2.0);
    let line_length = radicand.width();
    let radicand_pos =
        Point::new(offset + sqrt.width(), ascender + thickness + gap.max(remains));
    let baseline = radicand_pos.y + radicand.ascent();

    let mut frame = Frame::new(size);
    frame.set_baseline(baseline);
    frame.push_frame(index_pos, index);
    frame.push_frame(sqrt_pos, sqrt);
    frame.push(
        line_pos,
        Element::Shape(
            Geometry::Line(Point::with_x(line_length))
                .stroked(Stroke { paint: ctx.fill, thickness }),
        ),
    );
    frame.push_frame(radicand_pos, radicand);
    ctx.push(frame);

    Ok(())
}

/// Select a precomposed radical, if the font has it.
fn precomposed(ctx: &MathContext, index: Option<&Content>, target: Abs) -> Option<Frame> {
    let node = index?.to::<MathNode>()?.body.to::<AtomNode>()?;
    let c = match node.0.as_str() {
        "3" => '∛',
        "4" => '∜',
        _ => return None,
    };

    ctx.ttf.glyph_index(c)?;
    let glyph = GlyphFragment::new(ctx, c);
    let variant = glyph.stretch_vertical(ctx, target, Abs::zero()).frame;
    if variant.height() < target {
        return None;
    }

    Some(variant)
}
