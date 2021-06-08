use std::f64::consts::SQRT_2;

use decorum::N64;

use super::*;
use crate::color::Color;
use crate::layout::{BackgroundNode, BackgroundShape, Fill, FixedNode, PadNode};

/// `rect`: A rectangle with optional content.
///
/// # Positional parameters
/// - Body: optional, of type `template`.
///
/// # Named parameters
/// - Width: `width`, of type `linear` relative to parent width.
/// - Height: `height`, of type `linear` relative to parent height.
/// - Fill color: `fill`, of type `color`.
///
/// # Return value
/// A template that inserts a rectangle and sets the body into it.
pub fn rect(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let width = args.eat_named(ctx, "width");
    let height = args.eat_named(ctx, "height");
    let fill = args.eat_named(ctx, "fill");
    let body = args.eat::<TemplateValue>(ctx).unwrap_or_default();
    rect_impl("rect", width, height, None, fill, body)
}

/// `square`: A square with optional content.
///
/// # Positional parameters
/// - Body: optional, of type `template`.
///
/// # Named parameters
/// - Side length: `length`, of type `length`.
/// - Width: `width`, of type `linear` relative to parent width.
/// - Height: `height`, of type `linear` relative to parent height.
/// - Fill color: `fill`, of type `color`.
///
/// Note that you can specify only one of `length`, `width` and `height`. The
/// width and height parameters exist so that you can size the square relative
/// to its parent's size, which isn't possible by setting the side length.
///
/// # Return value
/// A template that inserts a square and sets the body into it.
pub fn square(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let length = args.eat_named::<Length>(ctx, "length").map(Linear::from);
    let width = length.or_else(|| args.eat_named(ctx, "width"));
    let height = width.is_none().then(|| args.eat_named(ctx, "height")).flatten();
    let fill = args.eat_named(ctx, "fill");
    let body = args.eat::<TemplateValue>(ctx).unwrap_or_default();
    rect_impl("square", width, height, Some(N64::from(1.0)), fill, body)
}

fn rect_impl(
    name: &str,
    width: Option<Linear>,
    height: Option<Linear>,
    aspect: Option<N64>,
    fill: Option<Color>,
    body: TemplateValue,
) -> Value {
    Value::template(name, move |ctx| {
        let mut stack = ctx.exec_template_stack(&body);
        stack.aspect = aspect;

        let fixed = FixedNode { width, height, child: stack.into() };

        if let Some(color) = fill {
            ctx.push(BackgroundNode {
                shape: BackgroundShape::Rect,
                fill: Fill::Color(color),
                child: fixed.into(),
            });
        } else {
            ctx.push(fixed);
        }
    })
}

/// `ellipse`: An ellipse with optional content.
///
/// # Positional parameters
/// - Body: optional, of type `template`.
///
/// # Named parameters
/// - Width: `width`, of type `linear` relative to parent width.
/// - Height: `height`, of type `linear` relative to parent height.
/// - Fill color: `fill`, of type `color`.
///
/// # Return value
/// A template that inserts an ellipse and sets the body into it.
pub fn ellipse(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let width = args.eat_named(ctx, "width");
    let height = args.eat_named(ctx, "height");
    let fill = args.eat_named(ctx, "fill");
    let body = args.eat::<TemplateValue>(ctx).unwrap_or_default();
    ellipse_impl("ellipse", width, height, None, fill, body)
}

/// `circle`: A circle with optional content.
///
/// # Positional parameters
/// - Body: optional, of type `template`.
///
/// # Named parameters
/// - Radius: `radius`, of type `length`.
/// - Width: `width`, of type `linear` relative to parent width.
/// - Height: `height`, of type `linear` relative to parent height.
/// - Fill color: `fill`, of type `color`.
///
/// Note that you can specify only one of `radius`, `width` and `height`. The
/// width and height parameters exist so that you can size the circle relative
/// to its parent's size, which isn't possible by setting the radius.
///
/// # Return value
/// A template that inserts a circle and sets the body into it.
pub fn circle(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let radius = args.eat_named::<Length>(ctx, "radius").map(|r| 2.0 * Linear::from(r));
    let width = radius.or_else(|| args.eat_named(ctx, "width"));
    let height = width.is_none().then(|| args.eat_named(ctx, "height")).flatten();
    let fill = args.eat_named(ctx, "fill");
    let body = args.eat::<TemplateValue>(ctx).unwrap_or_default();
    ellipse_impl("circle", width, height, Some(N64::from(1.0)), fill, body)
}

fn ellipse_impl(
    name: &str,
    width: Option<Linear>,
    height: Option<Linear>,
    aspect: Option<N64>,
    fill: Option<Color>,
    body: TemplateValue,
) -> Value {
    Value::template(name, move |ctx| {
        // This padding ratio ensures that the rectangular padded region fits
        // perfectly into the ellipse.
        const PAD: f64 = 0.5 - SQRT_2 / 4.0;

        let mut stack = ctx.exec_template_stack(&body);
        stack.aspect = aspect;

        let fixed = FixedNode {
            width,
            height,
            child: PadNode {
                padding: Sides::splat(Relative::new(PAD).into()),
                child: stack.into(),
            }
            .into(),
        };

        if let Some(color) = fill {
            ctx.push(BackgroundNode {
                shape: BackgroundShape::Ellipse,
                fill: Fill::Color(color),
                child: fixed.into(),
            });
        } else {
            ctx.push(fixed);
        }
    })
}
