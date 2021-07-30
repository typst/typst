use std::f64::consts::SQRT_2;

use decorum::N64;

use super::*;
use crate::layout::{
    BackgroundNode, BackgroundShape, FixedNode, ImageNode, PadNode, Paint,
};

/// `image`: An image.
pub fn image(ctx: &mut EvalContext, args: &mut FuncArgs) -> TypResult<Value> {
    let path = args.expect::<Spanned<EcoString>>("path to image file")?;
    let width = args.named("width")?;
    let height = args.named("height")?;

    let file = ctx.resolve(&path.v, path.span)?;
    let node = match ctx.images.load(file) {
        Some(id) => ImageNode { id, width, height },
        None => bail!(args.file, path.span, "failed to load image"),
    };

    Ok(Value::template(move |ctx| ctx.push_into_par(node)))
}

/// `rect`: A rectangle with optional content.
pub fn rect(_: &mut EvalContext, args: &mut FuncArgs) -> TypResult<Value> {
    let width = args.named("width")?;
    let height = args.named("height")?;
    let fill = args.named("fill")?;
    let body = args.eat().unwrap_or_default();
    Ok(rect_impl(width, height, None, fill, body))
}

/// `square`: A square with optional content.
pub fn square(_: &mut EvalContext, args: &mut FuncArgs) -> TypResult<Value> {
    let length = args.named::<Length>("length")?.map(Linear::from);
    let width = match length {
        Some(length) => Some(length),
        None => args.named("width")?,
    };
    let height = match width {
        Some(_) => None,
        None => args.named("height")?,
    };
    let aspect = Some(N64::from(1.0));
    let fill = args.named("fill")?;
    let body = args.eat().unwrap_or_default();
    Ok(rect_impl(width, height, aspect, fill, body))
}

fn rect_impl(
    width: Option<Linear>,
    height: Option<Linear>,
    aspect: Option<N64>,
    fill: Option<Color>,
    body: Template,
) -> Value {
    Value::template(move |ctx| {
        let mut stack = ctx.exec_template_stack(&body);
        stack.aspect = aspect;

        let fixed = FixedNode { width, height, child: stack.into() };

        if let Some(fill) = fill {
            ctx.push_into_par(BackgroundNode {
                shape: BackgroundShape::Rect,
                fill: Paint::Color(fill),
                child: fixed.into(),
            });
        } else {
            ctx.push_into_par(fixed);
        }
    })
}

/// `ellipse`: An ellipse with optional content.
pub fn ellipse(_: &mut EvalContext, args: &mut FuncArgs) -> TypResult<Value> {
    let width = args.named("width")?;
    let height = args.named("height")?;
    let fill = args.named("fill")?;
    let body = args.eat().unwrap_or_default();
    Ok(ellipse_impl(width, height, None, fill, body))
}

/// `circle`: A circle with optional content.
pub fn circle(_: &mut EvalContext, args: &mut FuncArgs) -> TypResult<Value> {
    let diameter = args.named("radius")?.map(|r: Length| 2.0 * Linear::from(r));
    let width = match diameter {
        None => args.named("width")?,
        diameter => diameter,
    };
    let height = match width {
        None => args.named("height")?,
        width => width,
    };
    let aspect = Some(N64::from(1.0));
    let fill = args.named("fill")?;
    let body = args.eat().unwrap_or_default();
    Ok(ellipse_impl(width, height, aspect, fill, body))
}

fn ellipse_impl(
    width: Option<Linear>,
    height: Option<Linear>,
    aspect: Option<N64>,
    fill: Option<Color>,
    body: Template,
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

        if let Some(fill) = fill {
            ctx.push_into_par(BackgroundNode {
                shape: BackgroundShape::Ellipse,
                fill: Paint::Color(fill),
                child: fixed.into(),
            });
        } else {
            ctx.push_into_par(fixed);
        }
    })
}
