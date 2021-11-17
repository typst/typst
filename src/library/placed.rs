use super::parse_aligns;
use super::prelude::*;

/// `place`: Place content at an absolute position.
pub fn place(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let Spec { x, y } = parse_aligns(args)?;
    let dx = args.named("dx")?;
    let dy = args.named("dy")?;
    let body: Template = args.expect("body")?;
    Ok(Value::Template(Template::from_block(move |style| {
        PlacedNode {
            child: body
                .pack(style)
                .moved(dx, dy)
                .aligned(Some(x.unwrap_or(Align::Left)), y),
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
