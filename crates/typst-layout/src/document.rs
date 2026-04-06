use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, OnceLock};

use crate::page_store::DiskPageStore;

use ecow::EcoVec;
use typst_library::diag::SourceResult;
use typst_library::engine::Engine;
use typst_library::foundations::{Content, Output, Smart, StyleChain, Target};
use typst_library::introspection::Introspector;
use typst_library::layout::Frame;
use typst_library::model::{Document, DocumentInfo, Numbering};
use typst_library::visualize::{Color, Paint};

use crate::PagedIntrospector;

/// A finished document with metadata and page frames.
///
/// The introspector is built lazily on first access. For documents that
/// don't use introspection features (no `query()`, `locate()`, `counter()`,
/// `state()`), the introspector may never be built, saving significant
/// memory (~200MB+ for large documents).
pub struct PagedDocument {
    pages: EcoVec<Page>,
    info: DocumentInfo,
    introspector: OnceLock<Arc<PagedIntrospector>>,
    /// Optional disk-backed page store for large documents.
    /// When set, pages may have been spilled to disk during layout.
    page_store: Option<Arc<DiskPageStore>>,
}

impl PagedDocument {
    /// Creates a new paged document from its parts.
    ///
    /// The introspector is built lazily on first access via `introspector()`.
    pub fn new(pages: EcoVec<Page>, info: DocumentInfo) -> Self {
        Self { pages, info, introspector: OnceLock::new(), page_store: None }
    }

    /// The document's finished pages.
    pub fn pages(&self) -> &[Page] {
        &self.pages
    }

    /// Details about the document, mutably.
    pub fn info_mut(&mut self) -> &mut DocumentInfo {
        &mut self.info
    }

    /// Provides the ability to execute queries on the document.
    ///
    /// On first call, this builds the introspector by scanning all page
    /// frames. Subsequent calls return the cached introspector.
    pub fn introspector(&self) -> &Arc<PagedIntrospector> {
        self.introspector.get_or_init(|| {
            Arc::new(PagedIntrospector::new(&self.pages))
        })
    }

    /// Drops page frames to free memory, keeping only the introspector
    /// and document info. After calling this, `pages()` returns an empty
    /// slice. This is used in the convergence loop where historical
    /// documents only need their introspector.
    pub fn drop_pages(&mut self) {
        // Ensure introspector is built before dropping pages,
        // since building it requires scanning the pages.
        self.introspector();
        self.pages = EcoVec::new();
    }

    /// Takes ownership of the pages, leaving the document with empty pages.
    /// This allows the caller to process and drop pages one at a time.
    pub fn take_pages(&mut self) -> EcoVec<Page> {
        std::mem::replace(&mut self.pages, EcoVec::new())
    }

    /// Creates a new document from pages and info, with a lazy introspector.
    /// Used when reconstructing a document from disk-backed page store.
    pub fn from_pages_and_info(pages: EcoVec<Page>, info: DocumentInfo) -> Self {
        Self::new(pages, info)
    }

    /// Returns the document info.
    pub fn info(&self) -> &DocumentInfo {
        &self.info
    }

    /// Sets a pre-built introspector on this document.
    /// Used when the introspector was built incrementally during layout.
    pub fn set_introspector(&mut self, introspector: PagedIntrospector) {
        let _ = self.introspector.set(Arc::new(introspector));
    }

    /// Sets the disk-backed page store for this document.
    pub fn set_page_store(&mut self, store: DiskPageStore) {
        self.page_store = Some(Arc::new(store));
    }

    /// Returns the disk-backed page store, if any.
    pub fn page_store(&self) -> Option<&Arc<DiskPageStore>> {
        self.page_store.as_ref()
    }

    /// Takes the disk-backed page store out of this document.
    pub fn take_page_store(&mut self) -> Option<Arc<DiskPageStore>> {
        self.page_store.take()
    }
}

impl Clone for PagedDocument {
    fn clone(&self) -> Self {
        Self {
            pages: self.pages.clone(),
            info: self.info.clone(),
            introspector: match self.introspector.get() {
                Some(intro) => {
                    let lock = OnceLock::new();
                    let _ = lock.set(intro.clone());
                    lock
                }
                None => OnceLock::new(),
            },
            page_store: self.page_store.clone(),
        }
    }
}

impl Debug for PagedDocument {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("PagedDocument")
            .field("pages", &self.pages.len())
            .field("info", &self.info)
            .field("introspector_built", &self.introspector.get().is_some())
            .finish()
    }
}

impl Hash for PagedDocument {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // The introspector is fully derived from the pages. Thus, there is
        // no need to hash it.
        self.pages.hash(state);
        self.info.hash(state);
    }
}

impl Document for PagedDocument {
    fn info(&self) -> &DocumentInfo {
        &self.info
    }
}

impl Output for PagedDocument {
    fn introspector(&self) -> &dyn Introspector {
        self.introspector().as_ref()
    }

    fn target() -> Target {
        Target::Paged
    }

    fn create(
        engine: &mut Engine,
        content: &Content,
        styles: StyleChain,
    ) -> SourceResult<Self> {
        crate::layout_document(engine, content, styles)
    }

    fn drop_pages(&mut self) {
        PagedDocument::drop_pages(self);
    }

    fn should_stream(&self) -> bool {
        // Only use streaming for documents large enough to benefit.
        // The threshold matches SPILL_THRESHOLD in pages/mod.rs.
        self.pages.len() > 100
    }
}

/// A finished page.
#[derive(Debug, Clone, Hash)]
pub struct Page {
    /// The frame that defines the page.
    pub frame: Frame,
    /// How the page is filled.
    ///
    /// - When `None`, the background is transparent.
    /// - When `Auto`, the background is transparent for PDF and white
    ///   for raster and SVG targets.
    ///
    /// Exporters should access the resolved value of this property through
    /// `fill_or_transparent()` or `fill_or_white()`.
    pub fill: Smart<Option<Paint>>,
    /// The page's numbering.
    pub numbering: Option<Numbering>,
    /// The page's supplement.
    pub supplement: Content,
    /// The logical page number (controlled by `counter(page)` and may thus not
    /// match the physical number).
    pub number: u64,
}

impl Page {
    /// Get the configured background or `None` if it is `Auto`.
    ///
    /// This is used in PDF export.
    pub fn fill_or_transparent(&self) -> Option<Paint> {
        self.fill.clone().unwrap_or(None)
    }

    /// Get the configured background or white if it is `Auto`.
    ///
    /// This is used in raster and SVG export.
    pub fn fill_or_white(&self) -> Option<Paint> {
        self.fill.clone().unwrap_or_else(|| Some(Color::WHITE.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paged_document_is_send_and_sync() {
        fn ensure_send_and_sync<T: Send + Sync>() {}
        ensure_send_and_sync::<PagedDocument>();
    }
}
