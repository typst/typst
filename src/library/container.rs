use super::prelude::*;
use super::{ShapeKind, ShapeNode};

/// `box`: Place content in a rectangular box.
pub fn box_(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let width = args.named("width")?;
    let height = args.named("height")?;
    let fill = args.named("fill")?;
    let body: Template = args.find().unwrap_or_default();
    Ok(Value::Template(Template::from_inline(move |style| {
        ShapeNode {
            shape: ShapeKind::Rect,
            width,
            height,
            fill: fill.map(Paint::Color),
            child: Some(body.to_flow(style).pack()),
        }
    })))
}

/// `block`: Place content in a block.
pub fn block(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let body: Template = args.expect("body")?;
    Ok(Value::Template(Template::from_block(move |style| {
        body.to_flow(style)
    })))
}
