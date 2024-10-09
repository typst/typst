use comemo::Track;

use crate::diag::{At, SourceResult};
use crate::foundations::{cast, elem, Content, Context, Func, Packed, Smart, StyleChain};
use crate::layout::{
    Abs, Angle, Frame, FrameItem, Length, Point, Ratio, Rel, Size, Transform,
};
use crate::math::{scaled_font_size, FrameFragment, LayoutMath, MathContext};
use crate::syntax::Span;
use crate::text::TextElem;
use crate::visualize::{FixedStroke, Geometry, Stroke};

/// Displays a diagonal line over a part of an equation.
///
/// This is commonly used to show the elimination of a term.
///
/// # Example
/// ```example
/// >>> #set page(width: 140pt)
/// Here, we can simplify:
/// $ (a dot b dot cancel(x)) /
///     cancel(x) $
/// ```
#[elem(LayoutMath)]
pub struct CancelElem {
    /// The content over which the line should be placed.
    #[required]
    pub body: Content,

    /// The length of the line, relative to the length of the diagonal spanning
    /// the whole element being "cancelled". A value of `{100%}` would then have
    /// the line span precisely the element's diagonal.
    ///
    /// ```example
    /// >>> #set page(width: 140pt)
    /// $ a + cancel(x, length: #200%)
    ///     - cancel(x, length: #200%) $
    /// ```
    #[default(Rel::new(Ratio::one(), Abs::pt(3.0).into()))]
    pub length: Rel<Length>,

    /// Whether the cancel line should be inverted (flipped along the y-axis).
    /// For the default angle setting, inverted means the cancel line
    /// points to the top left instead of top right.
    ///
    /// ```example
    /// >>> #set page(width: 140pt)
    /// $ (a cancel((b + c), inverted: #true)) /
    ///     cancel(b + c, inverted: #true) $
    /// ```
    #[default(false)]
    pub inverted: bool,

    /// Whether two opposing cancel lines should be drawn, forming a cross over
    /// the element. Overrides `inverted`.
    ///
    /// ```example
    /// >>> #set page(width: 140pt)
    /// $ cancel(Pi, cross: #true) $
    /// ```
    #[default(false)]
    pub cross: bool,

    /// How much to rotate the cancel line.
    ///
    /// - If given an angle, the line is rotated by that angle clockwise with
    ///   respect to the y-axis.
    /// - If `{auto}`, the line assumes the default angle; that is, along the
    ///   rising diagonal of the content box.
    /// - If given a function `angle => angle`, the line is rotated, with
    ///   respect to the y-axis, by the angle returned by that function. The
    ///   function receives the default angle as its input.
    ///
    /// ```example
    /// >>> #set page(width: 140pt)
    /// $ cancel(Pi)
    ///   cancel(Pi, angle: #0deg)
    ///   cancel(Pi, angle: #45deg)
    ///   cancel(Pi, angle: #90deg)
    ///   cancel(1/(1+x), angle: #(a => a + 45deg))
    ///   cancel(1/(1+x), angle: #(a => a + 90deg)) $
    /// ```
    pub angle: Smart<CancelAngle>,

    /// How to [stroke]($stroke) the cancel line.
    ///
    /// ```example
    /// >>> #set page(width: 140pt)
    /// $ cancel(
    ///   sum x,
    ///   stroke: #(
    ///     paint: red,
    ///     thickness: 1.5pt,
    ///     dash: "dashed",
    ///   ),
    /// ) $
    /// ```
    #[resolve]
    #[fold]
    #[default(Stroke {
        // Default stroke has 0.5pt for better visuals.
        thickness: Smart::Custom(Abs::pt(0.5).into()),
        ..Default::default()
    })]
    pub stroke: Stroke,
}

impl LayoutMath for Packed<CancelElem> {
    #[typst_macros::time(name = "math.cancel", span = self.span())]
    fn layout_math(&self, ctx: &mut MathContext, styles: StyleChain) -> SourceResult<()> {
        let body = ctx.layout_into_fragment(self.body(), styles)?;
        // Preserve properties of body.
        let body_class = body.class();
        let body_italics = body.italics_correction();
        let body_attach = body.accent_attach();
        let body_text_like = body.is_text_like();

        let mut body = body.into_frame();
        let body_size = body.size();
        let span = self.span();
        let length = self.length(styles).at(scaled_font_size(ctx, styles));

        let stroke = self.stroke(styles).unwrap_or(FixedStroke {
            paint: TextElem::fill_in(styles).as_decoration(),
            ..Default::default()
        });

        let invert = self.inverted(styles);
        let cross = self.cross(styles);
        let angle = self.angle(styles);

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
            let second_line = draw_cancel_line(
                ctx, length, stroke, true, &angle, body_size, styles, span,
            )?;

            body.push_frame(center, second_line);
        }

        ctx.push(
            FrameFragment::new(ctx, styles, body)
                .with_class(body_class)
                .with_italics_correction(body_italics)
                .with_accent_attach(body_attach)
                .with_text_like(body_text_like),
        );

        Ok(())
    }
}

/// Defines the cancel line.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum CancelAngle {
    Angle(Angle),
    Func(Func),
}

cast! {
    CancelAngle,
    self => match self {
        Self::Angle(v) => v.into_value(),
        Self::Func(v) => v.into_value()
    },
    v: Angle => CancelAngle::Angle(v),
    v: Func => CancelAngle::Func(v),
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
