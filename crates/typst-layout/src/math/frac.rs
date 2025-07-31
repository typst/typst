use typst_library::diag::SourceResult;
use typst_library::foundations::{Content, Packed, Resolve, StyleChain, SymbolElem};
use typst_library::layout::{Abs, Em, Frame, FrameItem, Point, Size};
use typst_library::math::{BinomElem, FracElem, FracStyle};
use typst_library::text::TextElem;
use typst_library::visualize::{FixedStroke, Geometry};
use typst_syntax::Span;

use super::{
    DELIM_SHORT_FALL, FrameFragment, GlyphFragment, MathContext, style_for_denominator,
    style_for_numerator,
};

const FRAC_AROUND: Em = Em::new(0.1);
const FRAC_DIAGONAL_SKEW: Em = Em::new(0.3); // shift up and down
const FRAC_DIAGONAL_OFFSET: Em = Em::new(0.1); // how much operands can bite into the slash

/// Lays out a [`FracElem`].
#[typst_macros::time(name = "math.frac", span = elem.span())]
pub fn layout_frac(
    elem: &Packed<FracElem>,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<()> {
    match elem.style.get(styles) {
        Some(FracStyle::Skewed) => {
            layout_skewed_frac(ctx, styles, &elem.num, &elem.denom, elem.span())
        }
        Some(FracStyle::Horizontal) => layout_horizontal_frac(
            ctx,
            styles,
            &elem.num,
            &elem.denom,
            elem.span(),
            elem.num_deparenthesized.unwrap_or(false),
            elem.denom_deparenthesized.unwrap_or(false),
        ),
        _ => layout_vertical_frac_like(
            ctx,
            styles,
            &elem.num,
            std::slice::from_ref(&elem.denom),
            false,
            elem.span(),
        ),
    }
}

/// Lays out a [`BinomElem`].
#[typst_macros::time(name = "math.binom", span = elem.span())]
pub fn layout_binom(
    elem: &Packed<BinomElem>,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<()> {
    layout_vertical_frac_like(ctx, styles, &elem.upper, &elem.lower, true, elem.span())
}

/// Layout a vertical fraction or binomial.
fn layout_vertical_frac_like(
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
        let mut left = GlyphFragment::new_char(ctx.font, styles, '(', span)?;
        left.stretch_vertical(ctx, height - short_fall);
        left.center_on_axis();
        ctx.push(left);
        ctx.push(FrameFragment::new(styles, frame));
        let mut right = GlyphFragment::new_char(ctx.font, styles, ')', span)?;
        right.stretch_vertical(ctx, height - short_fall);
        right.center_on_axis();
        ctx.push(right);
    } else {
        frame.push(
            line_pos,
            FrameItem::Shape(
                Geometry::Line(Point::with_x(line_width)).stroked(
                    FixedStroke::from_pair(
                        styles.get_ref(TextElem::fill).as_decoration(),
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

// Lays out a horizontal fraction
fn layout_horizontal_frac(
    ctx: &mut MathContext,
    styles: StyleChain,
    num: &Content,
    denom: &Content,
    span: Span,
    num_deparen: bool,
    denom_deparen: bool,
) -> SourceResult<()> {
    let num_frame = if num_deparen {
        ctx.layout_into_frame(
            &Content::sequence(vec![
                SymbolElem::packed('('),
                num.clone(),
                SymbolElem::packed(')'),
            ]),
            styles,
        )?
    } else {
        ctx.layout_into_frame(num, styles)?
    };
    ctx.push(FrameFragment::new(styles, num_frame));

    let mut slash = GlyphFragment::new_char(ctx.font, styles, '/', span)?;
    slash.center_on_axis();
    ctx.push(slash);

    let denom_frame = if denom_deparen {
        ctx.layout_into_frame(
            &Content::sequence(vec![
                SymbolElem::packed('('),
                denom.clone(),
                SymbolElem::packed(')'),
            ]),
            styles,
        )?
    } else {
        ctx.layout_into_frame(denom, styles)?
    };
    ctx.push(FrameFragment::new(styles, denom_frame));

    Ok(())
}

/// Lay out a skewed fraction.
fn layout_skewed_frac(
    ctx: &mut MathContext,
    styles: StyleChain,
    num: &Content,
    denom: &Content,
    span: Span,
) -> SourceResult<()> {
    let num_frame = ctx.layout_into_frame(num, styles)?;
    let denom_frame = ctx.layout_into_frame(denom, styles)?;

    let skew = FRAC_DIAGONAL_SKEW.resolve(styles);
    let offset = FRAC_DIAGONAL_OFFSET.resolve(styles);
    let short_fall = DELIM_SHORT_FALL.resolve(styles);

    let baseline = Abs::zero();
    let num_y = baseline - skew - num_frame.baseline();
    let denom_y = baseline + skew - denom_frame.baseline();

    // height without the slash
    let provisional_top = num_y.min(denom_y);
    let provisional_bottom =
        (num_y + num_frame.height()).max(denom_y + denom_frame.height());
    let provisional_height = provisional_bottom - provisional_top;

    // stretch the slash to (height - short_fall) and center it on the math axis.
    let mut slash_frag = GlyphFragment::new_char(ctx.font, styles, '/', span)?;
    slash_frag.stretch_vertical(ctx, provisional_height - short_fall);
    slash_frag.center_on_axis();
    let slash_frame = slash_frag.into_frame();

    let slash_y = baseline - slash_frame.baseline();

    let num_x = Abs::zero();
    let slash_x = num_frame.width() - offset;
    let denom_x = slash_x + slash_frame.width() - offset;

    let top = num_y.min(slash_y).min(denom_y);
    let bottom = (num_y + num_frame.height())
        .max(slash_y + slash_frame.height())
        .max(denom_y + denom_frame.height());
    let height = bottom - top;
    let width = denom_x + denom_frame.width();

    let mut frame = Frame::soft(Size::new(width, height));
    frame.set_baseline(baseline - top);

    let shift = -top;
    frame.push_frame(Point::new(num_x, num_y + shift), num_frame);
    frame.push_frame(Point::new(slash_x, slash_y + shift), slash_frame);
    frame.push_frame(Point::new(denom_x, denom_y + shift), denom_frame);

    ctx.push(FrameFragment::new(styles, frame));

    Ok(())
}
