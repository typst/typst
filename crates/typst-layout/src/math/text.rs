use typst_library::diag::SourceResult;
use typst_library::foundations::{Resolve, StyleChain};
use typst_library::layout::{Abs, Size};
use typst_library::math::{EquationElem, GlyphItem, MathProperties, MathSize, TextItem};
use typst_library::text::{
    BottomEdge, BottomEdgeMetric, TextElem, TopEdge, TopEdgeMetric,
};
use typst_syntax::{Span, is_newline};
use unicode_math_class::MathClass;

use crate::math::run::MathFragmentsExt;
use crate::math::stretch::stretch_fragment;

use super::{FrameFragment, GlyphFragment, MathContext, MathFragment};

/// Lays out a [`TextItem`].
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
    Ok(FrameFragment::new(props, frame))
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
        Ok(FrameFragment::new(props, frame).with_text_like(true))
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

        Ok(FrameFragment::new(props, frame).with_text_like(true))
    }
}

/// Layout a single character in the math font.
pub fn layout_glyph(
    item: &GlyphItem,
    ctx: &mut MathContext,
    styles: StyleChain,
    props: &MathProperties,
) -> SourceResult<()> {
    if let Some(mut glyph) =
        GlyphFragment::new(ctx.engine.world, styles, &item.text, props.span)
    {
        if glyph.class == MathClass::Large {
            if styles.get(EquationElem::size) == MathSize::Display {
                let height = glyph
                    .item
                    .font
                    .math()
                    .display_operator_min_height
                    .at(glyph.item.size);
                glyph.stretch_vertical(ctx.engine, height, Abs::zero());
            };
            // TeXbook p 155. Large operators are always vertically centered on
            // the axis.
            glyph.center_on_axis();
        }
        glyph.class = props.class;
        if item.mid_stretched.is_some() {
            glyph.mid_stretched = Some(false);
        }
        let mut glyph = glyph.into();
        if let Some((stretch, _)) = item.stretch {
            stretch_fragment(ctx.engine, &mut glyph, None, None, stretch, Abs::zero());
        }
        ctx.push(glyph);
    }
    Ok(())
}
