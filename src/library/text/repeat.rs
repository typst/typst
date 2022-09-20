use crate::library::prelude::*;

/// A node that should be repeated to fill up a line.
#[derive(Debug, Hash)]
pub struct RepeatNode(pub LayoutNode);

#[node]
impl RepeatNode {
    fn construct(_: &mut Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Content::inline(Self(args.expect("body")?)))
    }
}

impl Layout for RepeatNode {
    fn layout(
        &self,
        world: &dyn World,
        regions: &Regions,
        styles: StyleChain,
    ) -> SourceResult<Vec<Frame>> {
        // The actual repeating happens directly in the paragraph.
        self.0.layout(world, regions, styles)
    }
}
