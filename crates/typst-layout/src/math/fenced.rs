use typst_library::diag::SourceResult;
use typst_library::foundations::StyleChain;
use typst_library::layout::{Abs, Axis};
use typst_library::math::ir::{FencedItem, MathItem, MathProperties};

use super::{MathContext, fragment::MathFragment};

/// Lays out a [`FencedItem`].
#[typst_macros::time(name = "math fenced layout", span = props.span)]
pub fn layout_fenced(
    item: &FencedItem,
    ctx: &mut MathContext,
    styles: StyleChain,
    props: &MathProperties,
) -> SourceResult<()> {
    // Compute relative_to for delimiter sizing.
    let (relative_to, initial_body) = if let Some(sizing) = item.body.sizing() {
        let relative_to = sizing.try_get_or_update(|items| {
            relative_to_from_sizing(items, ctx, styles, item.balanced)
        })?;
        (relative_to, None)
    } else {
        let body = ctx.layout_into_fragments(&item.body, styles)?;
        let body_styles = item.body.styles().unwrap_or(styles);
        let relative_to =
            relative_to_from_fragments(&body, ctx, body_styles, item.balanced);
        (relative_to, Some(body))
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

    // Check if the body needs re-layout, since stretch info was updated or we
    // deferred initial layout.
    let body = if !has_mid_stretched && let Some(body) = initial_body {
        body
    } else {
        ctx.layout_into_fragments(&item.body, styles)?
    };
    ctx.extend(body);

    // Layout the closing delimiter if present.
    if let Some(close) = &item.close {
        close.set_stretch_relative_to(relative_to, Axis::Y);
        let close = ctx.layout_into_fragment(close, styles)?;
        ctx.push(close);
    }

    Ok(())
}

fn relative_to_from_sizing(
    items: &[MathItem],
    ctx: &mut MathContext,
    styles: StyleChain,
    balanced: bool,
) -> SourceResult<Abs> {
    items.iter().try_fold(Abs::zero(), |max_abs, item| {
        let fragments = ctx.layout_into_fragments(item, styles)?;
        let item_styles = item.styles().unwrap_or(styles);
        Ok(max_abs.max(relative_to_from_fragments(
            &fragments,
            ctx,
            item_styles,
            balanced,
        )))
    })
}

fn relative_to_from_fragments(
    fragments: &[MathFragment],
    ctx: &MathContext,
    styles: StyleChain,
    balanced: bool,
) -> Abs {
    fragments
        .iter()
        .map(|f| {
            if balanced {
                let (font, size) = f.font(ctx, styles);
                let axis = font.math().axis_height.at(size);
                2.0 * (f.ascent() - axis).max(f.descent() + axis)
            } else {
                f.height()
            }
        })
        .max()
        .unwrap_or_default()
}
