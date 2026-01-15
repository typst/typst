use typst_library::diag::SourceResult;
use typst_library::foundations::{Resolve, StyleChain};
use typst_library::layout::{Abs, Frame, FrameItem, Point, Size};
use typst_library::math::ir::{LineItem, MathProperties, Position};
use typst_library::text::TextElem;
use typst_library::visualize::{FixedStroke, Geometry};

use super::MathContext;
use super::fragment::FrameFragment;

/// Lays out a [`LineItem`].
#[typst_macros::time(name = "math line layout", span = props.span)]
pub fn layout_line(
    item: &LineItem,
    ctx: &mut MathContext,
    styles: StyleChain,
    props: &MathProperties,
) -> SourceResult<()> {
    let (extra_height, content, line_pos, content_pos, baseline, thickness, line_adjust);
    match item.position {
        Position::Below => {
            content = ctx.layout_into_fragment(&item.base, styles)?;

            let (font, size) = content.font(ctx, item.base.styles().unwrap_or(styles));
            let sep = font.math().underbar_extra_descender.at(size);
            thickness = font.math().underbar_rule_thickness.at(size);
            let gap = font.math().underbar_vertical_gap.at(size);
            extra_height = sep + thickness + gap;

            line_pos = Point::with_y(content.height() + gap + thickness / 2.0);
            content_pos = Point::zero();
            baseline = content.ascent();
            line_adjust = -content.italics_correction();
        }
        Position::Above => {
            content = ctx.layout_into_fragment(&item.base, styles)?;

            let (font, size) = content.font(ctx, item.base.styles().unwrap_or(styles));
            let sep = font.math().overbar_extra_ascender.at(size);
            thickness = font.math().overbar_rule_thickness.at(size);
            let gap = font.math().overbar_vertical_gap.at(size);
            extra_height = sep + thickness + gap;

            line_pos = Point::with_y(sep + thickness / 2.0);
            content_pos = Point::with_y(extra_height);
            baseline = content.ascent() + extra_height;
            line_adjust = Abs::zero();
        }
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

    let text_fill = styles.get_ref(TextElem::fill).as_decoration();
    let line = match styles.get_ref(TextElem::stroke) {
        Some(stroke) => Geometry::Rect(Size::new(line_width, thickness))
            .filled_and_stroked(
                text_fill.clone(),
                stroke.clone().resolve(styles).unwrap_or_default(),
            ),
        None => Geometry::Line(Point::with_x(line_width))
            .stroked(FixedStroke::from_pair(text_fill, thickness)),
    };
    frame.push(line_pos, FrameItem::Shape(line, props.span));

    ctx.push(
        FrameFragment::new(props, styles, frame)
            .with_italics_correction(content_italics_correction)
            .with_text_like(content_text_like),
    );
    Ok(())
}
