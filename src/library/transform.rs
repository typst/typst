use super::prelude::*;
use super::{ShapeKind, ShapeNode};

/// `move`: Move content without affecting layout.
pub fn move_(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let x = args.named("x")?;
    let y = args.named("y")?;
    let body: Template = args.expect("body")?;

    Ok(Value::Template(Template::from_inline(move |style| {
        MoveNode {
            offset: Spec::new(x, y),
            child: ShapeNode {
                shape: ShapeKind::Rect,
                width: None,
                height: None,
                fill: None,
                child: Some(body.to_flow(style).pack()),
            },
        }
    })))
}

#[derive(Debug, Hash)]
struct MoveNode {
    offset: Spec<Option<Linear>>,
    child: ShapeNode,
}

impl InlineLevel for MoveNode {
    fn layout(&self, ctx: &mut LayoutContext, space: Length, base: Size) -> Frame {
        let offset = Point::new(
            self.offset.x.map(|x| x.resolve(base.w)).unwrap_or_default(),
            self.offset.y.map(|y| y.resolve(base.h)).unwrap_or_default(),
        );

        let mut frame = self.child.layout(ctx, space, base);
        for (point, _) in &mut frame.children {
            *point += offset;
        }

        frame
    }
}
