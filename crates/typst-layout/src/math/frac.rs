use typst_library::diag::SourceResult;
use typst_library::foundations::{Content, Packed, Resolve, StyleChain, SymbolElem};
use typst_library::layout::{Em, Frame, FrameItem, Point, Size};
use typst_library::math::{BinomElem, FracElem};
use typst_library::text::TextElem;
use typst_library::visualize::{FixedStroke, Geometry};
use typst_syntax::Span;

use super::{
    style_for_denominator, style_for_numerator, FrameFragment, GlyphFragment,
    MathContext, DELIM_SHORT_FALL,
};

const FRAC_AROUND: Em = Em::new(0.1);

/// Lays out a [`FracElem`].
#[typst_macros::time(name = "math.frac", span = elem.span())]
pub fn layout_frac(
    elem: &Packed<FracElem>,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<()> {
    layout_frac_like(
        ctx,
        styles,
        &elem.num,
        std::slice::from_ref(&elem.denom),
        false,
        elem.span(),
    )
}

/// Lays out a [`BinomElem`].
#[typst_macros::time(name = "math.binom", span = elem.span())]
pub fn layout_binom(
    elem: &Packed<BinomElem>,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<()> {
    layout_frac_like(ctx, styles, &elem.upper, &elem.lower, true, elem.span())
}

/// Layout a fraction or binomial.
fn layout_frac_like(
    ctx: &mut MathContext,
    styles: StyleChain,
    num: &Content,
    denom: &[Content],
    binom: bool,
    span: Span,
) -> SourceResult<()> {
    let short_fall = DELIM_SHORT_FALL.resolve(styles);
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
            denom
                .iter()
                .flat_map(|a| [SymbolElem::packed(','), a.clone()])
                .skip(1),
        ),
        styles.chain(&denom_style),
    )?;

    let around = FRAC_AROUND.resolve(styles);
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
        let mut left = GlyphFragment::new(ctx.font, styles, '(', span);
        left.stretch_vertical(ctx, height - short_fall);
        left.center_on_axis();
        ctx.push(left);
        ctx.push(FrameFragment::new(styles, frame));
        let mut right = GlyphFragment::new(ctx.font, styles, ')', span);
        right.stretch_vertical(ctx, height - short_fall);
        right.center_on_axis();
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
        ctx.push(FrameFragment::new(styles, frame));
    }

    Ok(())
}
