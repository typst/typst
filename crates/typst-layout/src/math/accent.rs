use typst_library::diag::SourceResult;
use typst_library::foundations::{Packed, StyleChain};
use typst_library::layout::{Em, Frame, Point, Size};
use typst_library::math::{Accent, AccentElem};

use super::{
    style_cramped, style_dtls, style_flac, FrameFragment, GlyphFragment, MathContext,
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
    // Try to replace the base glyph with its dotless variant.
    let dtls = style_dtls();
    let base_styles = if elem.dotless(styles) { styles.chain(&dtls) } else { styles };

    let cramped = style_cramped();
    let base = ctx.layout_into_fragment(&elem.base, base_styles.chain(&cramped))?;

    // Preserve class to preserve automatic spacing.
    let base_class = base.class();
    let base_attach = base.accent_attach();

    let width = elem.size(styles).relative_to(base.width());

    // Try to replace the accent glyph with its flattened variant.
    let flattened_base_height = scaled!(ctx, styles, flattened_accent_base_height);
    let flac = style_flac();
    let accent_styles =
        if base.ascent() > flattened_base_height { styles.chain(&flac) } else { styles };

    let Accent(c) = elem.accent;
    let mut glyph = GlyphFragment::new(ctx.font, accent_styles, c, elem.span());

    // Forcing the accent to be at least as large as the base makes it too
    // wide in many case.
    let short_fall = ACCENT_SHORT_FALL.at(glyph.text.size);
    glyph.stretch_horizontal(ctx, width, short_fall);
    let accent_attach = glyph.accent_attach;
    let accent = glyph.into_frame();

    // Descent is negative because the accent's ink bottom is above the
    // baseline. Therefore, the default gap is the accent's negated descent
    // minus the accent base height. Only if the base is very small, we need
    // a larger gap so that the accent doesn't move too low.
    let accent_base_height = scaled!(ctx, styles, accent_base_height);
    let gap = -accent.descent() - base.ascent().min(accent_base_height);
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
        FrameFragment::new(styles, frame)
            .with_class(base_class)
            .with_base_ascent(base_ascent)
            .with_italics_correction(base_italics_correction)
            .with_accent_attach(base_attach)
            .with_text_like(base_text_like),
    );

    Ok(())
}
