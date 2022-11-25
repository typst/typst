use crate::layout::{LayoutRoot, PageNode};
use crate::prelude::*;

/// A sequence of page runs.
#[derive(Hash)]
pub struct DocNode(pub StyleVec<PageNode>);

#[node(LayoutRoot)]
impl DocNode {}

impl LayoutRoot for DocNode {
    /// Layout the document into a sequence of frames, one per page.
    fn layout_root(
        &self,
        world: Tracked<dyn World>,
        styles: StyleChain,
    ) -> SourceResult<Vec<Frame>> {
        let mut frames = vec![];
        for (page, map) in self.0.iter() {
            let number = 1 + frames.len();
            frames.extend(page.layout(world, number, styles.chain(map))?);
        }
        Ok(frames)
    }
}

impl Debug for DocNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("Doc ")?;
        self.0.fmt(f)
    }
}
