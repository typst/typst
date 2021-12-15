use super::prelude::*;
use super::PageNode;

/// The root layout node, a document consisting of top-level page runs.
#[derive(Hash)]
pub struct DocumentNode(pub Vec<PageNode>);

impl DocumentNode {
    /// Layout the document into a sequence of frames, one per page.
    pub fn layout(&self, ctx: &mut LayoutContext) -> Vec<Rc<Frame>> {
        self.0.iter().flat_map(|node| node.layout(ctx)).collect()
    }
}

impl Debug for DocumentNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("Document ")?;
        f.debug_list().entries(&self.0).finish()
    }
}
