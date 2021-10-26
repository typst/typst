use std::io;

use super::*;
use crate::diag::Error;
use crate::layout::{ImageNode, ShapeKind, ShapeNode};

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
    let body = args.find();
    Ok(shape_impl(ShapeKind::Rect, width, height, fill, body))
}

/// `square`: A square with optional content.
pub fn square(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let size = args.named::<Length>("size")?.map(Linear::from);
    let width = match size {
        None => args.named("width")?,
        size => size,
    };
    let height = match size {
        None => args.named("height")?,
        size => size,
    };
    let fill = args.named("fill")?;
    let body = args.find();
    Ok(shape_impl(ShapeKind::Square, width, height, fill, body))
}

/// `ellipse`: An ellipse with optional content.
pub fn ellipse(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let width = args.named("width")?;
    let height = args.named("height")?;
    let fill = args.named("fill")?;
    let body = args.find();
    Ok(shape_impl(ShapeKind::Ellipse, width, height, fill, body))
}

/// `circle`: A circle with optional content.
pub fn circle(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let diameter = args.named("radius")?.map(|r: Length| 2.0 * Linear::from(r));
    let width = match diameter {
        None => args.named("width")?,
        diameter => diameter,
    };
    let height = match diameter {
        None => args.named("height")?,
        diameter => diameter,
    };
    let fill = args.named("fill")?;
    let body = args.find();
    Ok(shape_impl(ShapeKind::Circle, width, height, fill, body))
}

fn shape_impl(
    shape: ShapeKind,
    mut width: Option<Linear>,
    mut height: Option<Linear>,
    fill: Option<Color>,
    body: Option<Template>,
) -> Value {
    // Set default shape size if there's no body.
    if body.is_none() {
        let v = Length::pt(30.0).into();
        height.get_or_insert(v);
        width.get_or_insert(match shape {
            ShapeKind::Square | ShapeKind::Circle => v,
            ShapeKind::Rect | ShapeKind::Ellipse => 1.5 * v,
        });
    }

    Value::Template(Template::from_inline(move |style| ShapeNode {
        shape,
        width,
        height,
        fill: Some(Paint::Color(
            fill.unwrap_or(Color::Rgba(RgbaColor::new(175, 175, 175, 255))),
        )),
        child: body.as_ref().map(|template| template.to_stack(style).pack()),
    }))
}
