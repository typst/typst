use crate::layout::{LayoutRoot, PageElem};
use crate::meta::ManualPageCounter;
use crate::prelude::*;

/// The root element of a document and its metadata.
///
/// All documents are automatically wrapped in a `document` element. You cannot
/// create a document element yourself. This function is only used with
/// [set rules]($styling/#set-rules) to specify document metadata. Such a set
/// rule must appear before any of the document's contents.
///
/// ```example
/// #set document(title: "Hello")
///
/// This has no visible output, but
/// embeds metadata into the PDF!
/// ```
///
/// Note that metadata set with this function is not rendered within the
/// document. Instead, it is embedded in the compiled PDF file.
#[elem(Construct, LayoutRoot)]
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

impl Construct for DocumentElem {
    fn construct(_: &mut Vm, args: &mut Args) -> SourceResult<Content> {
        bail!(args.span, "can only be used in set rules")
    }
}

fn get_next_page(contents: &[Content], starting_idx: usize) -> Option<&PageElem> {
    // get the next content that is a PageElem given a starting idx.
    // returns None if `starting_idx` is already the last page
    for content in contents.iter().skip(starting_idx + 1) {
        let content =
            if let Some((elem, _)) = content.to_styled() { elem } else { content };
        if let Some(page) = content.to::<PageElem>() {
            return Some(page);
        }
    }
    None
}

impl LayoutRoot for DocumentElem {
    /// Layout the document into a sequence of frames, one per page.
    #[tracing::instrument(name = "DocumentElem::layout_root", skip_all)]
    fn layout_root(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Document> {
        tracing::info!("Document layout");

        let mut pages = vec![];
        let mut page_counter = ManualPageCounter::new();

        for (current_idx, mut child) in self.children().iter().enumerate() {
            let outer = styles;
            let mut styles = styles;
            if let Some((elem, local)) = child.to_styled() {
                styles = outer.chain(local);
                child = elem;
            }

            if let Some(page) = child.to::<PageElem>() {
                let mut page = page.clone();
                if let Some(next_page) = get_next_page(&self.children(), current_idx) {
                    if let Some(clear) = next_page.clear(styles).take() {
                        page.push_clear_to(Some(clear));
                    };
                };
                let fragment = page.layout(vt, styles, &mut page_counter)?;
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

cast! {
    Author,
    self => self.0.into_value(),
    v: EcoString => Self(vec![v]),
    v: Array => Self(v.into_iter().map(Value::cast).collect::<StrResult<_>>()?),
}
