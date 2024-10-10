use crate::diag::{bail, SourceResult};
use crate::foundations::{elem, Content, Packed, StyleChain, Value};
use crate::layout::{Em, Frame, FrameItem, Point, Size};
use crate::math::{
    scaled_font_size, style_for_denominator, style_for_numerator, FrameFragment,
    GlyphFragment, LayoutMath, MathContext, Scaled, DELIM_SHORT_FALL,
};
use crate::syntax::{Span, Spanned};
use crate::text::TextElem;
use crate::visualize::{FixedStroke, Geometry};

const FRAC_AROUND: Em = Em::new(0.1);

/// A mathematical fraction.
///
/// # Example
/// ```example
/// $ 1/2 < (x+1)/2 $
/// $ ((x+1)) / 2 = frac(a, b) $
/// ```
///
/// # Syntax
/// This function also has dedicated syntax: Use a slash to turn neighbouring
/// expressions into a fraction. Multiple atoms can be grouped into a single
/// expression using round grouping parenthesis. Such parentheses are removed
/// from the output, but you can nest multiple to force them.
#[elem(title = "Fraction", LayoutMath)]
pub struct FracElem {
    /// The fraction's numerator.
    #[required]
    pub num: Content,

    /// The fraction's denominator.
    #[required]
    pub denom: Content,
}

impl LayoutMath for Packed<FracElem> {
    #[typst_macros::time(name = "math.frac", span = self.span())]
    fn layout_math(&self, ctx: &mut MathContext, styles: StyleChain) -> SourceResult<()> {
        layout(
            ctx,
            styles,
            self.num(),
            std::slice::from_ref(self.denom()),
            false,
            self.span(),
        )
    }
}

/// A binomial expression.
///
/// # Example
/// ```example
/// $ binom(n, k) $
/// $ binom(n, k_1, k_2, k_3, ..., k_m) $
/// ```
#[elem(title = "Binomial", LayoutMath)]
pub struct BinomElem {
    /// The binomial's upper index.
    #[required]
    pub upper: Content,

    /// The binomial's lower index.
    #[required]
    #[variadic]
    #[parse(
        let values = args.all::<Spanned<Value>>()?;
        if values.is_empty() {
            // Prevents one element binomials
            bail!(args.span, "missing argument: lower");
        }
        values.into_iter().map(|spanned| spanned.v.display()).collect()
    )]
    pub lower: Vec<Content>,
}

impl LayoutMath for Packed<BinomElem> {
    #[typst_macros::time(name = "math.binom", span = self.span())]
    fn layout_math(&self, ctx: &mut MathContext, styles: StyleChain) -> SourceResult<()> {
        layout(ctx, styles, self.upper(), self.lower(), true, self.span())
    }
}

/// Layout a fraction or binomial.
fn layout(
    ctx: &mut MathContext,
    styles: StyleChain,
    num: &Content,
    denom: &[Content],
    binom: bool,
    span: Span,
) -> SourceResult<()> {
    let font_size = scaled_font_size(ctx, styles);
    let short_fall = DELIM_SHORT_FALL.at(font_size);
    let axis = scaled!(ctx, styles, axis_height);
    let thickness = scaled!(ctx, styles, fraction_rule_thickness);
    let shift_up = scaled!(
        ctx, styles,
        text: fraction_numerator_shift_up,
        display: fraction_numerator_display_style_shift_up,
    );
    let shift_down = scaled!(
        ctx, styles,
        text: fraction_denominator_shift_down,
        display: fraction_denominator_display_style_shift_down,
    );
    let num_min = scaled!(
        ctx, styles,
        text: fraction_numerator_gap_min,
        display: fraction_num_display_style_gap_min,
    );
    let denom_min = scaled!(
        ctx, styles,
        text: fraction_denominator_gap_min,
        display: fraction_denom_display_style_gap_min,
    );

    let num_style = style_for_numerator(styles);
    let num = ctx.layout_into_frame(num, styles.chain(&num_style))?;

    let denom_style = style_for_denominator(styles);
    let denom = ctx.layout_into_frame(
        &Content::sequence(
            // Add a comma between each element.
            denom.iter().flat_map(|a| [TextElem::packed(','), a.clone()]).skip(1),
        ),
        styles.chain(&denom_style),
    )?;

    let around = FRAC_AROUND.at(font_size);
    let num_gap = (shift_up - (axis + thickness / 2.0) - num.descent()).max(num_min);
    let denom_gap =
        (shift_down + (axis - thickness / 2.0) - denom.ascent()).max(denom_min);

    let line_width = num.width().max(denom.width());
    let width = line_width + 2.0 * around;
    let height = num.height() + num_gap + thickness + denom_gap + denom.height();
    let size = Size::new(width, height);
    let num_pos = Point::with_x((width - num.width()) / 2.0);
    let line_pos =
        Point::new((width - line_width) / 2.0, num.height() + num_gap + thickness / 2.0);
    let denom_pos = Point::new((width - denom.width()) / 2.0, height - denom.height());
    let baseline = line_pos.y + axis;

    let mut frame = Frame::soft(size);
    frame.set_baseline(baseline);
    frame.push_frame(num_pos, num);
    frame.push_frame(denom_pos, denom);

    if binom {
        let mut left = GlyphFragment::new(ctx, styles, '(', span)
            .stretch_vertical(ctx, height, short_fall);
        left.center_on_axis(ctx);
        ctx.push(left);
        ctx.push(FrameFragment::new(ctx, styles, frame));
        let mut right = GlyphFragment::new(ctx, styles, ')', span)
            .stretch_vertical(ctx, height, short_fall);
        right.center_on_axis(ctx);
        ctx.push(right);
    } else {
        frame.push(
            line_pos,
            FrameItem::Shape(
                Geometry::Line(Point::with_x(line_width)).stroked(
                    FixedStroke::from_pair(
                        TextElem::fill_in(styles).as_decoration(),
                        thickness,
                    ),
                ),
                span,
            ),
        );
        ctx.push(FrameFragment::new(ctx, styles, frame));
    }

    Ok(())
}
