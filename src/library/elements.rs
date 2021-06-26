use std::f64::consts::SQRT_2;

use decorum::N64;

use super::*;
use crate::color::Color;
use crate::layout::{
    BackgroundNode, BackgroundShape, Fill, FixedNode, ImageNode, PadNode,
};

/// `image`: An image.
pub fn image(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let path = args.expect::<Spanned<String>>(ctx, "path to image file");
    let width = args.named(ctx, "width");
    let height = args.named(ctx, "height");

    let mut node = None;
    if let Some(path) = &path {
        if let Some((resolved, _)) = ctx.resolve(&path.v, path.span) {
            if let Some(id) = ctx.cache.image.load(ctx.loader, &resolved) {
                node = Some(ImageNode { id, width, height });
            } else {
                ctx.diag(error!(path.span, "failed to load image"));
            }
        }
    }

    Value::template(move |ctx| {
        if let Some(node) = node {
            ctx.push_into_par(node);
        }
    })
}

/// `rect`: A rectangle with optional content.
pub fn rect(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let width = args.named(ctx, "width");
    let height = args.named(ctx, "height");
    let fill = args.named(ctx, "fill");
    let body = args.eat(ctx).unwrap_or_default();
    rect_impl(width, height, None, fill, body)
}

/// `square`: A square with optional content.
pub fn square(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let length = args.named::<Length>(ctx, "length").map(Linear::from);
    let width = length.or_else(|| args.named(ctx, "width"));
    let height = width.is_none().then(|| args.named(ctx, "height")).flatten();
    let fill = args.named(ctx, "fill");
    let body = args.eat(ctx).unwrap_or_default();
    rect_impl(width, height, Some(N64::from(1.0)), fill, body)
}

fn rect_impl(
    width: Option<Linear>,
    height: Option<Linear>,
    aspect: Option<N64>,
    fill: Option<Color>,
    body: TemplateValue,
) -> Value {
    Value::template(move |ctx| {
        let mut stack = ctx.exec_template_stack(&body);
        stack.aspect = aspect;

        let fixed = FixedNode { width, height, child: stack.into() };

        if let Some(color) = fill {
            ctx.push_into_par(BackgroundNode {
                shape: BackgroundShape::Rect,
                fill: Fill::Color(color),
                child: fixed.into(),
            });
        } else {
            ctx.push_into_par(fixed);
        }
    })
}

/// `ellipse`: An ellipse with optional content.
pub fn ellipse(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let width = args.named(ctx, "width");
    let height = args.named(ctx, "height");
    let fill = args.named(ctx, "fill");
    let body = args.eat(ctx).unwrap_or_default();
    ellipse_impl(width, height, None, fill, body)
}

/// `circle`: A circle with optional content.
pub fn circle(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let radius = args.named::<Length>(ctx, "radius").map(|r| 2.0 * Linear::from(r));
    let width = radius.or_else(|| args.named(ctx, "width"));
    let height = width.is_none().then(|| args.named(ctx, "height")).flatten();
    let fill = args.named(ctx, "fill");
    let body = args.eat(ctx).unwrap_or_default();
    ellipse_impl(width, height, Some(N64::from(1.0)), fill, body)
}

fn ellipse_impl(
    width: Option<Linear>,
    height: Option<Linear>,
    aspect: Option<N64>,
    fill: Option<Color>,
    body: TemplateValue,
) -> Value {
    Value::template(move |ctx| {
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
            ctx.push_into_par(BackgroundNode {
                shape: BackgroundShape::Ellipse,
                fill: Fill::Color(color),
                child: fixed.into(),
            });
        } else {
            ctx.push_into_par(fixed);
        }
    })
}
