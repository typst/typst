use std::f64::consts::SQRT_2;
use std::io;

use decorum::N64;

use super::*;
use crate::diag::Error;
use crate::layout::{BackgroundNode, BackgroundShape, FixedNode, ImageNode, PadNode};

/// `image`: An image.
pub fn image(ctx: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let path = args.expect::<Spanned<Str>>("path to image file")?;
    let width = args.named("width")?;
    let height = args.named("height")?;

    let full = ctx.make_path(&path.v);
    let id = ctx.images.load(&full).map_err(|err| {
        Error::boxed(path.span, match err.kind() {
            io::ErrorKind::NotFound => "file not found".into(),
            _ => format!("failed to load image ({})", err),
        })
    })?;

    Ok(Value::Template(Template::from_inline(move |_| ImageNode {
        id,
        width,
        height,
    })))
}

/// `rect`: A rectangle with optional content.
pub fn rect(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let width = args.named("width")?;
    let height = args.named("height")?;
    let fill = args.named("fill")?;
    let body = args.eat().unwrap_or_default();
    Ok(rect_impl(width, height, None, fill, body))
}

/// `square`: A square with optional content.
pub fn square(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let size = args.named::<Length>("size")?.map(Linear::from);
    let width = match size {
        Some(size) => Some(size),
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
    Value::Template(Template::from_inline(move |style| {
        let mut node = LayoutNode::new(FixedNode {
            width,
            height,
            aspect,
            child: body.to_stack(style).into(),
        });

        if let Some(fill) = fill {
            node = LayoutNode::new(BackgroundNode {
                shape: BackgroundShape::Rect,
                fill: Paint::Color(fill),
                child: node,
            });
        }

        node
    }))
}

/// `ellipse`: An ellipse with optional content.
pub fn ellipse(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let width = args.named("width")?;
    let height = args.named("height")?;
    let fill = args.named("fill")?;
    let body = args.eat().unwrap_or_default();
    Ok(ellipse_impl(width, height, None, fill, body))
}

/// `circle`: A circle with optional content.
pub fn circle(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
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
    Value::Template(Template::from_inline(move |style| {
        // This padding ratio ensures that the rectangular padded region fits
        // perfectly into the ellipse.
        const PAD: f64 = 0.5 - SQRT_2 / 4.0;

        let mut node = LayoutNode::new(FixedNode {
            width,
            height,
            aspect,
            child: LayoutNode::new(PadNode {
                padding: Sides::splat(Relative::new(PAD).into()),
                child: body.to_stack(style).into(),
            }),
        });

        if let Some(fill) = fill {
            node = LayoutNode::new(BackgroundNode {
                shape: BackgroundShape::Ellipse,
                fill: Paint::Color(fill),
                child: node,
            });
        }

        node
    }))
}
