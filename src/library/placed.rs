use super::prelude::*;

/// `place`: Place content at an absolute position.
pub fn place(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let aligns = args.find().unwrap_or(Spec::new(Some(Align::Left), None));
    let offset = Spec::new(args.named("dx")?, args.named("dy")?);
    let body: Template = args.expect("body")?;
    Ok(Value::Template(Template::from_block(move |style| {
        PlacedNode {
            child: body.pack(style).moved(offset).aligned(aligns),
        }
    })))
}

/// A node that places its child out-of-flow.
#[derive(Debug, Hash)]
pub struct PlacedNode {
    /// The node to be placed.
    pub child: PackedNode,
}

impl Layout for PlacedNode {
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
    ) -> Vec<Constrained<Rc<Frame>>> {
        let mut frames = self.child.layout(ctx, regions);
        for frame in frames.iter_mut() {
            Rc::make_mut(&mut frame.item).size = Size::zero();
        }
        frames
    }
}
