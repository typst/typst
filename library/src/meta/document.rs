=use crate::layout::{LayoutRoot, PageElem};
use crate::prelude::*;

/// Use this element to define the root of your document and set metadata such as
/// the title and author. All other content in your document should be added as
/// children of the `DocumentElem` instance.
/// For example, to create a two-page document with a title and author, you could
/// define your `DocumentElem` like this:

 let document = document(
     title("My Document"),
     author("John Doe"),
     page(
         // content for first page
     ),
     page(
         // content for second page
     ),
 );


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
#[element(LayoutRoot)]
pub struct DocumentElem {
    /// The document's title. This is often rendered as the title of the
    /// PDF viewer window.
    pub title: Option<EcoString>,

    /// The document's authors.
    pub author: Author,

    /// The page runs.
    #[internal]
    #[variadic]
    pub children: Vec<Content>,
}

impl LayoutRoot for DocumentElem {
    /// Layout the document into a sequence of frames, one per page.
    fn layout_root(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Document> {
        let mut pages = vec![];

        for mut child in &self.children() {
            let outer = styles;
            let mut styles = styles;
            if let Some((elem, local)) = child.to_styled() {
                styles = outer.chain(local);
                child = elem;
            }

            if let Some(page) = child.to::<PageElem>() {
                let fragment = page.layout(vt, styles)?;
                pages.extend(fragment);
            } else {
                bail!(child.span(), "unexpected document child");
            }
        }

        Ok(Document {
            pages,
            title: self.title(styles),
            author: self.author(styles).0,
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
