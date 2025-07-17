use crate::diag::LoadResult;
use crate::foundations::Bytes;
use hayro_syntax::pdf::Pdf;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use hayro_syntax::document::page::Page;

#[derive(Clone)]
struct DocumentRepr {
    pdf: Arc<Pdf>,
    data: Bytes,
    page_sizes: Vec<(f32, f32)>,
}

impl Hash for DocumentRepr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.data.hash(state);
    }
}

/// A PDF document.
#[derive(Clone, Hash)]
pub struct PdfDocument(Arc<DocumentRepr>);

impl PdfDocument {
    /// Load a PDF document.
    #[comemo::memoize]
    #[typst_macros::time(name = "load pdf document")]
    pub fn new(data: Bytes) -> LoadResult<PdfDocument> {
        // TODO: Remove unwraps
        let pdf = Arc::new(Pdf::new(Arc::new(data.clone())).unwrap());
        let pages = pdf.pages().unwrap();

        let page_sizes = pages.get().iter().map(|p| p.render_dimensions()).collect();

        Ok(Self(Arc::new(DocumentRepr { data, pdf, page_sizes })))
    }
}

struct ImageRepr {
    pub document: PdfDocument,
    pub page_index: usize,
    pub width: f32,
    pub height: f32,
}

impl Hash for ImageRepr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.document.hash(state);
        self.page_index.hash(state);
    }
}

/// A page of a PDF file.
#[derive(Clone, Hash)]
pub struct PdfImage(Arc<ImageRepr>);

impl PdfImage {
    #[comemo::memoize]
    pub fn new(document: PdfDocument, page: usize) -> LoadResult<PdfImage> {
        // TODO: Don't allow loading if pdf-embedding feature is disabled.
        // TODO: Remove Unwrap
        let dimensions = *(&document.0).page_sizes.get(page).unwrap();

        Ok(Self(Arc::new(ImageRepr {
            document,
            page_index: page,
            width: dimensions.0,
            height: dimensions.1,
        })))
    }

    pub fn with_page<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&Page) -> R,
    {
        let pages = self.0.document.0.pdf.pages().unwrap();
        f(&pages.get().get(self.0.page_index).unwrap())
    }

    pub fn width(&self) -> f32 {
        self.0.width
    }

    pub fn height(&self) -> f32 {
        self.0.height
    }

    pub fn data(&self) -> &Bytes {
        &self.0.document.0.data
    }

    pub fn page_index(&self) -> usize {
        self.0.page_index
    }
}
