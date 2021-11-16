use super::prelude::*;
use super::PageNode;

/// The root layout node, a document consisting of top-level page runs.
#[derive(Debug, Hash)]
pub struct DocumentNode {
    /// The page runs.
    pub pages: Vec<PageNode>,
}

impl DocumentNode {
    /// Layout the document into a sequence of frames, one per page.
    pub fn layout(&self, ctx: &mut LayoutContext) -> Vec<Rc<Frame>> {
        self.pages.iter().flat_map(|node| node.layout(ctx)).collect()
    }
}
