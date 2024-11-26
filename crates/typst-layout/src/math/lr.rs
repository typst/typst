use typst_library::diag::SourceResult;
use typst_library::foundations::{Packed, Smart, StyleChain};
use typst_library::layout::{Abs, Axis, Length, Rel};
use typst_library::math::{EquationElem, LrElem, MidElem};
use unicode_math_class::MathClass;

use super::{stretch_fragment, MathContext, MathFragment, DELIM_SHORT_FALL};

/// Lays out an [`LrElem`].
#[typst_macros::time(name = "math.lr", span = elem.span())]
pub fn layout_lr(
    elem: &Packed<LrElem>,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<()> {
    let mut body = elem.body();

    // Extract from an EquationElem.
    if let Some(equation) = body.to_packed::<EquationElem>() {
        body = equation.body();
    }

    // Extract implicit LrElem.
    if let Some(lr) = body.to_packed::<LrElem>() {
        if lr.size(styles).is_auto() {
            body = lr.body();
        }
    }

    let mut fragments = ctx.layout_into_fragments(body, styles)?;

    // Ignore leading and trailing ignorant fragments.
    let start_idx = fragments
        .iter()
        .position(|f| !f.is_ignorant())
        .unwrap_or(fragments.len());
    let end_idx = fragments
        .iter()
        .skip(start_idx)
        .rposition(|f| !f.is_ignorant())
        .map_or(start_idx, |i| start_idx + i + 1);
    let inner_fragments = &mut fragments[start_idx..end_idx];

    let axis = scaled!(ctx, styles, axis_height);
    let max_extent = inner_fragments
        .iter()
        .map(|fragment| (fragment.ascent() - axis).max(fragment.descent() + axis))
        .max()
        .unwrap_or_default();

    let relative_to = 2.0 * max_extent;
    let height = elem.size(styles);

    // Scale up fragments at both ends.
    match inner_fragments {
        [one] => scale(ctx, styles, one, relative_to, height, None),
        [first, .., last] => {
            scale(ctx, styles, first, relative_to, height, Some(MathClass::Opening));
            scale(ctx, styles, last, relative_to, height, Some(MathClass::Closing));
        }
        _ => {}
    }

    // Handle MathFragment::Variant fragments that should be scaled up.
    for fragment in inner_fragments {
        if let MathFragment::Variant(ref mut variant) = fragment {
            if variant.mid_stretched == Some(false) {
                variant.mid_stretched = Some(true);
                scale(ctx, styles, fragment, relative_to, height, Some(MathClass::Large));
            }
        }
    }

    // Remove weak SpacingFragment immediately after the opening or immediately
    // before the closing.
    let mut index = 0;
    fragments.retain(|fragment| {
        index += 1;
        (index != start_idx + 2 && index + 1 != end_idx)
            || !matches!(fragment, MathFragment::Spacing(_, true))
    });

    ctx.extend(fragments);

    Ok(())
}

/// Lays out a [`MidElem`].
#[typst_macros::time(name = "math.mid", span = elem.span())]
pub fn layout_mid(
    elem: &Packed<MidElem>,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<()> {
    let mut fragments = ctx.layout_into_fragments(elem.body(), styles)?;

    for fragment in &mut fragments {
        match fragment {
            MathFragment::Glyph(glyph) => {
                let mut new = glyph.clone().into_variant();
                new.mid_stretched = Some(false);
                new.class = MathClass::Fence;
                *fragment = MathFragment::Variant(new);
            }
            MathFragment::Variant(variant) => {
                variant.mid_stretched = Some(false);
                variant.class = MathClass::Fence;
            }
            _ => {}
        }
    }

    ctx.extend(fragments);
    Ok(())
}

/// Scale a math fragment to a height.
fn scale(
    ctx: &mut MathContext,
    styles: StyleChain,
    fragment: &mut MathFragment,
    relative_to: Abs,
    height: Smart<Rel<Length>>,
    apply: Option<MathClass>,
) {
    if matches!(
        fragment.class(),
        MathClass::Opening | MathClass::Closing | MathClass::Fence
    ) {
        // This unwrap doesn't really matter. If it is None, then the fragment
        // won't be stretchable anyways.
        let short_fall = DELIM_SHORT_FALL.at(fragment.font_size().unwrap_or_default());
        stretch_fragment(
            ctx,
            styles,
            fragment,
            Some(Axis::Y),
            Some(relative_to),
            height,
            short_fall,
        );

        if let Some(class) = apply {
            fragment.set_class(class);
        }
    }
}
