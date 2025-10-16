use typst_library::diag::SourceResult;
use typst_library::foundations::{Packed, StyleChain, SymbolElem};
use typst_library::layout::{Abs, Frame, FrameItem, Point, Size};
use typst_library::math::{EquationElem, MathSize, RootElem};
use typst_library::text::TextElem;
use typst_library::visualize::{FixedStroke, Geometry};

use super::{FrameFragment, MathContext, style_cramped};

/// Lays out a [`RootElem`].
///
/// TeXbook page 443, page 360
/// See also: <https://www.w3.org/TR/mathml-core/#radicals-msqrt-mroot>
#[typst_macros::time(name = "math.root", span = elem.span())]
pub fn layout_root(
    elem: &Packed<RootElem>,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<()> {
    let span = elem.span();

    // Layout radicand.
    let radicand = {
        let cramped = style_cramped();
        let styles = styles.chain(&cramped);
        let run = ctx.layout_into_run(&elem.radicand, styles)?;
        let multiline = run.is_multiline();
        let radicand = run.into_fragment(styles);
        if multiline {
            // Align the frame center line with the math axis.
            let (font, size) = radicand.font(ctx, styles);
            let axis = font.math().axis_height.at(size);
            let mut radicand = radicand.into_frame();
            radicand.set_baseline(radicand.height() / 2.0 + axis);
            radicand
        } else {
            radicand.into_frame()
        }
    };

    // Layout root symbol.
    let mut sqrt =
        ctx.layout_into_fragment(&SymbolElem::packed('√').spanned(span), styles)?;

    let (font, size) = sqrt.font(ctx, styles);
    let thickness = font.math().radical_rule_thickness.at(size);
    let extra_ascender = font.math().radical_extra_ascender.at(size);
    let kern_before = font.math().radical_kern_before_degree.at(size);
    let kern_after = font.math().radical_kern_after_degree.at(size);
    let raise_factor = font.math().radical_degree_bottom_raise_percent;
    let gap = match styles.get(EquationElem::size) {
        MathSize::Display => font.math().radical_display_style_vertical_gap,
        _ => font.math().radical_vertical_gap,
    }
    .at(size);

    let line = FrameItem::Shape(
        Geometry::Line(Point::with_x(radicand.width())).stroked(FixedStroke::from_pair(
            sqrt.fill()
                .unwrap_or_else(|| styles.get_ref(TextElem::fill).as_decoration()),
            thickness,
        )),
        span,
    );

    let target = radicand.height() + thickness + gap;
    sqrt.stretch_vertical(ctx, target, Abs::zero());
    let sqrt = sqrt.into_frame();

    // Layout the index.
    let sscript = EquationElem::size.set(MathSize::ScriptScript).wrap();
    let index = elem
        .index
        .get_ref(styles)
        .as_ref()
        .map(|elem| ctx.layout_into_frame(elem, styles.chain(&sscript)))
        .transpose()?;

    // TeXbook, page 443, item 11
    // Keep original gap, and then distribute any remaining free space
    // equally above and below.
    let gap = gap.max((sqrt.height() - thickness - radicand.height() + gap) / 2.0);

    let sqrt_ascent = radicand.ascent() + gap + thickness;
    let descent = sqrt.height() - sqrt_ascent;
    let inner_ascent = sqrt_ascent + extra_ascender;

    let mut sqrt_offset = Abs::zero();
    let mut shift_up = Abs::zero();
    let mut ascent = inner_ascent;

    if let Some(index) = &index {
        sqrt_offset = kern_before + index.width() + kern_after;
        // The formula below for how much raise the index by comes from
        // the TeXbook, page 360, in the definition of `\root`.
        // However, the `+ index.descent()` part is different from TeX.
        // Without it, descenders can collide with the surd, a rarity
        // in practice, but possible.  MS Word also adjusts index positions
        // for descenders.
        shift_up = raise_factor * (inner_ascent - descent) + index.descent();
        ascent.set_max(shift_up + index.ascent());
    }

    let sqrt_x = sqrt_offset.max(Abs::zero());
    let radicand_x = sqrt_x + sqrt.width();
    let radicand_y = ascent - radicand.ascent();
    let width = radicand_x + radicand.width();
    let size = Size::new(width, ascent + descent);

    // The extra "- thickness" comes from the fact that the sqrt is placed
    // in `push_frame` with respect to its top, not its baseline.
    let sqrt_pos = Point::new(sqrt_x, radicand_y - gap - thickness);
    let line_pos = Point::new(radicand_x, radicand_y - gap - (thickness / 2.0));
    let radicand_pos = Point::new(radicand_x, radicand_y);

    let mut frame = Frame::soft(size);
    frame.set_baseline(ascent);

    if let Some(index) = index {
        let index_x = -sqrt_offset.min(Abs::zero()) + kern_before;
        let index_pos = Point::new(index_x, ascent - index.ascent() - shift_up);
        frame.push_frame(index_pos, index);
    }

    frame.push_frame(sqrt_pos, sqrt);
    frame.push(line_pos, line);
    frame.push_frame(radicand_pos, radicand);
    ctx.push(FrameFragment::new(styles, frame));

    Ok(())
}
