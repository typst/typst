use ttf_parser::math::{GlyphAssembly, GlyphConstruction, GlyphPart};
use ttf_parser::LazyArray16;

use crate::diag::SourceResult;
use crate::foundations::{elem, Content, Packed, Resolve, Smart, StyleChain};
use crate::layout::{Abs, Frame, Length, Point, Rel, Size, VAlignment};
use crate::math::{
    GlyphFragment, LayoutMath, MathContext, MathFragment, Scaled, VariantFragment,
};

/// Maximum number of times extenders can be repeated.
const MAX_REPEATS: usize = 1024;

/// Stretches a base.
///
/// This function can also be used to automatically stretch the base of an
/// attachment, so that it fits the top and bottom attachments.
///
/// Note that only some glyphs can be stretched, and which ones can depends on
/// the math font being used. Generally however, all math fonts are the same in
/// this regard.
///
/// ```example
/// $ H stretch(=)^"define" U + p V $
/// $ f : X stretch(->>, size: #150%)_"surjective" Y $
/// $ x stretch(harpoons.ltrb, size: #3em) y
///     stretch(\[, size: #150%) z $
/// ```
#[elem(LayoutMath)]
pub struct StretchElem {
    /// The base to stretch.
    #[required]
    pub base: Content,

    /// The size to stretch to, relative to the glyph's current size.
    pub size: Smart<Rel<Length>>,

    /// Whether the size should be relative to the width of any top or bottom
    /// attachments.
    ///
    /// ```example
    /// #set math.stretch(attach: false)
    /// $ stretch(->, size: #250%)_"very long text" $
    /// ```
    #[default(true)]
    pub attach: bool,
}

impl LayoutMath for Packed<StretchElem> {
    #[typst_macros::time(name = "math.stretch", span = self.span())]
    fn layout_math(&self, ctx: &mut MathContext, styles: StyleChain) -> SourceResult<()> {
        let mut fragment = ctx.layout_into_fragment(self.base(), styles)?;
        stretch_fragment(
            ctx,
            styles,
            &mut fragment,
            None,
            None,
            self.size(styles),
            Abs::zero(),
        );
        ctx.push(fragment);
        Ok(())
    }
}

/// Attempts to stretch the given fragment by/to the amount given in stretch.
pub(super) fn stretch_fragment(
    ctx: &mut MathContext,
    styles: StyleChain,
    fragment: &mut MathFragment,
    axis_horizontal: Option<bool>,
    relative_to: Option<Abs>,
    stretch: Smart<Rel<Length>>,
    short_fall: Abs,
) {
    let glyph = match fragment {
        MathFragment::Glyph(glyph) => glyph.clone(),
        MathFragment::Variant(variant) => {
            GlyphFragment::new(ctx, styles, variant.c, variant.span)
        }
        _ => return,
    };

    let horizontal = if let Some(axis) = axis_horizontal {
        axis
    } else if let Some(axis) = is_stretchable(ctx, &glyph) {
        axis
    } else {
        return;
    };

    let relative_to_size = relative_to.unwrap_or_else(|| {
        if horizontal {
            fragment.width()
        } else {
            fragment.height()
        }
    });

    let mut variant = stretch_glyph(
        ctx,
        glyph,
        stretch
            .unwrap_or(Rel::one())
            .resolve(styles)
            .relative_to(relative_to_size),
        short_fall,
        horizontal,
    );

    if !horizontal {
        variant.align_on_axis(ctx, delimiter_alignment(variant.c));
    }

    *fragment = MathFragment::Variant(variant);
}

pub(super) fn delimiter_alignment(delimiter: char) -> VAlignment {
    match delimiter {
        '\u{231c}' | '\u{231d}' => VAlignment::Top,
        '\u{231e}' | '\u{231f}' => VAlignment::Bottom,
        _ => VAlignment::Horizon,
    }
}

/// Return whether the glyph is stretchable and if it is, along which axis it
/// can be stretched.
fn is_stretchable(ctx: &MathContext, base: &GlyphFragment) -> Option<bool> {
    let base_id = base.id;
    let vertical = ctx
        .table
        .variants
        .and_then(|variants| variants.vertical_constructions.get(base_id))
        .map(|_| false);
    let horizontal = ctx
        .table
        .variants
        .and_then(|variants| variants.horizontal_constructions.get(base_id))
        .map(|_| true);

    match (vertical, horizontal) {
        (vertical, None) => vertical,
        (None, horizontal) => horizontal,
        _ => {
            // As far as we know, there aren't any glyphs that have both
            // vertical and horizontal constructions. So for the time being, we
            // will assume that a glyph cannot have both.
            panic!("glyph {:?} has both vertical and horizontal constructions", base.c);
        }
    }
}

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
    // If the base glyph is good enough, use it.
    let advance = if horizontal { base.width } else { base.height() };
    let short_target = target - short_fall;
    if short_target <= advance {
        return base.into_variant();
    }

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
    frame.post_process_raw(base.dests, base.hidden);

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
