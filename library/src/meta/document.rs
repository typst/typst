use typst::model::StyledNode;

use crate::layout::{LayoutRoot, PageNode};
use crate::prelude::*;

/// The root element of a document and its metadata.
///
/// All documents are automatically wrapped in a `document` element. The main
/// use of this element is to use it in `set` rules to specify document
/// metadata.
///
/// The metadata set with this function is not rendered within the document.
/// Instead, it is embedded in the compiled PDF file.
///
/// Display: Document
/// Category: meta
#[node(LayoutRoot)]
pub struct DocumentNode {
    /// The page runs.
    #[variadic]
    pub children: Vec<Content>,

    /// The document's title. This is often rendered as the title of the
    /// PDF viewer window.
    #[settable]
    #[default]
    pub title: Option<EcoString>,

    /// The document's authors.
    #[settable]
    #[default]
    pub author: Author,
}

impl LayoutRoot for DocumentNode {
    /// Layout the document into a sequence of frames, one per page.
    fn layout_root(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Document> {
        let mut pages = vec![];

        for mut child in self.children() {
            let map;
            let outer = styles;
            let mut styles = outer;
            if let Some(node) = child.to::<StyledNode>() {
                map = node.map();
                styles = outer.chain(&map);
                child = node.body();
            }

            if let Some(page) = child.to::<PageNode>() {
                let number = 1 + pages.len();
                let fragment = page.layout(vt, number, styles)?;
                pages.extend(fragment);
            } else if let Some(span) = child.span() {
                bail!(span, "unexpected document child");
            }
        }

        Ok(Document {
            pages,
            title: styles.get(Self::TITLE).clone(),
            author: styles.get(Self::AUTHOR).0.clone(),
        })
    }
}

/// A list of authors.
#[derive(Debug, Default, Clone, Hash)]
pub struct Author(Vec<EcoString>);

cast_from_value! {
    Author,
    v: EcoString => Self(vec![v]),
    v: Array => Self(v.into_iter().map(Value::cast).collect::<StrResult<_>>()?),
}

cast_to_value! {
    v: Author => v.0.into()
}
