use typst_library::foundations::{Content, Smart};
use typst_library::introspection::Introspector;
use typst_library::layout::Frame;
use typst_library::model::{DocumentInfo, Numbering};
use typst_library::visualize::{Color, Paint};

/// A finished document with metadata and page frames.
#[derive(Debug, Default, Clone)]
pub struct PagedDocument {
    /// The document's finished pages.
    pub pages: Vec<Page>,
    /// Details about the document.
    pub info: DocumentInfo,
    /// Provides the ability to execute queries on the document.
    pub introspector: Introspector,
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
