//! Hiding of nodes without affecting layout.

use super::prelude::*;

/// Hide a node without affecting layout.
#[derive(Debug, Hash)]
pub struct HideNode(pub LayoutNode);

#[class]
impl HideNode {
    fn construct(_: &mut EvalContext, args: &mut Args) -> TypResult<Template> {
        Ok(Template::inline(Self(args.expect("body")?)))
    }
}

impl Layout for HideNode {
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
        styles: StyleChain,
    ) -> Vec<Constrained<Arc<Frame>>> {
        let mut frames = self.0.layout(ctx, regions, styles);

        // Clear the frames.
        for Constrained { item: frame, .. } in &mut frames {
            *frame = Arc::new(Frame { elements: vec![], ..**frame });
        }

        frames
    }
}
