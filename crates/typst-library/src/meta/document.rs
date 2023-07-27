use crate::layout::{LayoutRoot, PageElem};
use crate::meta::ProvideElem;
use crate::prelude::*;
use ecow::EcoVec;
use std::collections::BTreeMap;

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
///
/// Display: Document
/// Category: meta
#[element(Construct, LayoutRoot)]
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

impl LayoutRoot for DocumentElem {
    /// Layout the document into a sequence of frames, one per page.
    #[tracing::instrument(name = "DocumentElem::layout_root", skip_all)]
    fn layout_root(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Document> {
        tracing::info!("Document layout");

        let mut pages = vec![];

        for mut child in &self.children() {
            let outer = styles;
            let mut styles = styles;
            if let Some((elem, local)) = child.to_styled() {
                styles = outer.chain(local);
                child = elem;
            }

            if let Some(page) = child.to::<PageElem>() {
                let number = NonZeroUsize::ONE.saturating_add(pages.len());
                let fragment = page.layout(vt, styles, number)?;
                pages.extend(fragment);
            } else {
                bail!(child.span(), "unexpected document child");
            }
        }

        // Get all provided metadata elements, filter out null keys, build up Map.
        let provided_metadata = vt
            .introspector
            .query(&Selector::Elem(ProvideElem::func(), None))
            .iter()
            .filter_map(|c| {
                c.field("key")
                    .map(|k| (k.cast().unwrap(), c.field("value").unwrap_or_default()))
            })
            .fold(BTreeMap::<EcoString, EcoVec<Value>>::new(), |mut acc, elem| {
                acc.entry(elem.0).or_default().push(elem.1);
                acc
            });

        Ok(Document {
            pages,
            title: self.title(styles),
            author: self.author(styles).0,
            provided_metadata,
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
