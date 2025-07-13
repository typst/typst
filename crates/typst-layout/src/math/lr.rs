use typst_library::diag::SourceResult;
use typst_library::foundations::{Packed, StyleChain};
use typst_library::layout::{Abs, Axis, Rel};
use typst_library::math::{EquationElem, LrElem, MidElem};
use typst_utils::SliceExt;
use unicode_math_class::MathClass;

use super::{stretch_fragment, MathContext, MathFragment, DELIM_SHORT_FALL};

/// Lays out an [`LrElem`].
#[typst_macros::time(name = "math.lr", span = elem.span())]
pub fn layout_lr(
    elem: &Packed<LrElem>,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<()> {
    // Extract from an EquationElem.
    let mut body = &elem.body;
    if let Some(equation) = body.to_packed::<EquationElem>() {
        body = &equation.body;
    }

    // Extract implicit LrElem.
    if let Some(lr) = body.to_packed::<LrElem>() {
        if lr.size.get(styles).is_one() {
            body = &lr.body;
        }
    }

    let mut fragments = ctx.layout_into_fragments(body, styles)?;

    // Ignore leading and trailing ignorant fragments.
    let (start_idx, end_idx) = fragments.split_prefix_suffix(|f| f.is_ignorant());
    let inner_fragments = &mut fragments[start_idx..end_idx];

    let axis = scaled!(ctx, styles, axis_height);
    let max_extent = inner_fragments
        .iter()
        .map(|fragment| (fragment.ascent() - axis).max(fragment.descent() + axis))
        .max()
        .unwrap_or_default();

    let relative_to = 2.0 * max_extent;
    let height = elem.size.resolve(styles);

    // Scale up fragments at both ends.
    match inner_fragments {
        [one] => scale_if_delimiter(ctx, one, relative_to, height, None),
        [first, .., last] => {
            scale_if_delimiter(ctx, first, relative_to, height, Some(MathClass::Opening));
            scale_if_delimiter(ctx, last, relative_to, height, Some(MathClass::Closing));
        }
        [] => {}
    }

    // Handle MathFragment::Glyph fragments that should be scaled up.
    for fragment in inner_fragments.iter_mut() {
        if let MathFragment::Glyph(ref mut glyph) = fragment {
            if glyph.mid_stretched == Some(false) {
                glyph.mid_stretched = Some(true);
                scale(ctx, fragment, relative_to, height);
            }
        }
    }

    // Remove weak SpacingFragment immediately after the opening or immediately
    // before the closing.
    let mut index = 0;
    let opening_exists = inner_fragments
        .first()
        .is_some_and(|f| f.class() == MathClass::Opening);
    let closing_exists = inner_fragments
        .last()
        .is_some_and(|f| f.class() == MathClass::Closing);
    fragments.retain(|fragment| {
        let discard = (index == start_idx + 1 && opening_exists
            || index + 2 == end_idx && closing_exists)
            && matches!(fragment, MathFragment::Spacing(_, true));
        index += 1;
        !discard
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
    let mut fragments = ctx.layout_into_fragments(&elem.body, styles)?;

    for fragment in &mut fragments {
        if let MathFragment::Glyph(ref mut glyph) = fragment {
            glyph.mid_stretched = Some(false);
            glyph.class = MathClass::Relation;
        }
    }

    ctx.extend(fragments);
    Ok(())
}

/// Scales a math fragment to a height if it has the class Opening, Closing, or
/// Fence.
///
/// In case `apply` is `Some(class)`, `class` will be applied to the fragment if
/// it is a delimiter, in a way that cannot be overridden by the user.
fn scale_if_delimiter(
    ctx: &mut MathContext,
    fragment: &mut MathFragment,
    relative_to: Abs,
    height: Rel<Abs>,
    apply: Option<MathClass>,
) {
    if matches!(
        fragment.class(),
        MathClass::Opening | MathClass::Closing | MathClass::Fence
    ) {
        scale(ctx, fragment, relative_to, height);

        if let Some(class) = apply {
            fragment.set_class(class);
        }
    }
}

/// Scales a math fragment to a height.
fn scale(
    ctx: &mut MathContext,
    fragment: &mut MathFragment,
    relative_to: Abs,
    height: Rel<Abs>,
) {
    // This unwrap doesn't really matter. If it is None, then the fragment
    // won't be stretchable anyways.
    let short_fall = DELIM_SHORT_FALL.at(fragment.font_size().unwrap_or_default());
    stretch_fragment(ctx, fragment, Some(Axis::Y), Some(relative_to), height, short_fall);
}
