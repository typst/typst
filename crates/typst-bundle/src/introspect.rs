use std::fmt::{self, Debug, Formatter};
use std::num::NonZeroUsize;
use std::ops::Deref;
use std::sync::Arc;

use ecow::{EcoString, EcoVec};
use rustc_hash::{FxHashMap, FxHashSet};
use typst_html::HtmlIntrospector;
use typst_layout::PagedIntrospector;
use typst_library::diag::StrResult;
use typst_library::foundations::{Content, Label, Selector};
use typst_library::introspection::{
    DocumentPosition, ElementIntrospector, ElementIntrospectorBuilder, Introspector,
    Location,
};
use typst_library::model::{AssetElem, DocumentElem, LinkElem, Numbering};
use typst_syntax::VirtualPath;

use crate::{BundleDocument, Item};

/// An introspector implementation for bundles.
#[derive(Clone)]
pub struct BundleIntrospector {
    /// The paths of and introspectors for all bundle documents. Does not
    /// include assets.
    children: Vec<(VirtualPath, ChildIntrospector, Location)>,
    /// The underlying target-agnostic introspector used for most queries.
    /// The positions are off-by-one indices into `children`.
    elements: ElementIntrospector<Option<NonZeroUsize>>,
    /// Maps from element locations to assigned link anchors. This is used to
    /// support intra-doc links. Link anchors are local to the relevant
    /// document.
    anchors: FxHashMap<Location, EcoString>,
}

impl BundleIntrospector {
    /// Creates an introspector for a bundle.
    #[typst_macros::time(name = "introspect bundle")]
    pub(crate) fn new(items: &[Item]) -> BundleIntrospector {
        let mut builder = BundleIntrospectorBuilder::default();
        for item in items {
            builder.discover_item(item);
        }
        builder.finish()
    }

    /// Computes all locations that are referenced by intra-doc links of any
    /// kind and returns them organized by the document they are in.
    pub fn link_targets(&self) -> FxHashMap<&VirtualPath, FxHashSet<Location>> {
        let mut map: FxHashMap<&VirtualPath, FxHashSet<Location>> = FxHashMap::default();
        for target in LinkElem::find_destinations(self).chain(
            self.children
                .iter()
                .flat_map(|(_, child, _)| child.frame_link_targets())
                .copied(),
        ) {
            let Some(path) = self.path(target) else { continue };
            map.entry(path).or_default().insert(target);
        }
        map
    }

    /// Enriches an existing introspector with HTML link anchors, which were
    /// assigned to the DOM in a post-processing step.
    pub fn set_anchors(&mut self, anchors: FxHashMap<Location, EcoString>) {
        self.anchors = anchors;
    }

    /// Retrieves the child introspector for the given location.
    ///
    /// Returns `None` if the location is not within a document.
    fn child(&self, location: Location) -> Option<&ChildIntrospector> {
        let pos = *self.elements.position(location)?;
        let index = pos?.get() - 1;
        Some(&self.children[index].1)
    }
}

impl Introspector for BundleIntrospector {
    fn query(&self, selector: &Selector) -> EcoVec<Content> {
        self.elements.query(selector)
    }

    fn query_first(&self, selector: &Selector) -> Option<Content> {
        self.elements.query_first(selector)
    }

    fn query_unique(&self, selector: &Selector) -> StrResult<Content> {
        self.elements.query_unique(selector)
    }

    fn query_label(&self, label: Label) -> StrResult<&Content> {
        self.elements.query_label(label)
    }

    fn query_labelled(&self) -> EcoVec<Content> {
        self.elements.query_labelled()
    }

    fn query_count_before(&self, selector: &Selector, end: Location) -> usize {
        self.elements.query_count_before(selector, end)
    }

    fn label_count(&self, label: Label) -> usize {
        self.elements.label_count(label)
    }

    fn locator(&self, key: u128, base: Location) -> Option<Location> {
        self.elements.locator(key, base)
    }

    fn pages(&self, location: Location) -> Option<NonZeroUsize> {
        self.child(location)?.pages(location)
    }

    fn page(&self, location: Location) -> Option<NonZeroUsize> {
        self.child(location)?.page(location)
    }

    fn position(&self, location: Location) -> Option<DocumentPosition> {
        self.child(location)?.position(location)
    }

    fn page_numbering(&self, location: Location) -> Option<&Numbering> {
        self.child(location)?.page_numbering(location)
    }

    fn page_supplement(&self, location: Location) -> Option<&Content> {
        self.child(location)?.page_supplement(location)
    }

    fn anchor(&self, location: Location) -> Option<&EcoString> {
        self.anchors.get(&location)
    }

    fn document(&self, location: Location) -> Option<Location> {
        let index = *self.elements.position(location)?;
        if let Some(index) = index {
            return Some(self.children[index.get() - 1].2);
        }

        self.elements
            .get_by_loc(&location)?
            .to_packed::<DocumentElem>()
            .map(|doc| doc.location().unwrap())
    }

    fn path(&self, location: Location) -> Option<&VirtualPath> {
        // Check whether the location is that of an element within one of the
        // bundle documents.
        let index = *self.elements.position(location)?;
        if let Some(index) = index {
            return Some(&self.children[index.get() - 1].0);
        }

        // Check whether the location is that of a document or asset itself.
        let content = self.elements.get_by_loc(&location)?;
        if let Some(doc) = content.to_packed::<DocumentElem>() {
            Some(doc.path.as_ref())
        } else if let Some(asset) = content.to_packed::<AssetElem>() {
            Some(asset.path.as_ref())
        } else {
            None
        }
    }
}

impl Debug for BundleIntrospector {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad("BundleIntrospector(..)")
    }
}

/// An introspector for a bundle document.
#[derive(Clone)]
enum ChildIntrospector {
    Paged(Arc<PagedIntrospector>),
    Html(Arc<HtmlIntrospector>),
}

impl ChildIntrospector {
    /// Returns the locations that the underlying document links to via
    /// `FrameItem::Link`.
    pub fn frame_link_targets(&self) -> &FxHashSet<Location> {
        match self {
            Self::Paged(introspector) => introspector.frame_link_targets(),
            Self::Html(introspector) => introspector.frame_link_targets(),
        }
    }
}

impl Deref for ChildIntrospector {
    type Target = dyn Introspector;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Paged(introspector) => introspector.as_ref(),
            Self::Html(introspector) => introspector.as_ref(),
        }
    }
}

/// An introspector implementation for HTML documents.
#[derive(Default)]
struct BundleIntrospectorBuilder {
    subdocuments: Vec<(VirtualPath, ChildIntrospector, Location)>,
    elements: ElementIntrospectorBuilder<Option<NonZeroUsize>>,
}

impl BundleIntrospectorBuilder {
    /// Returns the resulting introspector.
    fn finish(self) -> BundleIntrospector {
        BundleIntrospector {
            children: self.subdocuments,
            elements: self.elements.finalize(),
            anchors: FxHashMap::default(),
        }
    }

    /// Discovers introspectibles in a bundle item.
    fn discover_item(&mut self, item: &Item) {
        match item {
            Item::Tag(tag) => self.elements.discover_tag(tag, None),
            Item::Asset(..) => {}
            Item::Document(path, doc, loc) => self.discover_document(path, doc, *loc),
        }
    }

    /// Discovers introspectibles in a bundle document.
    fn discover_document(
        &mut self,
        path: &VirtualPath,
        doc: &BundleDocument,
        loc: Location,
    ) {
        let pos = NonZeroUsize::new(1 + self.subdocuments.len());
        let subintrospector = match doc {
            BundleDocument::Paged(doc, _) => {
                self.elements
                    .discover_elements(doc.introspector().elements(), |_| pos);
                ChildIntrospector::Paged(doc.introspector().clone())
            }
            BundleDocument::Html(doc) => {
                self.elements
                    .discover_elements(doc.introspector().elements(), |_| pos);
                ChildIntrospector::Html(doc.introspector().clone())
            }
        };
        self.subdocuments.push((path.clone(), subintrospector, loc));
    }
}
