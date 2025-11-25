use typst_library::diag::SourceResult;
use typst_library::layout::{Abs, Frame, Point, Size};
use typst_library::math::{AccentItem, MathProperties};

use crate::math::run::MathFragmentsExt;

use super::{FrameFragment, MathContext, MathFragment, style_flac};

/// Lays out an [`AccentItem`].
#[typst_macros::time(name = "math.accent", span = props.span)]
pub fn layout_accent(
    item: &AccentItem,
    ctx: &mut MathContext,
    props: &MathProperties,
) -> SourceResult<()> {
    let top_accent = !item.is_bottom;

    let base = ctx.layout_into_fragment(&item.base)?;
    let (font, size) = base.font(ctx, props.styles);
    let base_attach = base.accent_attach();

    // Try to replace the accent glyph with its flattened variant.
    let flattened_base_height = font.math().flattened_accent_base_height.at(size);
    let flac = style_flac();
    let accent_styles = if top_accent && base.ascent() > flattened_base_height {
        props.styles.chain(&flac)
    } else {
        props.styles
    };

    let mut accent = ctx
        .layout_into_fragments(&item.accent, accent_styles)?
        .into_fragment(accent_styles);

    // Forcing the accent to be at least as large as the base makes it too wide
    // in many cases.
    let width = item.target.relative_to(base.width());
    accent.stretch_horizontal(ctx.engine, width, item.short_fall.at(size));
    let accent_attach = accent.accent_attach().0;
    let accent = accent.into_frame();

    // Calculate the width of the final frame.
    let (width, base_x, accent_x) = {
        let base_attach = if top_accent { base_attach.0 } else { base_attach.1 };
        if !item.exact_frame_width {
            (base.width(), Abs::zero(), base_attach - accent_attach)
        } else {
            let pre_width = accent_attach - base_attach;
            let post_width =
                (accent.width() - accent_attach) - (base.width() - base_attach);
            let width =
                pre_width.max(Abs::zero()) + base.width() + post_width.max(Abs::zero());
            if pre_width < Abs::zero() {
                (width, Abs::zero(), -pre_width)
            } else {
                (width, pre_width, Abs::zero())
            }
        }
    };

    let (gap, accent_pos, base_pos) = if top_accent {
        // Descent is negative because the accent's ink bottom is above the
        // baseline. Therefore, the default gap is the accent's negated descent
        // minus the accent base height. Only if the base is very small, we
        // need a larger gap so that the accent doesn't move too low.
        let accent_base_height = font.math().accent_base_height.at(size);
        let gap = -accent.descent() - base.ascent().min(accent_base_height);
        let accent_pos = Point::with_x(accent_x);
        let base_pos = Point::new(base_x, accent.height() + gap);
        (gap, accent_pos, base_pos)
    } else {
        let gap = -accent.ascent();
        let accent_pos = Point::new(accent_x, base.height() + gap);
        let base_pos = Point::with_x(base_x);
        (gap, accent_pos, base_pos)
    };

    let size = Size::new(width, accent.height() + gap + base.height());
    let baseline = base_pos.y + base.ascent();

    let base_text_like = !item.exact_frame_width && base.is_text_like();
    let base_italics_correction = base.italics_correction();
    let base_ascent = match &base {
        MathFragment::Frame(frame) => frame.base_ascent,
        _ => base.ascent(),
    };
    let base_descent = match &base {
        MathFragment::Frame(frame) => frame.base_descent,
        _ => base.descent(),
    };

    let mut frame = Frame::soft(size);
    frame.set_baseline(baseline);
    frame.push_frame(accent_pos, accent);
    frame.push_frame(base_pos, base.into_frame());
    ctx.push(
        FrameFragment::new(props, frame)
            .with_base_ascent(base_ascent)
            .with_base_descent(base_descent)
            .with_italics_correction(base_italics_correction)
            .with_text_like(base_text_like)
            .with_accent_attach(base_attach),
    );

    Ok(())
}
