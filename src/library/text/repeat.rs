use crate::library::prelude::*;

/// Fill space by repeating something horizontally.
#[derive(Debug, Hash)]
pub struct RepeatNode(pub LayoutNode);

#[node]
impl RepeatNode {
    fn construct(_: &mut Context, args: &mut Args) -> TypResult<Content> {
        Ok(Content::inline(Self(args.expect("body")?)))
    }
}

impl Layout for RepeatNode {
    fn layout(
        &self,
        ctx: &mut Context,
        regions: &Regions,
        styles: StyleChain,
    ) -> TypResult<Vec<Arc<Frame>>> {
        // The actual repeating happens directly in the paragraph.
        self.0.layout(ctx, regions, styles)
    }
}
