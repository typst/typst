use ttf_parser::math::{GlyphAssembly, GlyphConstruction, GlyphPart};
use ttf_parser::LazyArray16;
use typst_library::diag::{warning, SourceResult};
use typst_library::foundations::{Packed, StyleChain};
use typst_library::layout::{Abs, Axis, Frame, Point, Rel, Size};
use typst_library::math::StretchElem;
use typst_utils::Get;

use super::{
    delimiter_alignment, GlyphFragment, MathContext, MathFragment, Scaled,
    VariantFragment,
};
use crate::modifiers::FrameModify;

/// Maximum number of times extenders can be repeated.
const MAX_REPEATS: usize = 1024;

/// Lays out a [`StretchElem`].
#[typst_macros::time(name = "math.stretch", span = elem.span())]
pub fn layout_stretch(
    elem: &Packed<StretchElem>,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<()> {
    let mut fragment = ctx.layout_into_fragment(&elem.body, styles)?;
    stretch_fragment(
        ctx,
        styles,
        &mut fragment,
        None,
        None,
        elem.size(styles),
        Abs::zero(),
    );
    ctx.push(fragment);
    Ok(())
}

/// Attempts to stretch the given fragment by/to the amount given in stretch.
pub fn stretch_fragment(
    ctx: &mut MathContext,
    styles: StyleChain,
    fragment: &mut MathFragment,
    axis: Option<Axis>,
    relative_to: Option<Abs>,
    stretch: Rel<Abs>,
    short_fall: Abs,
) {
    let glyph = match fragment {
        MathFragment::Glyph(glyph) => glyph.clone(),
        MathFragment::Variant(variant) => {
            GlyphFragment::new(ctx, styles, variant.c, variant.span)
        }
        _ => return,
    };

    // Return if we attempt to stretch along an axis which isn't stretchable,
    // so that the original fragment isn't modified.
    let Some(stretch_axis) = stretch_axis(ctx, &glyph) else { return };
    let axis = axis.unwrap_or(stretch_axis);
    if axis != stretch_axis {
        return;
    }

    let relative_to_size = relative_to.unwrap_or_else(|| fragment.size().get(axis));

    let mut variant = stretch_glyph(
        ctx,
        glyph,
        stretch.relative_to(relative_to_size) - short_fall,
        axis,
    );

    if axis == Axis::Y {
        variant.align_on_axis(ctx, delimiter_alignment(variant.c));
    }

    *fragment = MathFragment::Variant(variant);
}

/// Return whether the glyph is stretchable and if it is, along which axis it
/// can be stretched.
fn stretch_axis(ctx: &mut MathContext, base: &GlyphFragment) -> Option<Axis> {
    let base_id = base.id;
    let vertical = ctx
        .table
        .variants
        .and_then(|variants| variants.vertical_constructions.get(base_id))
        .map(|_| Axis::Y);
    let horizontal = ctx
        .table
        .variants
        .and_then(|variants| variants.horizontal_constructions.get(base_id))
        .map(|_| Axis::X);

    match (vertical, horizontal) {
        (vertical, None) => vertical,
        (None, horizontal) => horizontal,
        _ => {
            // As far as we know, there aren't any glyphs that have both
            // vertical and horizontal constructions. So for the time being, we
            // will assume that a glyph cannot have both.
            ctx.engine.sink.warn(warning!(
               base.span,
               "glyph has both vertical and horizontal constructions";
               hint: "this is probably a font bug";
               hint: "please file an issue at https://github.com/typst/typst/issues"
            ));

            None
        }
    }
}

/// Try to stretch a glyph to a desired width or height.
///
/// The resulting frame may not have the exact desired width.
pub fn stretch_glyph(
    ctx: &mut MathContext,
    mut base: GlyphFragment,
    target: Abs,
    axis: Axis,
) -> VariantFragment {
    // If the base glyph is good enough, use it.
    let advance = match axis {
        Axis::X => base.width,
        Axis::Y => base.height(),
    };
    if target <= advance {
        return base.into_variant();
    }

    let mut min_overlap = Abs::zero();
    let construction = ctx
        .table
        .variants
        .and_then(|variants| {
            min_overlap = variants.min_connector_overlap.scaled(ctx, base.font_size);
            match axis {
                Axis::X => variants.horizontal_constructions,
                Axis::Y => variants.vertical_constructions,
            }
            .get(base.id)
        })
        .unwrap_or(GlyphConstruction { assembly: None, variants: LazyArray16::new(&[]) });

    // Search for a pre-made variant with a good advance.
    let mut best_id = base.id;
    let mut best_advance = base.width;
    for variant in construction.variants {
        best_id = variant.variant_glyph;
        best_advance = base.font.to_em(variant.advance_measurement).at(base.font_size);
        if target <= best_advance {
            break;
        }
    }

    // This is either good or the best we've got.
    if target <= best_advance || construction.assembly.is_none() {
        base.set_id(ctx, best_id);
        return base.into_variant();
    }

    // Assemble from parts.
    let assembly = construction.assembly.unwrap();
    assemble(ctx, base, assembly, min_overlap, target, axis)
}

/// Assemble a glyph from parts.
fn assemble(
    ctx: &mut MathContext,
    base: GlyphFragment,
    assembly: GlyphAssembly,
    min_overlap: Abs,
    target: Abs,
    axis: Axis,
) -> VariantFragment {
    // Determine the number of times the extenders need to be repeated as well
    // as a ratio specifying how much to spread the parts apart
    // (0 = maximal overlap, 1 = minimal overlap).
    let mut full;
    let mut ratio;
    let mut repeat = 0;
    loop {
        full = Abs::zero();
        ratio = 0.0;

        let mut parts = parts(assembly, repeat).peekable();
        let mut growable = Abs::zero();

        while let Some(part) = parts.next() {
            let mut advance = part.full_advance.scaled(ctx, base.font_size);
            if let Some(next) = parts.peek() {
                let max_overlap = part
                    .end_connector_length
                    .min(next.start_connector_length)
                    .scaled(ctx, base.font_size);
                if max_overlap < min_overlap {
                    // This condition happening is indicative of a bug in the
                    // font.
                    ctx.engine.sink.warn(warning!(
                       base.span,
                       "glyph has assembly parts with overlap less than minConnectorOverlap";
                       hint: "its rendering may appear broken - this is probably a font bug";
                       hint: "please file an issue at https://github.com/typst/typst/issues"
                    ));
                }

                advance -= max_overlap;
                growable += max_overlap - min_overlap;
            }

            full += advance;
        }

        if full < target {
            let delta = target - full;
            ratio = (delta / growable).min(1.0);
            full += ratio * growable;
        }

        if target <= full || repeat >= MAX_REPEATS {
            break;
        }

        repeat += 1;
    }

    let mut selected = vec![];
    let mut parts = parts(assembly, repeat).peekable();
    while let Some(part) = parts.next() {
        let mut advance = part.full_advance.scaled(ctx, base.font_size);
        if let Some(next) = parts.peek() {
            let max_overlap = part
                .end_connector_length
                .min(next.start_connector_length)
                .scaled(ctx, base.font_size);
            advance -= max_overlap;
            advance += ratio * (max_overlap - min_overlap);
        }

        let mut fragment = base.clone();
        fragment.set_id(ctx, part.glyph_id);
        selected.push((fragment, advance));
    }

    let size;
    let baseline;
    match axis {
        Axis::X => {
            let height = base.ascent + base.descent;
            size = Size::new(full, height);
            baseline = base.ascent;
        }
        Axis::Y => {
            let axis = ctx.constants.axis_height().scaled(ctx, base.font_size);
            let width = selected.iter().map(|(f, _)| f.width).max().unwrap_or_default();
            size = Size::new(width, full);
            baseline = full / 2.0 + axis;
        }
    }

    let mut frame = Frame::soft(size);
    let mut offset = Abs::zero();
    frame.set_baseline(baseline);
    frame.modify(&base.modifiers);

    for (fragment, advance) in selected {
        let pos = match axis {
            Axis::X => Point::new(offset, frame.baseline() - fragment.ascent),
            Axis::Y => Point::with_y(full - offset - fragment.height()),
        };
        frame.push_frame(pos, fragment.into_frame());
        offset += advance;
    }

    let accent_attach = match axis {
        Axis::X => (frame.width() / 2.0, frame.width() / 2.0),
        Axis::Y => base.accent_attach,
    };

    VariantFragment {
        c: base.c,
        frame,
        font_size: base.font_size,
        italics_correction: Abs::zero(),
        accent_attach,
        class: base.class,
        math_size: base.math_size,
        span: base.span,
        limits: base.limits,
        mid_stretched: None,
        extended_shape: true,
    }
}

/// Return an iterator over the assembly's parts with extenders repeated the
/// specified number of times.
fn parts(assembly: GlyphAssembly, repeat: usize) -> impl Iterator<Item = GlyphPart> + '_ {
    assembly.parts.into_iter().flat_map(move |part| {
        let count = if part.part_flags.extender() { repeat } else { 1 };
        std::iter::repeat_n(part, count)
    })
}
