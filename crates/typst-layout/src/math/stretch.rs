use typst_library::diag::{warning, SourceResult};
use typst_library::foundations::{Packed, StyleChain};
use typst_library::layout::{Abs, Axis, Rel};
use typst_library::math::StretchElem;
use typst_utils::Get;

use super::{stretch_axes, MathContext, MathFragment};

/// Lays out a [`StretchElem`].
#[typst_macros::time(name = "math.stretch", span = elem.span())]
pub fn layout_stretch(
    elem: &Packed<StretchElem>,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<()> {
    let mut fragment = ctx.layout_into_fragment(&elem.body, styles)?;
    stretch_fragment(ctx, &mut fragment, None, None, elem.size(styles), Abs::zero());
    ctx.push(fragment);
    Ok(())
}

/// Attempts to stretch the given fragment by/to the amount given in stretch.
pub fn stretch_fragment(
    ctx: &mut MathContext,
    fragment: &mut MathFragment,
    axis: Option<Axis>,
    relative_to: Option<Abs>,
    stretch: Rel<Abs>,
    short_fall: Abs,
) {
    let size = fragment.size();

    let MathFragment::Glyph(glyph) = fragment else { return };

    // Return if we attempt to stretch along an axis which isn't stretchable,
    // so that the original fragment isn't modified.
    let axes = stretch_axes(&glyph.item.font, glyph.base_glyph.id);
    let stretch_axis = if let Some(axis) = axis {
        if !axes.get(axis) {
            return;
        }
        axis
    } else {
        match (axes.x, axes.y) {
            (true, false) => Axis::X,
            (false, true) => Axis::Y,
            (false, false) => return,
            (true, true) => {
                // As far as we know, there aren't any glyphs that have both
                // vertical and horizontal constructions. So for the time being, we
                // will assume that a glyph cannot have both.
                ctx.engine.sink.warn(warning!(
                   glyph.item.glyphs[0].span.0,
                   "glyph has both vertical and horizontal constructions";
                   hint: "this is probably a font bug";
                   hint: "please file an issue at https://github.com/typst/typst/issues"
                ));
                return;
            }
        }
    };

    let relative_to_size = relative_to.unwrap_or_else(|| size.get(stretch_axis));

    glyph.stretch(ctx, stretch.relative_to(relative_to_size) - short_fall, stretch_axis);

    if stretch_axis == Axis::Y {
        glyph.center_on_axis();
    }
}
