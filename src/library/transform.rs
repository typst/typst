use super::prelude::*;

/// `move`: Move content without affecting layout.
pub fn move_(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let x = args.named("x")?;
    let y = args.named("y")?;
    let body: Template = args.expect("body")?;

    Ok(Value::Template(Template::from_inline(move |style| {
        MoveNode {
            offset: Spec::new(x, y),
            child: body.pack(style),
        }
    })))
}

#[derive(Debug, Hash)]
struct MoveNode {
    offset: Spec<Option<Linear>>,
    child: PackedNode,
}

impl Layout for MoveNode {
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
    ) -> Vec<Constrained<Rc<Frame>>> {
        let mut frames = self.child.layout(ctx, regions);

        for (Constrained { item: frame, .. }, (_, base)) in
            frames.iter_mut().zip(regions.iter())
        {
            let offset = Point::new(
                self.offset.x.map(|x| x.resolve(base.w)).unwrap_or_default(),
                self.offset.y.map(|y| y.resolve(base.h)).unwrap_or_default(),
            );

            for (point, _) in &mut Rc::make_mut(frame).elements {
                *point += offset;
            }
        }

        frames
    }
}
