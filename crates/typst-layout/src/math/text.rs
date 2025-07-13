use std::f64::consts::SQRT_2;

use codex::styling::{to_style, MathStyle};
use ecow::EcoString;
use typst_library::diag::SourceResult;
use typst_library::foundations::{Packed, StyleChain, SymbolElem};
use typst_library::layout::{Abs, Size};
use typst_library::math::{EquationElem, MathSize};
use typst_library::text::{
    BottomEdge, BottomEdgeMetric, TextElem, TopEdge, TopEdgeMetric,
};
use typst_syntax::{is_newline, Span};
use unicode_math_class::MathClass;
use unicode_segmentation::UnicodeSegmentation;

use super::{
    has_dtls_feat, style_dtls, FrameFragment, GlyphFragment, MathContext, MathFragment,
    MathRun,
};

/// Lays out a [`TextElem`].
pub fn layout_text(
    elem: &Packed<TextElem>,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<()> {
    let text = &elem.text;
    let span = elem.span();
    let fragment = if text.contains(is_newline) {
        layout_text_lines(text.split(is_newline), span, ctx, styles)?
    } else {
        layout_inline_text(text, span, ctx, styles)?
    };
    ctx.push(fragment);
    Ok(())
}

/// Layout multiple lines of text.
fn layout_text_lines<'a>(
    lines: impl Iterator<Item = &'a str>,
    span: Span,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<FrameFragment> {
    let mut fragments = vec![];
    for (i, line) in lines.enumerate() {
        if i != 0 {
            fragments.push(MathFragment::Linebreak);
        }
        if !line.is_empty() {
            fragments.push(layout_inline_text(line, span, ctx, styles)?.into());
        }
    }
    let mut frame = MathRun::new(fragments).into_frame(styles);
    let axis = scaled!(ctx, styles, axis_height);
    frame.set_baseline(frame.height() / 2.0 + axis);
    Ok(FrameFragment::new(styles, frame))
}

/// Layout the given text string into a [`FrameFragment`] after styling all
/// characters for the math font (without auto-italics).
fn layout_inline_text(
    text: &str,
    span: Span,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<FrameFragment> {
    let variant = styles.get(EquationElem::variant);
    let bold = styles.get(EquationElem::bold);
    // Disable auto-italic.
    let italic = styles.get(EquationElem::italic).or(Some(false));

    if text.chars().all(|c| c.is_ascii_digit() || c == '.') {
        // Small optimization for numbers. Note that this lays out slightly
        // differently to normal text and is worth re-evaluating in the future.
        let mut fragments = vec![];
        for unstyled_c in text.chars() {
            // This is fine as ascii digits and '.' can never end up as more
            // than a single char after styling.
            let style = MathStyle::select(unstyled_c, variant, bold, italic);
            let c = to_style(unstyled_c, style).next().unwrap();

            let glyph = GlyphFragment::new_char(ctx.font, styles, c, span)?;
            fragments.push(glyph.into());
        }
        let frame = MathRun::new(fragments).into_frame(styles);
        Ok(FrameFragment::new(styles, frame).with_text_like(true))
    } else {
        let local = [
            TextElem::top_edge.set(TopEdge::Metric(TopEdgeMetric::Bounds)),
            TextElem::bottom_edge.set(BottomEdge::Metric(BottomEdgeMetric::Bounds)),
        ]
        .map(|p| p.wrap());

        let styles = styles.chain(&local);
        let styled_text: EcoString = text
            .chars()
            .flat_map(|c| to_style(c, MathStyle::select(c, variant, bold, italic)))
            .collect();

        let spaced = styled_text.graphemes(true).nth(1).is_some();
        let elem = TextElem::packed(styled_text).spanned(span);

        // There isn't a natural width for a paragraph in a math environment;
        // because it will be placed somewhere probably not at the left margin
        // it will overflow. So emulate an `hbox` instead and allow the
        // paragraph to extend as far as needed.
        let frame = crate::inline::layout_inline(
            ctx.engine,
            &[(&elem, styles)],
            &mut ctx.locator.next(&span).split(),
            styles,
            Size::splat(Abs::inf()),
            false,
        )?
        .into_frame();

        Ok(FrameFragment::new(styles, frame)
            .with_class(MathClass::Alphabetic)
            .with_text_like(true)
            .with_spaced(spaced))
    }
}

/// Layout a single character in the math font with the correct styling applied
/// (includes auto-italics).
pub fn layout_symbol(
    elem: &Packed<SymbolElem>,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<()> {
    // Switch dotless char to normal when we have the dtls OpenType feature.
    // This should happen before the main styling pass.
    let dtls = style_dtls();
    let (unstyled_c, symbol_styles) = match try_dotless(elem.text) {
        Some(c) if has_dtls_feat(ctx.font) => (c, styles.chain(&dtls)),
        _ => (elem.text, styles),
    };

    let variant = styles.get(EquationElem::variant);
    let bold = styles.get(EquationElem::bold);
    let italic = styles.get(EquationElem::italic);

    let style = MathStyle::select(unstyled_c, variant, bold, italic);
    let text: EcoString = to_style(unstyled_c, style).collect();

    let fragment: MathFragment =
        match GlyphFragment::new(ctx.font, symbol_styles, &text, elem.span()) {
            Ok(mut glyph) => {
                adjust_glyph_layout(&mut glyph, ctx, styles);
                glyph.into()
            }
            Err(_) => {
                // Not in the math font, fallback to normal inline text layout.
                // TODO: Should replace this with proper fallback in [`GlyphFragment::new`].
                layout_inline_text(&text, elem.span(), ctx, styles)?.into()
            }
        };
    ctx.push(fragment);
    Ok(())
}

/// Centers large glyphs vertically on the axis, scaling them if in display
/// style.
fn adjust_glyph_layout(
    glyph: &mut GlyphFragment,
    ctx: &mut MathContext,
    styles: StyleChain,
) {
    if glyph.class == MathClass::Large {
        if styles.get(EquationElem::size) == MathSize::Display {
            let height = scaled!(ctx, styles, display_operator_min_height)
                .max(SQRT_2 * glyph.size.y);
            glyph.stretch_vertical(ctx, height);
        };
        // TeXbook p 155. Large operators are always vertically centered on the
        // axis.
        glyph.center_on_axis();
    }
}

/// The non-dotless version of a dotless character that can be used with the
/// `dtls` OpenType feature.
pub fn try_dotless(c: char) -> Option<char> {
    match c {
        'ı' => Some('i'),
        'ȷ' => Some('j'),
        _ => None,
    }
}
