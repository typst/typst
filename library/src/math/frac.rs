use super::*;

const FRAC_AROUND: Em = Em::new(0.1);

/// # Fraction
/// A mathematical fraction.
///
/// ## Syntax
/// This function also has dedicated syntax: Use a slash to turn neighbouring
/// expressions into a fraction. Multiple atoms can be grouped into a single
/// expression using round grouping parenthesis. Such parentheses are removed
/// from the output, but you can nest multiple to force them.
///
/// ## Example
/// ```
/// $ 1/2 < (x+1)/2 $
/// $ ((x+1)) / 2 = frac(a, b) $
/// ```
///
/// ## Parameters
/// - num: Content (positional, required)
///   The fraction's numerator.
///
/// - denom: Content (positional, required)
///   The fraction's denominator.
///
/// ## Category
/// math
#[func]
#[capable(LayoutMath)]
#[derive(Debug, Hash)]
pub struct FracNode {
    /// The numerator.
    pub num: Content,
    /// The denominator.
    pub denom: Content,
}

#[node]
impl FracNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let num = args.expect("numerator")?;
        let denom = args.expect("denominator")?;
        Ok(Self { num, denom }.pack())
    }
}

impl LayoutMath for FracNode {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        layout(ctx, &self.num, &self.denom, false)
    }
}

/// # Binomial
/// A binomial expression.
///
/// ## Example
/// ```
/// $ binom(n, k) $
/// ```
///
/// ## Parameters
/// - upper: Content (positional, required)
///   The binomial's upper index.
///
/// - lower: Content (positional, required)
///   The binomial's lower index.
///
/// ## Category
/// math
#[func]
#[capable(LayoutMath)]
#[derive(Debug, Hash)]
pub struct BinomNode {
    /// The upper index.
    pub upper: Content,
    /// The lower index.
    pub lower: Content,
}

#[node]
impl BinomNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let upper = args.expect("upper index")?;
        let lower = args.expect("lower index")?;
        Ok(Self { upper, lower }.pack())
    }
}

impl LayoutMath for BinomNode {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        layout(ctx, &self.upper, &self.lower, true)
    }
}

/// Layout a fraction or binomial.
fn layout(
    ctx: &mut MathContext,
    num: &Content,
    denom: &Content,
    binom: bool,
) -> SourceResult<()> {
    let short_fall = DELIM_SHORT_FALL.scaled(ctx);
    let axis = scaled!(ctx, axis_height);
    let thickness = scaled!(ctx, fraction_rule_thickness);
    let shift_up = scaled!(
        ctx,
        text: fraction_numerator_shift_up,
        display: fraction_numerator_display_style_shift_up,
    );
    let shift_down = scaled!(
        ctx,
        text: fraction_denominator_shift_down,
        display: fraction_denominator_display_style_shift_down,
    );
    let num_min = scaled!(
        ctx,
        text: fraction_numerator_gap_min,
        display: fraction_num_display_style_gap_min,
    );
    let denom_min = scaled!(
        ctx,
        text: fraction_denominator_gap_min,
        display: fraction_denom_display_style_gap_min,
    );

    ctx.style(ctx.style.for_numerator());
    let num = ctx.layout_frame(num)?;
    ctx.unstyle();

    ctx.style(ctx.style.for_denominator());
    let denom = ctx.layout_frame(denom)?;
    ctx.unstyle();

    let around = FRAC_AROUND.scaled(ctx);
    let num_gap = (shift_up - axis - num.descent()).max(num_min + thickness / 2.0);
    let denom_gap = (shift_down + axis - denom.ascent()).max(denom_min + thickness / 2.0);

    let line_width = num.width().max(denom.width());
    let width = line_width + 2.0 * around;
    let height = num.height() + num_gap + thickness + denom_gap + denom.height();
    let size = Size::new(width, height);
    let num_pos = Point::with_x((width - num.width()) / 2.0);
    let line_pos =
        Point::new((width - line_width) / 2.0, num.height() + num_gap + thickness / 2.0);
    let denom_pos = Point::new((width - denom.width()) / 2.0, height - denom.height());
    let baseline = line_pos.y + axis;

    let mut frame = Frame::new(size);
    frame.set_baseline(baseline);
    frame.push_frame(num_pos, num);
    frame.push_frame(denom_pos, denom);

    if binom {
        ctx.push(GlyphFragment::new(ctx, '(').stretch_vertical(ctx, height, short_fall));
        ctx.push(frame);
        ctx.push(GlyphFragment::new(ctx, ')').stretch_vertical(ctx, height, short_fall));
    } else {
        frame.push(
            line_pos,
            Element::Shape(
                Geometry::Line(Point::with_x(line_width)).stroked(Stroke {
                    paint: ctx.styles().get(TextNode::FILL),
                    thickness,
                }),
            ),
        );
        ctx.push(frame);
    }

    Ok(())
}
