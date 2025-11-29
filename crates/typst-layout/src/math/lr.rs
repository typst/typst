use typst_library::diag::SourceResult;
use typst_library::foundations::StyleChain;
use typst_library::layout::{Abs, Axis};
use typst_library::math::{FencedItem, MathProperties};

use super::{MathContext, MathFragment, stretch_fragment};

/// Lays out an [`FencedItem`].
#[typst_macros::time(name = "math.lr", span = props.span)]
pub fn layout_fenced(
    item: &FencedItem,
    ctx: &mut MathContext,
    styles: StyleChain,
    props: &MathProperties,
) -> SourceResult<()> {
    let mut body = ctx.layout_into_fragments(&item.body)?;
    let open = item
        .open
        .as_ref()
        .map(|open| ctx.layout_into_fragment(open))
        .transpose()?;
    let close = item
        .close
        .as_ref()
        .map(|close| ctx.layout_into_fragment(close))
        .transpose()?;

    let relative_to = if item.balanced {
        let mut max_extent = Abs::zero();
        for fragment in body.iter().chain(&open).chain(&close) {
            let (font, size) = fragment.font(ctx, styles);
            let axis = font.math().axis_height.at(size);
            let extent = (fragment.ascent() - axis).max(fragment.descent() + axis);
            max_extent = max_extent.max(extent);
        }
        2.0 * max_extent
    } else {
        body.iter().map(|f| f.height()).max().unwrap_or_default()
    };

    if let Some(mut open) = open {
        let short_fall = item.short_fall.at(open.font_size().unwrap_or_default());
        stretch_fragment(
            ctx.engine,
            &mut open,
            Some(Axis::Y),
            Some(relative_to),
            item.target,
            short_fall,
        );
        ctx.push(open);
    }

    // Handle MathFragment::Glyph fragments that should be scaled up.
    for fragment in body.iter_mut() {
        if let MathFragment::Glyph(glyph) = fragment
            && glyph.mid_stretched == Some(false)
        {
            glyph.mid_stretched = Some(true);
            let short_fall = item.short_fall.at(fragment.font_size().unwrap_or_default());
            stretch_fragment(
                ctx.engine,
                fragment,
                Some(Axis::Y),
                Some(relative_to),
                item.target,
                short_fall,
            );
        }
    }
    ctx.extend(body);

    if let Some(mut close) = close {
        let short_fall = item.short_fall.at(close.font_size().unwrap_or_default());
        stretch_fragment(
            ctx.engine,
            &mut close,
            Some(Axis::Y),
            Some(relative_to),
            item.target,
            short_fall,
        );
        ctx.push(close);
    }

    Ok(())
}
