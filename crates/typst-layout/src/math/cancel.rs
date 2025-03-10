use comemo::Track;
use typst_library::diag::{At, SourceResult};
use typst_library::foundations::{Context, Packed, Smart, StyleChain};
use typst_library::layout::{Abs, Angle, Frame, FrameItem, Point, Rel, Size, Transform};
use typst_library::math::{CancelAngle, CancelElem};
use typst_library::text::TextElem;
use typst_library::visualize::{FixedStroke, Geometry};
use typst_syntax::Span;

use super::{FrameFragment, MathContext};

/// Lays out a [`CancelElem`].
#[typst_macros::time(name = "math.cancel", span = elem.span())]
pub fn layout_cancel(
    elem: &Packed<CancelElem>,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<()> {
    let body = ctx.layout_into_fragment(&elem.body, styles)?;

    // Preserve properties of body.
    let body_class = body.class();
    let body_italics = body.italics_correction();
    let body_attach = body.accent_attach();
    let body_text_like = body.is_text_like();

    let mut body = body.into_frame();
    let body_size = body.size();
    let span = elem.span();
    let length = elem.length(styles);

    let stroke = elem.stroke(styles).unwrap_or(FixedStroke {
        paint: TextElem::fill_in(styles).as_decoration(),
        ..Default::default()
    });

    let invert = elem.inverted(styles);
    let cross = elem.cross(styles);
    let angle = elem.angle(styles);

    let invert_first_line = !cross && invert;
    let first_line = draw_cancel_line(
        ctx,
        length,
        stroke.clone(),
        invert_first_line,
        &angle,
        body_size,
        styles,
        span,
    )?;

    // The origin of our line is the very middle of the element.
    let center = body_size.to_point() / 2.0;
    body.push_frame(center, first_line);

    if cross {
        // Draw the second line.
        let second_line =
            draw_cancel_line(ctx, length, stroke, true, &angle, body_size, styles, span)?;

        body.push_frame(center, second_line);
    }

    ctx.push(
        FrameFragment::new(styles, body)
            .with_class(body_class)
            .with_italics_correction(body_italics)
            .with_accent_attach(body_attach)
            .with_text_like(body_text_like),
    );

    Ok(())
}

/// Draws a cancel line.
#[allow(clippy::too_many_arguments)]
fn draw_cancel_line(
    ctx: &mut MathContext,
    length_scale: Rel<Abs>,
    stroke: FixedStroke,
    invert: bool,
    angle: &Smart<CancelAngle>,
    body_size: Size,
    styles: StyleChain,
    span: Span,
) -> SourceResult<Frame> {
    let default = default_angle(body_size);
    let mut angle = match angle {
        // Non specified angle defaults to the diagonal
        Smart::Auto => default,
        Smart::Custom(angle) => match angle {
            // This specifies the absolute angle w.r.t y-axis clockwise.
            CancelAngle::Angle(v) => *v,
            // This specifies a function that takes the default angle as input.
            CancelAngle::Func(func) => func
                .call(ctx.engine, Context::new(None, Some(styles)).track(), [default])?
                .cast()
                .at(span)?,
        },
    };

    // invert means flipping along the y-axis
    if invert {
        angle *= -1.0;
    }

    // same as above, the default length is the diagonal of the body box.
    let default_length = body_size.to_point().hypot();
    let length = length_scale.relative_to(default_length);

    // Draw a vertical line of length and rotate it by angle
    let start = Point::new(Abs::zero(), length / 2.0);
    let delta = Point::new(Abs::zero(), -length);

    let mut frame = Frame::soft(body_size);
    frame.push(start, FrameItem::Shape(Geometry::Line(delta).stroked(stroke), span));

    // Having the middle of the line at the origin is convenient here.
    frame.transform(Transform::rotate(angle));
    Ok(frame)
}

/// The default line angle for a body of the given size.
fn default_angle(body: Size) -> Angle {
    // The default cancel line is the diagonal.
    // We infer the default angle from
    // the diagonal w.r.t to the body box.
    //
    // The returned angle is in the range of [0, Pi/2]
    //
    // Note that the angle is computed w.r.t to the y-axis
    //
    //            B
    //           /|
    // diagonal / | height
    //         /  |
    //        /   |
    //       O ----
    //         width
    let (width, height) = (body.x, body.y);
    let default_angle = (width / height).atan(); // arctangent (in the range [0, Pi/2])
    Angle::rad(default_angle)
}
