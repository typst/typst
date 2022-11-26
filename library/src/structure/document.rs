use crate::layout::{LayoutRoot, PageNode};
use crate::prelude::*;

/// The root node of the model.
#[derive(Hash)]
pub struct DocumentNode(pub StyleVec<PageNode>);

#[node(LayoutRoot)]
impl DocumentNode {
    /// The document's title.
    #[property(referenced)]
    pub const TITLE: Option<EcoString> = None;

    /// The document's author.
    #[property(referenced)]
    pub const AUTHOR: Option<EcoString> = None;
}

impl LayoutRoot for DocumentNode {
    /// Layout the document into a sequence of frames, one per page.
    fn layout_root(
        &self,
        world: Tracked<dyn World>,
        styles: StyleChain,
    ) -> SourceResult<Document> {
        let mut pages = vec![];
        for (page, map) in self.0.iter() {
            let number = 1 + pages.len();
            pages.extend(page.layout(world, number, styles.chain(map))?);
        }

        Ok(Document {
            metadata: Metadata {
                title: styles.get(Self::TITLE).clone(),
                author: styles.get(Self::AUTHOR).clone(),
            },
            pages,
        })
    }
}

impl Debug for DocumentNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("Document ")?;
        self.0.fmt(f)
    }
}
