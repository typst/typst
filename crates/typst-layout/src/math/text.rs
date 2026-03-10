use codex::styling::{MathStyle, to_style};
use ecow::EcoString;
use typst_library::diag::SourceResult;
use typst_library::foundations::{Resolve, StyleChain};
use typst_library::layout::{Abs, Axis, Size};
use typst_library::math::ir::{GlyphItem, MathProperties, TextItem};
use typst_library::math::{EquationElem, MathSize, style_dtls, style_flac};
use typst_library::text::{
    BottomEdge, BottomEdgeMetric, Font, TextElem, TopEdge, TopEdgeMetric,
};
use typst_syntax::{Span, is_newline};
use typst_utils::Get;
use unicode_math_class::MathClass;

use super::MathContext;
use super::fragment::{FrameFragment, GlyphFragment, MathFragment};
use super::run::MathFragmentsExt;

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
            TextElem::overhang.set(false),
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
    let (styles, text): (_, EcoString) =
        if item.text.chars().any(|c| try_dotless(c).is_some())
            && has_dtls_feat(ctx.font())
        {
            dtls = style_dtls();
            let variant = styles.get(EquationElem::variant);
            let bold = styles.get(EquationElem::bold);
            let italic = styles.get(EquationElem::italic);
            let text = item
                .text
                .chars()
                .flat_map(|mut c| {
                    if let Some(d) = try_dotless(c) {
                        c = d;
                    }
                    to_style(c, MathStyle::select(c, variant, bold, italic))
                })
                .collect();
            (styles.chain(&dtls), text)
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

/// The non-dotless version of a dotless character that can be used with the
/// `dtls` OpenType feature.
fn try_dotless(c: char) -> Option<char> {
    match c {
        'ı' | '𝚤' => Some('i'),
        'ȷ' | '𝚥' => Some('j'),
        _ => None,
    }
}
