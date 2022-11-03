use crate::layout::PageNode;
use crate::prelude::*;

/// A sequence of page runs.
#[derive(Hash)]
pub struct DocNode(pub StyleVec<PageNode>);

impl DocNode {
    /// Layout the document into a sequence of frames, one per page.
    pub fn layout(
        &self,
        world: Tracked<dyn World>,
        styles: StyleChain,
    ) -> SourceResult<Vec<Frame>> {
        let mut frames = vec![];
        for (page, map) in self.0.iter() {
            let number = 1 + frames.len();
            frames.extend(page.layout(world, number, map.chain(&styles))?);
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
