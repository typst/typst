use typst_library::diag::SourceResult;
use typst_library::foundations::StyleChain;
use typst_library::layout::{Abs, Axis, Frame, Point, Size};
use typst_library::math::ir::{AccentItem, MathProperties, Position};

use super::MathContext;
use super::fragment::FrameFragment;

/// Lays out an [`AccentItem`].
#[typst_macros::time(name = "math accent layout", span = props.span)]
pub fn layout_accent(
    item: &AccentItem,
    ctx: &mut MathContext,
    styles: StyleChain,
    props: &MathProperties,
) -> SourceResult<()> {
    let top_accent = item.position == Position::Above;

    let base = ctx.layout_into_fragment(&item.base, styles)?;
    let (font, size) = base.font(ctx, item.base.styles().unwrap_or(styles));
    let base_attach = base.accent_attach();

    // Try to replace the accent glyph with its flattened variant.
    let flattened_base_height = font.math().flattened_accent_base_height.at(size);
    let accent = &item.accent;
    if top_accent && base.ascent() > flattened_base_height {
        accent.set_flac();
    }

    accent.set_stretch_relative_to(base.width(), Axis::X);
    accent.set_stretch_font_size(size, Axis::X);

    let accent = ctx.layout_into_fragment(accent, styles)?;
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
    let base_ascent = base.base_ascent();
    let base_descent = base.base_descent();

    let mut frame = Frame::soft(size);
    frame.set_baseline(baseline);
    frame.push_frame(accent_pos, accent);
    frame.push_frame(base_pos, base.into_frame());

    ctx.push(
        FrameFragment::new(props, styles, frame)
            .with_base_ascent(base_ascent)
            .with_base_descent(base_descent)
            .with_italics_correction(base_italics_correction)
            .with_text_like(base_text_like)
            .with_accent_attach(base_attach),
    );
    Ok(())
}
