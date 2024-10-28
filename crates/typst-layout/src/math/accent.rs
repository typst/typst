use typst_library::diag::SourceResult;
use typst_library::foundations::{Packed, StyleChain};
use typst_library::layout::{Em, Frame, Point, Rel, Size};
use typst_library::math::{Accent, AccentElem};

use super::{
    scaled_font_size, style_cramped, FrameFragment, GlyphFragment, MathContext,
    MathFragment,
};

/// How much the accent can be shorter than the base.
const ACCENT_SHORT_FALL: Em = Em::new(0.5);

/// Lays out an [`AccentElem`].
#[typst_macros::time(name = "math.accent", span = elem.span())]
pub fn layout_accent(
    elem: &Packed<AccentElem>,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<()> {
    let cramped = style_cramped();
    let base = ctx.layout_into_fragment(elem.base(), styles.chain(&cramped))?;

    // Preserve class to preserve automatic spacing.
    let base_class = base.class();
    let base_attach = base.accent_attach();

    let width = elem
        .size(styles)
        .unwrap_or(Rel::one())
        .at(scaled_font_size(ctx, styles))
        .relative_to(base.width());

    // Forcing the accent to be at least as large as the base makes it too
    // wide in many case.
    let Accent(c) = elem.accent();
    let glyph = GlyphFragment::new(ctx, styles, *c, elem.span());
    let short_fall = ACCENT_SHORT_FALL.at(glyph.font_size);
    let variant = glyph.stretch_horizontal(ctx, width, short_fall);
    let accent = variant.frame;
    let accent_attach = variant.accent_attach;

    // Descent is negative because the accent's ink bottom is above the
    // baseline. Therefore, the default gap is the accent's negated descent
    // minus the accent base height. Only if the base is very small, we need
    // a larger gap so that the accent doesn't move too low.
    let accent_base_height = scaled!(ctx, styles, accent_base_height);
    let gap = -accent.descent() - base.height().min(accent_base_height);
    let size = Size::new(base.width(), accent.height() + gap + base.height());
    let accent_pos = Point::with_x(base_attach - accent_attach);
    let base_pos = Point::with_y(accent.height() + gap);
    let baseline = base_pos.y + base.ascent();
    let base_italics_correction = base.italics_correction();
    let base_text_like = base.is_text_like();

    let base_ascent = match &base {
        MathFragment::Frame(frame) => frame.base_ascent,
        _ => base.ascent(),
    };

    let mut frame = Frame::soft(size);
    frame.set_baseline(baseline);
    frame.push_frame(accent_pos, accent);
    frame.push_frame(base_pos, base.into_frame());
    ctx.push(
        FrameFragment::new(ctx, styles, frame)
            .with_class(base_class)
            .with_base_ascent(base_ascent)
            .with_italics_correction(base_italics_correction)
            .with_accent_attach(base_attach)
            .with_text_like(base_text_like),
    );

    Ok(())
}
