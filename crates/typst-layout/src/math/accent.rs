use typst_library::diag::SourceResult;
use typst_library::foundations::{Packed, StyleChain, SymbolElem};
use typst_library::layout::{Abs, Em, Frame, Point, Rel, Size};
use typst_library::math::{Accent, AccentElem};
use typst_syntax::Span;

use super::{
    FrameFragment, MathContext, MathFragment, style_cramped, style_dtls, style_flac,
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
    let top_accent = !elem.accent.is_bottom();

    // Try to replace the base glyph with its dotless variant.
    let dtls = style_dtls();
    let base_styles =
        if top_accent && elem.dotless.get(styles) { styles.chain(&dtls) } else { styles };

    let cramped = style_cramped();
    let base_styles = base_styles.chain(&cramped);
    let base = ctx.layout_into_fragment(&elem.base, base_styles)?;

    // Try to replace the accent glyph with its flattened variant.
    let (font, size) = base.font(ctx, base_styles);
    let flattened_base_height = font.math().flattened_accent_base_height.at(size);
    let flac = style_flac();
    let accent_styles = if top_accent && base.ascent() > flattened_base_height {
        styles.chain(&flac)
    } else {
        styles
    };

    // Preserve class to preserve automatic spacing.
    let base_class = base.class();
    let base_attach = base.accent_attach();
    let base_italics_correction = base.italics_correction();
    let base_text_like = base.is_text_like();
    let base_ascent = match &base {
        MathFragment::Frame(frame) => frame.base_ascent,
        _ => base.ascent(),
    };
    let base_descent = match &base {
        MathFragment::Frame(frame) => frame.base_descent,
        _ => base.descent(),
    };

    let frame = place_accent(
        ctx,
        base,
        base_styles,
        elem.accent,
        accent_styles,
        elem.size.resolve(styles),
        ACCENT_SHORT_FALL,
        false,
        elem.span(),
    )?;

    ctx.push(
        FrameFragment::new(styles, frame)
            .with_class(base_class)
            .with_base_ascent(base_ascent)
            .with_base_descent(base_descent)
            .with_italics_correction(base_italics_correction)
            .with_accent_attach(base_attach)
            .with_text_like(base_text_like),
    );

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn place_accent(
    ctx: &mut MathContext,
    base: MathFragment,
    base_styles: StyleChain,
    accent: Accent,
    accent_styles: StyleChain,
    stretch_width: Rel<Abs>,
    short_fall: Em,
    exact_frame_width: bool,
    span: Span,
) -> SourceResult<Frame> {
    let top_accent = !accent.is_bottom();
    let base_attach = base.accent_attach();
    let (font, size) = base.font(ctx, base_styles);

    let mut accent = ctx.layout_into_fragment(
        &SymbolElem::packed(accent.0).spanned(span),
        accent_styles,
    )?;

    // Forcing the accent to be at least as large as the base makes it too wide
    // in many cases.
    let stretch_width = stretch_width.relative_to(base.width());
    let short_fall = short_fall.at(size);
    accent.stretch_horizontal(ctx, stretch_width, short_fall);
    let accent_attach = accent.accent_attach().0;
    let accent = accent.into_frame();

    // Calculate the width of the final frame.
    let (width, base_x, accent_x) = {
        let base_attach = if top_accent { base_attach.0 } else { base_attach.1 };
        if !exact_frame_width {
            (base.width(), Abs::zero(), base_attach - accent_attach)
        } else {
            let pre_width = accent_attach - base_attach;
            let post_width =
                (accent.width() - accent_attach) - (base.width() - base_attach);
            let width =
                pre_width.max(Abs::zero()) + base.width() + post_width.max(Abs::zero());
            if pre_width < Abs::zero() {
                (width, Abs::zero(), -pre_width)
            } else {
                (width, pre_width, Abs::zero())
            }
        }
    };

    let (gap, accent_pos, base_pos) = if top_accent {
        // Descent is negative because the accent's ink bottom is above the
        // baseline. Therefore, the default gap is the accent's negated descent
        // minus the accent base height. Only if the base is very small, we
        // need a larger gap so that the accent doesn't move too low.
        let accent_base_height = font.math().accent_base_height.at(size);
        let gap = -accent.descent() - base.ascent().min(accent_base_height);
        let accent_pos = Point::with_x(accent_x);
        let base_pos = Point::new(base_x, accent.height() + gap);
        (gap, accent_pos, base_pos)
    } else {
        let gap = -accent.ascent();
        let accent_pos = Point::new(accent_x, base.height() + gap);
        let base_pos = Point::with_x(base_x);
        (gap, accent_pos, base_pos)
    };

    let size = Size::new(width, accent.height() + gap + base.height());
    let baseline = base_pos.y + base.ascent();

    let mut frame = Frame::soft(size);
    frame.set_baseline(baseline);
    frame.push_frame(accent_pos, accent);
    frame.push_frame(base_pos, base.into_frame());
    Ok(frame)
}
