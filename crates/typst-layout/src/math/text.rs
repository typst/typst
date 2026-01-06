use ecow::EcoString;
use typst_library::diag::SourceResult;
use typst_library::foundations::{Resolve, StyleChain};
use typst_library::layout::{Abs, Axis, Size};
use typst_library::math::{
    GlyphItem, MathProperties, MathSize, TextItem, style_dtls, style_flac,
};
use typst_library::text::{
    BottomEdge, BottomEdgeMetric, Font, TextElem, TopEdge, TopEdgeMetric,
};
use typst_syntax::{Span, is_newline};
use typst_utils::Get;
use unicode_math_class::MathClass;
use unicode_segmentation::UnicodeSegmentation;

use crate::math::run::MathFragmentsExt;

use super::{FrameFragment, GlyphFragment, MathContext, MathFragment};

/// Lays out a [`TextItem`].
#[typst_macros::time(name = "math text layout", span = props.span)]
pub fn layout_text(
    item: &TextItem,
    ctx: &mut MathContext,
    styles: StyleChain,
    props: &MathProperties,
) -> SourceResult<()> {
    let text = &item.text;
    let span = props.span;
    let fragment = if text.contains(is_newline) {
        layout_text_lines(text.split(is_newline), span, ctx, styles, props)?
    } else {
        layout_inline_text(text, span, ctx, styles, props)?
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
    props: &MathProperties,
) -> SourceResult<FrameFragment> {
    let mut fragments = vec![];
    for (i, line) in lines.enumerate() {
        if i != 0 {
            fragments.push(MathFragment::Linebreak);
        }
        if !line.is_empty() {
            fragments.push(layout_inline_text(line, span, ctx, styles, props)?.into());
        }
    }
    let mut frame = fragments.into_frame(styles);
    let axis = ctx.font().math().axis_height.resolve(styles);
    frame.set_baseline(frame.height() / 2.0 + axis);
    Ok(FrameFragment::new(props, styles, frame))
}

/// Layout the given text string into a [`FrameFragment`] after styling all
/// characters for the math font (without auto-italics).
fn layout_inline_text(
    text: &str,
    span: Span,
    ctx: &mut MathContext,
    styles: StyleChain,
    props: &MathProperties,
) -> SourceResult<FrameFragment> {
    if text.chars().all(|c| c.is_ascii_digit() || c == '.') {
        // Small optimization for numbers. Note that this lays out slightly
        // differently to normal text and is worth re-evaluating in the future.
        let mut fragments = vec![];
        for c in text.chars() {
            // This won't panic as ASCII digits and '.' will never end up as
            // nothing after shaping.
            let glyph = GlyphFragment::new_char(ctx, styles, c, span).unwrap();
            fragments.push(glyph.into());
        }
        let frame = fragments.into_frame(styles);
        Ok(FrameFragment::new(props, styles, frame).with_text_like(true))
    } else {
        let local = [
            TextElem::top_edge.set(TopEdge::Metric(TopEdgeMetric::Bounds)),
            TextElem::bottom_edge.set(BottomEdge::Metric(BottomEdgeMetric::Bounds)),
        ]
        .map(|p| p.wrap());

        let styles = styles.chain(&local);
        let elem = TextElem::packed(text).spanned(span);

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

        Ok(FrameFragment::new(props, styles, frame).with_text_like(true))
    }
}

/// Layout a single character in the math font.
#[typst_macros::time(name = "math glyph layout", span = props.span)]
pub fn layout_glyph(
    item: &GlyphItem,
    ctx: &mut MathContext,
    styles: StyleChain,
    props: &MathProperties,
) -> SourceResult<()> {
    let flac;
    let styles = if item.flac.get() {
        flac = style_flac();
        styles.chain(&flac)
    } else {
        styles
    };

    let dtls;
    let (styles, text): (_, EcoString) = if item.dtls {
        if has_dtls_feat(ctx.font()) {
            dtls = style_dtls();
            (styles.chain(&dtls), item.text.clone())
        } else {
            // Font doesn't support dtls, so convert back to dotless char.
            let text = item
                .text
                .graphemes(true)
                .flat_map(|c| undo_dotless(c).unwrap_or(c).chars())
                .collect();
            (styles, text)
        }
    } else {
        (styles, item.text.clone())
    };

    if let Some(mut glyph) =
        GlyphFragment::new(ctx.engine.world, styles, &text, props.span)
    {
        glyph.class = props.class;

        if let Some(axis) = glyph.stretch_axis(ctx.engine)
            && let Some(stretch) = item.stretch.get().resolve(axis)
        {
            let relative_to_size = stretch.relative_to.unwrap_or_else(|| {
                if axis == Axis::Y
                    && glyph.class == MathClass::Large
                    && props.size == MathSize::Display
                {
                    glyph.item.font.math().display_operator_min_height.at(glyph.item.size)
                } else {
                    glyph.size.get(axis)
                }
            });

            glyph.stretch(
                ctx.engine,
                stretch.target.relative_to(relative_to_size),
                stretch.short_fall.at(stretch.font_size.unwrap_or(glyph.item.size)),
                axis,
            );

            if axis == Axis::Y {
                glyph.center_on_axis();
            }
        }

        if glyph.class == MathClass::Large {
            // TeXbook p 155. Large operators are always vertically centered on
            // the axis.
            glyph.center_on_axis();
        }

        ctx.push(glyph);
    }
    Ok(())
}

/// Whether the given font has the dtls OpenType feature.
fn has_dtls_feat(font: &Font) -> bool {
    font.ttf()
        .tables()
        .gsub
        .and_then(|gsub| gsub.features.index(ttf_parser::Tag::from_bytes(b"dtls")))
        .is_some()
}

/// If the input is a small i or j, returns its dotless glyph.
///
/// Whether the returned glyph is italic or not matches whether the user used
/// `upright()` to obtain the styled char or not.
fn undo_dotless(c: &str) -> Option<&str> {
    match c {
        "i" | "𝐢" | "𝗂" | "𝗶" => Some("ı"),
        "j" | "𝐣" | "𝗃" | "𝗷" => Some("ȷ"),
        "𝑖" | "𝒊" | "𝒾\u{FE00}" | "𝓲\u{FE00}" | "𝒾\u{FE01}" | "𝓲\u{FE01}" | "𝘪" | "𝙞"
        | "𝚒" | "𝖎" | "𝕚" | "𝔦" => Some("𝚤"),
        "𝑗" | "𝒋" | "𝒿\u{FE00}" | "𝓳\u{FE00}" | "𝒿\u{FE01}" | "𝓳\u{FE01}" | "𝘫" | "𝙟"
        | "𝚓" | "𝖏" | "𝕛" | "𝔧" => Some("𝚥"),
        _ => None,
    }
}
