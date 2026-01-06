use typst_library::diag::SourceResult;
use typst_library::foundations::StyleChain;
use typst_library::layout::{Abs, Frame, FrameItem, Point, Size};
use typst_library::math::{LineItem, MathProperties};
use typst_library::text::TextElem;
use typst_library::visualize::{FixedStroke, Geometry};

use super::{FrameFragment, MathContext};

/// Lays out a [`LineItem`].
#[typst_macros::time(name = "math line layout", span = props.span)]
pub fn layout_line(
    item: &LineItem,
    ctx: &mut MathContext,
    styles: StyleChain,
    props: &MathProperties,
) -> SourceResult<()> {
    let (extra_height, content, line_pos, content_pos, baseline, bar_height, line_adjust);
    if item.under {
        content = ctx.layout_into_fragment(&item.base, styles)?;

        let (font, size) = content.font(ctx, item.base.styles().unwrap_or(styles));
        let sep = font.math().underbar_extra_descender.at(size);
        bar_height = font.math().underbar_rule_thickness.at(size);
        let gap = font.math().underbar_vertical_gap.at(size);
        extra_height = sep + bar_height + gap;

        line_pos = Point::with_y(content.height() + gap + bar_height / 2.0);
        content_pos = Point::zero();
        baseline = content.ascent();
        line_adjust = -content.italics_correction();
    } else {
        content = ctx.layout_into_fragment(&item.base, styles)?;

        let (font, size) = content.font(ctx, item.base.styles().unwrap_or(styles));
        let sep = font.math().overbar_extra_ascender.at(size);
        bar_height = font.math().overbar_rule_thickness.at(size);
        let gap = font.math().overbar_vertical_gap.at(size);
        extra_height = sep + bar_height + gap;

        line_pos = Point::with_y(sep + bar_height / 2.0);
        content_pos = Point::with_y(extra_height);
        baseline = content.ascent() + extra_height;
        line_adjust = Abs::zero();
    }

    let width = content.width();
    let height = content.height() + extra_height;
    let size = Size::new(width, height);
    let line_width = width + line_adjust;

    let content_text_like = content.is_text_like();
    let content_italics_correction = content.italics_correction();
    let mut frame = Frame::soft(size);
    frame.set_baseline(baseline);
    frame.push_frame(content_pos, content.into_frame());
    frame.push(
        line_pos,
        FrameItem::Shape(
            Geometry::Line(Point::with_x(line_width)).stroked(FixedStroke {
                paint: styles.get_ref(TextElem::fill).as_decoration(),
                thickness: bar_height,
                ..FixedStroke::default()
            }),
            props.span,
        ),
    );

    ctx.push(
        FrameFragment::new(props, styles, frame)
            .with_italics_correction(content_italics_correction)
            .with_text_like(content_text_like),
    );
    Ok(())
}
