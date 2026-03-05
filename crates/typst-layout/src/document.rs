use std::sync::Arc;

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
#[derive(Debug, Clone)]
pub struct PagedDocument {
    pages: EcoVec<Page>,
    info: DocumentInfo,
    introspector: Arc<PagedIntrospector>,
}

impl PagedDocument {
    /// Creates a new paged document from its parts.
    ///
    /// Internally builds the introspector.
    pub fn new(pages: EcoVec<Page>, info: DocumentInfo) -> Self {
        let introspector = PagedIntrospector::new(&pages);
        Self { pages, info, introspector: Arc::new(introspector) }
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
    pub fn introspector(&self) -> &Arc<PagedIntrospector> {
        &self.introspector
    }
}

impl Document for PagedDocument {
    fn info(&self) -> &DocumentInfo {
        &self.info
    }
}

impl Output for PagedDocument {
    fn introspector(&self) -> &dyn Introspector {
        self.introspector.as_ref()
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
