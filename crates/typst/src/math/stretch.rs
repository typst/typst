use ttf_parser::math::{GlyphAssembly, GlyphConstruction, GlyphPart};
use ttf_parser::LazyArray16;

use crate::layout::{Abs, Frame, Point, Size};
use crate::math::{GlyphFragment, MathContext, Scaled, VariantFragment};

/// Maximum number of times extenders can be repeated.
const MAX_REPEATS: usize = 1024;

impl GlyphFragment {
    /// Try to stretch a glyph to a desired height.
    pub fn stretch_vertical(
        self,
        ctx: &MathContext,
        height: Abs,
        short_fall: Abs,
    ) -> VariantFragment {
        stretch_glyph(ctx, self, height, short_fall, false)
    }

    /// Try to stretch a glyph to a desired width.
    pub fn stretch_horizontal(
        self,
        ctx: &MathContext,
        width: Abs,
        short_fall: Abs,
    ) -> VariantFragment {
        stretch_glyph(ctx, self, width, short_fall, true)
    }
}

/// Try to stretch a glyph to a desired width or height.
///
/// The resulting frame may not have the exact desired width.
fn stretch_glyph(
    ctx: &MathContext,
    mut base: GlyphFragment,
    target: Abs,
    short_fall: Abs,
    horizontal: bool,
) -> VariantFragment {
    let short_target = target - short_fall;
    let mut min_overlap = Abs::zero();
    let construction = ctx
        .table
        .variants
        .and_then(|variants| {
            min_overlap = variants.min_connector_overlap.scaled(ctx, base.font_size);
            if horizontal {
                variants.horizontal_constructions
            } else {
                variants.vertical_constructions
            }
            .get(base.id)
        })
        .unwrap_or(GlyphConstruction { assembly: None, variants: LazyArray16::new(&[]) });

    // If the base glyph is good enough, use it.
    let advance = if horizontal { base.width } else { base.height() };
    if short_target <= advance {
        return base.into_variant();
    }

    // Search for a pre-made variant with a good advance.
    let mut best_id = base.id;
    let mut best_advance = base.width;
    for variant in construction.variants {
        best_id = variant.variant_glyph;
        best_advance = base.font.to_em(variant.advance_measurement).at(base.font_size);
        if short_target <= best_advance {
            break;
        }
    }

    // This is either good or the best we've got.
    if short_target <= best_advance || construction.assembly.is_none() {
        base.set_id(ctx, best_id);
        return base.into_variant();
    }

    // Assemble from parts.
    let assembly = construction.assembly.unwrap();
    assemble(ctx, base, assembly, min_overlap, target, horizontal)
}

/// Assemble a glyph from parts.
fn assemble(
    ctx: &MathContext,
    base: GlyphFragment,
    assembly: GlyphAssembly,
    min_overlap: Abs,
    target: Abs,
    horizontal: bool,
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
    if horizontal {
        let height = base.ascent + base.descent;
        size = Size::new(full, height);
        baseline = base.ascent;
    } else {
        let axis = ctx.constants.axis_height().scaled(ctx, base.font_size);
        let width = selected.iter().map(|(f, _)| f.width).max().unwrap_or_default();
        size = Size::new(width, full);
        baseline = full / 2.0 + axis;
    }

    let mut frame = Frame::soft(size);
    let mut offset = Abs::zero();
    frame.set_baseline(baseline);
    frame.meta_iter(base.meta);

    for (fragment, advance) in selected {
        let pos = if horizontal {
            Point::new(offset, frame.baseline() - fragment.ascent)
        } else {
            Point::with_y(full - offset - fragment.height())
        };
        frame.push_frame(pos, fragment.into_frame());
        offset += advance;
    }

    let accent_attach = if horizontal { frame.width() / 2.0 } else { base.accent_attach };

    VariantFragment {
        c: base.c,
        id: None,
        frame,
        font_size: base.font_size,
        italics_correction: Abs::zero(),
        accent_attach,
        class: base.class,
        math_size: base.math_size,
        span: base.span,
        limits: base.limits,
        mid_stretched: None,
    }
}

/// Return an iterator over the assembly's parts with extenders repeated the
/// specified number of times.
fn parts(assembly: GlyphAssembly, repeat: usize) -> impl Iterator<Item = GlyphPart> + '_ {
    assembly.parts.into_iter().flat_map(move |part| {
        let count = if part.part_flags.extender() { repeat } else { 1 };
        std::iter::repeat(part).take(count)
    })
}
