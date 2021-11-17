use super::prelude::*;

/// `move`: Move content without affecting layout.
pub fn move_(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let dx = args.named("dx")?;
    let dy = args.named("dy")?;
    let body: Template = args.expect("body")?;
    Ok(Value::Template(Template::from_inline(move |style| {
        body.pack(style).moved(dx, dy)
    })))
}

/// A node that moves its child without affecting layout.
#[derive(Debug, Hash)]
pub struct MoveNode {
    /// The node whose contents should be moved.
    pub child: PackedNode,
    /// How much to move the contents.
    pub offset: Spec<Option<Linear>>,
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
