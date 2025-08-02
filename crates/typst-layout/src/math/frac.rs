use typst_library::diag::SourceResult;
use typst_library::foundations::{
    Content, NativeElement, Packed, Resolve, StyleChain, SymbolElem,
};
use typst_library::layout::{Abs, Em, Frame, FrameItem, Point, Size};
use typst_library::math::{BinomElem, FracElem, FracStyle, LrElem};
use typst_library::text::TextElem;
use typst_library::visualize::{FixedStroke, Geometry};
use typst_syntax::Span;

use super::{
    DELIM_SHORT_FALL, FrameFragment, GlyphFragment, MathContext, style_for_denominator,
    style_for_numerator,
};

const FRAC_AROUND: Em = Em::new(0.1);

/// Lays out a [`FracElem`].
#[typst_macros::time(name = "math.frac", span = elem.span())]
pub fn layout_frac(
    elem: &Packed<FracElem>,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<()> {
    match elem.style.get(styles) {
        FracStyle::Skewed => {
            layout_skewed_frac(ctx, styles, &elem.num, &elem.denom, elem.span())
        }
        FracStyle::Horizontal => layout_horizontal_frac(
            ctx,
            styles,
            &elem.num,
            &elem.denom,
            elem.span(),
            elem.num_deparenthesized.as_option().unwrap_or(false),
            elem.denom_deparenthesized.as_option().unwrap_or(false),
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
    let num = if num_deparen {
        &LrElem::new(Content::sequence(vec![
            SymbolElem::packed('('),
            num.clone(),
            SymbolElem::packed(')'),
        ]))
        .pack()
    } else {
        num
    };
    let num_frame = ctx.layout_into_frame(num, styles)?;
    ctx.push(FrameFragment::new(styles, num_frame));

    let mut slash = GlyphFragment::new_char(ctx.font, styles, '/', span)?;
    slash.center_on_axis();
    ctx.push(slash);

    let denom = if denom_deparen {
        &LrElem::new(Content::sequence(vec![
            SymbolElem::packed('('),
            denom.clone(),
            SymbolElem::packed(')'),
        ]))
        .pack()
    } else {
        denom
    };
    let denom_frame = ctx.layout_into_fragment(denom, styles)?;
    ctx.push(denom_frame);

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
    // Font-derived constants
    let vgap = scaled!(ctx, styles, skewed_fraction_vertical_gap);
    let hgap = scaled!(ctx, styles, skewed_fraction_horizontal_gap);
    let axis = scaled!(ctx, styles, axis_height);
    // let vgap = vgap + axis / 2.0; // LuaTeX behavior

    let num_style = style_for_numerator(styles);
    let num_frame = ctx.layout_into_frame(num, styles.chain(&num_style))?;
    let num_size = num_frame.size();
    let denom_style = style_for_denominator(styles);
    let denom_frame = ctx.layout_into_frame(denom, styles.chain(&denom_style))?;
    let denom_size = denom_frame.size();

    let short_fall = DELIM_SHORT_FALL.resolve(styles);

    // Size of the fraction frame
    // We recalculate these values below if the slash glyph overflows
    let mut fraction_height = num_size.y + denom_size.y + vgap;
    let fraction_width;

    // Build the slash glyph to calculate its size
    let mut slash_frag = GlyphFragment::new_char(ctx.font, styles, '\u{2044}', span)?;
    let pre_stretch_height = slash_frag.size.y;
    slash_frag.stretch_vertical(ctx, fraction_height - short_fall);
    // If the standard slash was not stretchable, try the fraction slash
    if slash_frag.size.y == pre_stretch_height {
        slash_frag = GlyphFragment::new_char(ctx.font, styles, '/', span)?;
        slash_frag.stretch_vertical(ctx, fraction_height - short_fall);
    }
    slash_frag.center_on_axis();
    let slash_frame = slash_frag.into_frame();

    // Adjust the fraction height if the slash overflows
    // Fraction width will be re-calculated later on after we adjusted the x values to avoid
    // overlap with the slash.
    let slash_size = slash_frame.size();
    let vertical_offset = Abs::zero().max(slash_size.y - fraction_height) / 2.0;
    fraction_height.set_max(slash_size.y);

    // Reference points for all three objects, used to place them in the frame.
    let mut slash_center = Point::new(num_size.x + hgap / 2.0, fraction_height / 2.0);
    let mut num_up_left = Point::with_y(vertical_offset);
    let mut denom_up_left = num_up_left + num_size.to_point() + Point::new(hgap, vgap);

    // Check for overlap with the slash glyph. We assume the slash is a straight line without
    // thickness that joins the upper right corner to the lower left corner of slash_frame.
    // Begin with the numerator
    let vec_num_slash = num_up_left + num_size.to_point() - slash_center;
    let mut extra_hgap = Point::zero();
    if vec_num_slash.x.to_raw() * slash_size.y.to_raw()
        + vec_num_slash.y.to_raw() * slash_size.x.to_raw()
        > 0.0
    {
        extra_hgap = Point::with_x(
            vec_num_slash.x
                + vec_num_slash.y * slash_size.x.to_raw() / slash_size.y.to_raw(),
        )
    }
    // Shift slash and denom to the right so that the num no longer overlaps
    slash_center += extra_hgap;
    denom_up_left += extra_hgap;
    // Same with denominator
    let vec_denom_slash = denom_up_left - slash_center;
    extra_hgap = Point::zero();
    if vec_denom_slash.x.to_raw() * slash_size.y.to_raw()
        + vec_denom_slash.y.to_raw() * slash_size.x.to_raw()
        < 0.0
    {
        extra_hgap = -Point::with_x(
            vec_denom_slash.x
                + vec_denom_slash.y * slash_size.x.to_raw() / slash_size.y.to_raw(),
        )
    }
    denom_up_left += extra_hgap;

    // Adjust final width
    let mut slash_up_left = slash_center - slash_size.to_point() / 2.0;
    fraction_width = (denom_up_left.x + denom_size.x)
        .max(slash_center.x + slash_size.x / 2.0)
        - num_up_left.x.min(slash_up_left.x);
    // We have to shift everything right to avoid going in the negatives for the x coordinate
    let horizontal_offset = Point::with_x(Abs::zero().max(num_up_left.x - slash_up_left.x));
    slash_up_left += horizontal_offset;
    num_up_left += horizontal_offset;
    denom_up_left += horizontal_offset;

    // Build the final frame
    let mut fraction_frame = Frame::soft(Size::new(fraction_width, fraction_height));

    // Baseline (use axis height to center slash on the axis)
    fraction_frame.set_baseline(fraction_height / 2.0 + axis);

    // Debugging help
    // num_frame.mark_box_in_place();
    // denom_frame.mark_box_in_place();
    // slash_frame.mark_box_in_place();
    // fraction_frame.mark_box_in_place();

    // Numerator, Denominator, Slash
    fraction_frame.push_frame(num_up_left, num_frame);
    fraction_frame.push_frame(denom_up_left, denom_frame);
    fraction_frame.push_frame(slash_up_left, slash_frame);

    ctx.push(FrameFragment::new(styles, fraction_frame));

    Ok(())
}
