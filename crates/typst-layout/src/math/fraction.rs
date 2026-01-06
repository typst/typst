use typst_library::diag::SourceResult;
use typst_library::foundations::{Resolve, StyleChain};
use typst_library::layout::{Abs, Axis, Frame, FrameItem, Point, Size};
use typst_library::math::{FractionItem, MathProperties, MathSize, SkewedFractionItem};
use typst_library::text::TextElem;
use typst_library::visualize::{FixedStroke, Geometry};

use super::{FrameFragment, MathContext};

/// Lays out a [`FractionItem`].
#[typst_macros::time(name = "math fraction layout", span = props.span)]
pub fn layout_fraction(
    item: &FractionItem,
    ctx: &mut MathContext,
    styles: StyleChain,
    props: &MathProperties,
) -> SourceResult<()> {
    let constants = ctx.font().math();
    let axis = constants.axis_height.resolve(styles);
    let thickness = constants.fraction_rule_thickness.resolve(styles);
    let size = props.size;
    let shift_up = match size {
        MathSize::Display => constants.fraction_numerator_display_style_shift_up,
        _ => constants.fraction_numerator_shift_up,
    }
    .resolve(styles);
    let shift_down = match size {
        MathSize::Display => constants.fraction_denominator_display_style_shift_down,
        _ => constants.fraction_denominator_shift_down,
    }
    .resolve(styles);
    let num_min = match size {
        MathSize::Display => constants.fraction_num_display_style_gap_min,
        _ => constants.fraction_numerator_gap_min,
    }
    .resolve(styles);
    let denom_min = match size {
        MathSize::Display => constants.fraction_denom_display_style_gap_min,
        _ => constants.fraction_denominator_gap_min,
    }
    .resolve(styles);

    let num = ctx.layout_into_fragment(&item.numerator, styles)?.into_frame();
    let denom = ctx.layout_into_fragment(&item.denominator, styles)?.into_frame();

    let num_gap = (shift_up - (axis + thickness / 2.0) - num.descent()).max(num_min);
    let denom_gap =
        (shift_down + (axis - thickness / 2.0) - denom.ascent()).max(denom_min);

    let line_width = num.width().max(denom.width());
    let width = line_width + 2.0 * item.around.resolve(styles);
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

    if item.line {
        frame.push(
            line_pos,
            FrameItem::Shape(
                Geometry::Line(Point::with_x(line_width)).stroked(
                    FixedStroke::from_pair(
                        styles.get_ref(TextElem::fill).as_decoration(),
                        thickness,
                    ),
                ),
                props.span,
            ),
        );
    }

    ctx.push(FrameFragment::new(props, styles, frame));
    Ok(())
}

/// Lay out a skewed fraction.
#[typst_macros::time(name = "math skewed fraction layout", span = props.span)]
pub fn layout_skewed_fraction(
    item: &SkewedFractionItem,
    ctx: &mut MathContext,
    styles: StyleChain,
    props: &MathProperties,
) -> SourceResult<()> {
    // Font-derived constants
    let constants = ctx.font().math();
    let vgap = constants.skewed_fraction_vertical_gap.resolve(styles);
    let hgap = constants.skewed_fraction_horizontal_gap.resolve(styles);
    let axis = constants.axis_height.resolve(styles);

    let num_frame = ctx.layout_into_fragment(&item.numerator, styles)?.into_frame();
    let num_size = num_frame.size();
    let denom_frame = ctx.layout_into_fragment(&item.denominator, styles)?.into_frame();
    let denom_size = denom_frame.size();

    // Height of the fraction frame
    // We recalculate this value below if the slash glyph overflows
    let mut fraction_height = num_size.y + denom_size.y + vgap;

    // Build the slash glyph to calculate its size
    item.slash.set_stretch_relative_to(fraction_height, Axis::Y);
    let slash_frag = ctx.layout_into_fragment(&item.slash, styles)?;
    let slash_frame = slash_frag.into_frame();

    // Adjust the fraction height if the slash overflows
    let slash_size = slash_frame.size();
    let vertical_offset = Abs::zero().max(slash_size.y - fraction_height) / 2.0;
    fraction_height.set_max(slash_size.y);

    // Reference points for all three objects, used to place them in the frame.
    let mut slash_up_left = Point::new(num_size.x + hgap / 2.0, fraction_height / 2.0)
        - slash_size.to_point() / 2.0;
    let mut num_up_left = Point::with_y(vertical_offset);
    let mut denom_up_left = num_up_left + num_size.to_point() + Point::new(hgap, vgap);

    // Fraction width
    let fraction_width = (denom_up_left.x + denom_size.x)
        .max(slash_up_left.x + slash_size.x)
        + Abs::zero().max(-slash_up_left.x);
    // We have to shift everything right to avoid going in the negatives for
    // the x coordinate
    let horizontal_offset = Point::with_x(Abs::zero().max(-slash_up_left.x));
    slash_up_left += horizontal_offset;
    num_up_left += horizontal_offset;
    denom_up_left += horizontal_offset;

    // Build the final frame
    let mut fraction_frame = Frame::soft(Size::new(fraction_width, fraction_height));

    // Baseline (use axis height to center slash on the axis)
    fraction_frame.set_baseline(fraction_height / 2.0 + axis);

    // Numerator, Denominator, Slash
    fraction_frame.push_frame(num_up_left, num_frame);
    fraction_frame.push_frame(denom_up_left, denom_frame);
    fraction_frame.push_frame(slash_up_left, slash_frame);

    ctx.push(FrameFragment::new(props, styles, fraction_frame));
    Ok(())
}
