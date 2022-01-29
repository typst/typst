//! Hiding of nodes without affecting layout.

use super::prelude::*;

/// A node that hides its child without affecting layout.
#[derive(Debug, Hash)]
pub struct HideNode(pub PackedNode);

#[class]
impl HideNode {
    fn construct(_: &mut EvalContext, args: &mut Args) -> TypResult<Node> {
        Ok(Node::inline(Self(args.expect("body")?)))
    }
}

impl Layout for HideNode {
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
        styles: StyleChain,
    ) -> Vec<Constrained<Rc<Frame>>> {
        let mut frames = self.0.layout(ctx, regions, styles);

        // Clear the frames.
        for Constrained { item: frame, .. } in &mut frames {
            *frame = Rc::new(Frame { elements: vec![], ..**frame });
        }

        frames
    }
}
