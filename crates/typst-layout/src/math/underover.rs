use typst_library::diag::SourceResult;
use typst_library::foundations::{Content, Packed, StyleChain};
use typst_library::layout::{Abs, Em, FixedAlignment, Frame, FrameItem, Point, Size};
use typst_library::math::{
    OverbraceElem, OverbracketElem, OverlineElem, OverparenElem, OvershellElem,
    UnderbraceElem, UnderbracketElem, UnderlineElem, UnderparenElem, UndershellElem,
};
use typst_library::text::TextElem;
use typst_library::visualize::{FixedStroke, Geometry};
use typst_syntax::Span;

use super::{
    scaled_font_size, stack, style_cramped, style_for_subscript, style_for_superscript,
    FrameFragment, GlyphFragment, LeftRightAlternator, MathContext, MathRun,
};

const BRACE_GAP: Em = Em::new(0.25);
const BRACKET_GAP: Em = Em::new(0.25);
const PAREN_GAP: Em = Em::new(0.25);
const SHELL_GAP: Em = Em::new(0.25);

/// A marker to distinguish under- and overlines.
enum Position {
    Under,
    Over,
}

/// Lays out an [`UnderlineElem`].
#[typst_macros::time(name = "math.underline", span = elem.span())]
pub fn layout_underline(
    elem: &Packed<UnderlineElem>,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<()> {
    layout_underoverline(ctx, styles, elem.body(), elem.span(), Position::Under)
}

/// Lays out an [`OverlineElem`].
#[typst_macros::time(name = "math.overline", span = elem.span())]
pub fn layout_overline(
    elem: &Packed<OverlineElem>,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<()> {
    layout_underoverline(ctx, styles, elem.body(), elem.span(), Position::Over)
}

/// Lays out an [`UnderbraceElem`].
#[typst_macros::time(name = "math.underbrace", span = elem.span())]
pub fn layout_underbrace(
    elem: &Packed<UnderbraceElem>,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<()> {
    layout_underoverspreader(
        ctx,
        styles,
        elem.body(),
        &elem.annotation(styles),
        '⏟',
        BRACE_GAP,
        Position::Under,
        elem.span(),
    )
}

/// Lays out an [`OverbraceElem`].
#[typst_macros::time(name = "math.overbrace", span = elem.span())]
pub fn layout_overbrace(
    elem: &Packed<OverbraceElem>,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<()> {
    layout_underoverspreader(
        ctx,
        styles,
        elem.body(),
        &elem.annotation(styles),
        '⏞',
        BRACE_GAP,
        Position::Over,
        elem.span(),
    )
}

/// Lays out an [`UnderbracketElem`].
#[typst_macros::time(name = "math.underbracket", span = elem.span())]
pub fn layout_underbracket(
    elem: &Packed<UnderbracketElem>,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<()> {
    layout_underoverspreader(
        ctx,
        styles,
        elem.body(),
        &elem.annotation(styles),
        '⎵',
        BRACKET_GAP,
        Position::Under,
        elem.span(),
    )
}

/// Lays out an [`OverbracketElem`].
#[typst_macros::time(name = "math.overbracket", span = elem.span())]
pub fn layout_overbracket(
    elem: &Packed<OverbracketElem>,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<()> {
    layout_underoverspreader(
        ctx,
        styles,
        elem.body(),
        &elem.annotation(styles),
        '⎴',
        BRACKET_GAP,
        Position::Over,
        elem.span(),
    )
}

/// Lays out an [`UnderparenElem`].
#[typst_macros::time(name = "math.underparen", span = elem.span())]
pub fn layout_underparen(
    elem: &Packed<UnderparenElem>,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<()> {
    layout_underoverspreader(
        ctx,
        styles,
        elem.body(),
        &elem.annotation(styles),
        '⏝',
        PAREN_GAP,
        Position::Under,
        elem.span(),
    )
}

/// Lays out an [`OverparenElem`].
#[typst_macros::time(name = "math.overparen", span = elem.span())]
pub fn layout_overparen(
    elem: &Packed<OverparenElem>,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<()> {
    layout_underoverspreader(
        ctx,
        styles,
        elem.body(),
        &elem.annotation(styles),
        '⏜',
        PAREN_GAP,
        Position::Over,
        elem.span(),
    )
}

/// Lays out an [`UndershellElem`].
#[typst_macros::time(name = "math.undershell", span = elem.span())]
pub fn layout_undershell(
    elem: &Packed<UndershellElem>,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<()> {
    layout_underoverspreader(
        ctx,
        styles,
        elem.body(),
        &elem.annotation(styles),
        '⏡',
        SHELL_GAP,
        Position::Under,
        elem.span(),
    )
}

/// Lays out an [`OvershellElem`].
#[typst_macros::time(name = "math.overshell", span = elem.span())]
pub fn layout_overshell(
    elem: &Packed<OvershellElem>,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<()> {
    layout_underoverspreader(
        ctx,
        styles,
        elem.body(),
        &elem.annotation(styles),
        '⏠',
        SHELL_GAP,
        Position::Over,
        elem.span(),
    )
}

/// layout under- or overlined content.
fn layout_underoverline(
    ctx: &mut MathContext,
    styles: StyleChain,
    body: &Content,
    span: Span,
    position: Position,
) -> SourceResult<()> {
    let (extra_height, content, line_pos, content_pos, baseline, bar_height, line_adjust);
    match position {
        Position::Under => {
            let sep = scaled!(ctx, styles, underbar_extra_descender);
            bar_height = scaled!(ctx, styles, underbar_rule_thickness);
            let gap = scaled!(ctx, styles, underbar_vertical_gap);
            extra_height = sep + bar_height + gap;

            content = ctx.layout_into_fragment(body, styles)?;

            line_pos = Point::with_y(content.height() + gap + bar_height / 2.0);
            content_pos = Point::zero();
            baseline = content.ascent();
            line_adjust = -content.italics_correction();
        }
        Position::Over => {
            let sep = scaled!(ctx, styles, overbar_extra_ascender);
            bar_height = scaled!(ctx, styles, overbar_rule_thickness);
            let gap = scaled!(ctx, styles, overbar_vertical_gap);
            extra_height = sep + bar_height + gap;

            let cramped = style_cramped();
            content = ctx.layout_into_fragment(body, styles.chain(&cramped))?;

            line_pos = Point::with_y(sep + bar_height / 2.0);
            content_pos = Point::with_y(extra_height);
            baseline = content.ascent() + extra_height;
            line_adjust = Abs::zero();
        }
    }

    let width = content.width();
    let height = content.height() + extra_height;
    let size = Size::new(width, height);
    let line_width = width + line_adjust;

    let content_class = content.class();
    let content_is_text_like = content.is_text_like();
    let content_italics_correction = content.italics_correction();
    let mut frame = Frame::soft(size);
    frame.set_baseline(baseline);
    frame.push_frame(content_pos, content.into_frame());
    frame.push(
        line_pos,
        FrameItem::Shape(
            Geometry::Line(Point::with_x(line_width)).stroked(FixedStroke {
                paint: TextElem::fill_in(styles).as_decoration(),
                thickness: bar_height,
                ..FixedStroke::default()
            }),
            span,
        ),
    );

    ctx.push(
        FrameFragment::new(ctx, styles, frame)
            .with_class(content_class)
            .with_text_like(content_is_text_like)
            .with_italics_correction(content_italics_correction),
    );

    Ok(())
}

/// Layout an over- or underbrace-like object.
#[allow(clippy::too_many_arguments)]
fn layout_underoverspreader(
    ctx: &mut MathContext,
    styles: StyleChain,
    body: &Content,
    annotation: &Option<Content>,
    c: char,
    gap: Em,
    position: Position,
    span: Span,
) -> SourceResult<()> {
    let font_size = scaled_font_size(ctx, styles);
    let gap = gap.at(font_size);
    let body = ctx.layout_into_run(body, styles)?;
    let body_class = body.class();
    let body = body.into_fragment(ctx, styles);
    let glyph = GlyphFragment::new(ctx, styles, c, span);
    let stretched = glyph.stretch_horizontal(ctx, body.width(), Abs::zero());

    let mut rows = vec![];
    let baseline = match position {
        Position::Under => {
            rows.push(MathRun::new(vec![body]));
            rows.push(stretched.into());
            if let Some(annotation) = annotation {
                let under_style = style_for_subscript(styles);
                let annotation_styles = styles.chain(&under_style);
                rows.push(ctx.layout_into_run(annotation, annotation_styles)?);
            }
            0
        }
        Position::Over => {
            if let Some(annotation) = annotation {
                let over_style = style_for_superscript(styles);
                let annotation_styles = styles.chain(&over_style);
                rows.push(ctx.layout_into_run(annotation, annotation_styles)?);
            }
            rows.push(stretched.into());
            rows.push(MathRun::new(vec![body]));
            rows.len() - 1
        }
    };

    let frame = stack(
        rows,
        FixedAlignment::Center,
        gap,
        baseline,
        LeftRightAlternator::Right,
        None,
    );
    ctx.push(FrameFragment::new(ctx, styles, frame).with_class(body_class));

    Ok(())
}
