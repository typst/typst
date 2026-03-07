use typst_library::diag::SourceResult;
use typst_library::foundations::StyleChain;
use typst_library::layout::{Abs, Axis};
use typst_library::math::ir::{FencedItem, MathProperties};

use super::MathContext;

/// Lays out a [`FencedItem`].
#[typst_macros::time(name = "math fenced layout", span = props.span)]
pub fn layout_fenced(
    item: &FencedItem,
    ctx: &mut MathContext,
    styles: StyleChain,
    props: &MathProperties,
) -> SourceResult<()> {
    // Layout the body to compute relative_to for delimiter sizing.
    let body = ctx.layout_into_fragments(&item.body, styles)?;
    let relative_to = if item.balanced {
        let mut max_extent = Abs::zero();
        for fragment in body.iter() {
            let (font, size) = fragment.font(ctx, item.body.styles().unwrap_or(styles));
            let axis = font.math().axis_height.at(size);
            let extent = (fragment.ascent() - axis).max(fragment.descent() + axis);
            max_extent = max_extent.max(extent);
        }
        2.0 * max_extent
    } else {
        body.iter().map(|f| f.height()).max().unwrap_or_default()
    };

    // Set stretch info for stretched mid items.
    let mut has_mid_stretched = false;
    for body_item in item.body.as_slice() {
        if body_item.mid_stretched().is_some_and(|x| x) {
            has_mid_stretched = true;
            body_item.set_stretch_relative_to(relative_to, Axis::Y);
        }
    }

    // Layout the opening delimiter if present.
    if let Some(open) = &item.open {
        open.set_stretch_relative_to(relative_to, Axis::Y);
        let open = ctx.layout_into_fragment(open, styles)?;
        ctx.push(open);
    }

    // Check if the body needs re-layout, since stretch info was updated after
    // initial layout.
    if has_mid_stretched {
        let body = ctx.layout_into_fragments(&item.body, styles)?;
        ctx.extend(body);
    } else {
        ctx.extend(body);
    }

    // Layout the closing delimiter if present.
    if let Some(close) = &item.close {
        close.set_stretch_relative_to(relative_to, Axis::Y);
        let close = ctx.layout_into_fragment(close, styles)?;
        ctx.push(close);
    }

    Ok(())
}
