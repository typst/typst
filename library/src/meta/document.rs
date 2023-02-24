use crate::layout::{LayoutRoot, PageNode};
use crate::prelude::*;

/// # Document
/// The root element of a document and its metadata.
///
/// All documents are automatically wrapped in a `document` element. The main
/// use of this element is to use it in `set` rules to specify document
/// metadata.
///
/// The metadata set with this function is not rendered within the document.
/// Instead, it is embedded in the compiled PDF file.
///
/// ## Category
/// meta
#[func]
#[capable(LayoutRoot)]
#[derive(Hash)]
pub struct DocumentNode(pub StyleVec<PageNode>);

#[node]
impl DocumentNode {
    /// The document's title. This is often rendered as the title of the
    /// PDF viewer window.
    #[property(referenced)]
    pub const TITLE: Option<EcoString> = None;

    /// The document's authors.
    #[property(referenced)]
    pub const AUTHOR: Author = Author(vec![]);
}

impl LayoutRoot for DocumentNode {
    /// Layout the document into a sequence of frames, one per page.
    fn layout_root(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Document> {
        let mut pages = vec![];
        for (page, map) in self.0.iter() {
            let number = 1 + pages.len();
            let fragment = page.layout(vt, number, styles.chain(map))?;
            pages.extend(fragment);
        }

        Ok(Document {
            pages,
            title: styles.get(Self::TITLE).clone(),
            author: styles.get(Self::AUTHOR).0.clone(),
        })
    }
}

impl Debug for DocumentNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("Document ")?;
        self.0.fmt(f)
    }
}

/// A list of authors.
#[derive(Debug, Clone, Hash)]
pub struct Author(Vec<EcoString>);

castable! {
    Author,
    v: EcoString => Self(vec![v]),
    v: Array => Self(v.into_iter().map(Value::cast).collect::<StrResult<_>>()?),
}
