use std::hash::{Hash, Hasher};
use std::sync::Arc;

use hayro_syntax::page::Page;
use hayro_syntax::{LoadPdfError, Pdf};

use crate::foundations::Bytes;

/// A PDF document.
#[derive(Clone, Hash)]
pub struct PdfDocument(Arc<DocumentRepr>);

/// The internal representation of a `PdfDocument`.
struct DocumentRepr {
    pdf: Arc<Pdf>,
    data: Bytes,
}

impl PdfDocument {
    /// Loads a PDF document.
    #[comemo::memoize]
    #[typst_macros::time(name = "load pdf document")]
    pub fn new(data: Bytes) -> Result<PdfDocument, LoadPdfError> {
        let pdf = Arc::new(Pdf::new(Arc::new(data.clone()))?);
        Ok(Self(Arc::new(DocumentRepr { data, pdf })))
    }

    /// Returns the underlying PDF document.
    pub fn pdf(&self) -> &Arc<Pdf> {
        &self.0.pdf
    }

    /// Return the number of pages in the PDF.
    pub fn num_pages(&self) -> usize {
        self.0.pdf.pages().len()
    }
}

impl Hash for DocumentRepr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.data.hash(state);
    }
}

/// A specific page of a PDF acting as an image.
#[derive(Clone, Hash)]
pub struct PdfImage(Arc<ImageRepr>);

/// The internal representation of a `PdfImage`.
struct ImageRepr {
    document: PdfDocument,
    page_index: usize,
    width: f32,
    height: f32,
}

impl PdfImage {
    /// Creates a new PDF image.
    ///
    /// Returns `None` if the page index is not valid.
    #[comemo::memoize]
    pub fn new(document: PdfDocument, page_index: usize) -> Option<PdfImage> {
        let (width, height) = document.0.pdf.pages().get(page_index)?.render_dimensions();
        Some(Self(Arc::new(ImageRepr { document, page_index, width, height })))
    }

    /// Returns the underlying Typst PDF document.
    pub fn document(&self) -> &PdfDocument {
        &self.0.document
    }

    /// Returns the PDF page of the image.
    pub fn page(&self) -> &Page {
        &self.document().pdf().pages()[self.0.page_index]
    }

    /// Returns the width of the image.
    pub fn width(&self) -> f32 {
        self.0.width
    }

    /// Returns the height of the image.
    pub fn height(&self) -> f32 {
        self.0.height
    }

    /// Returns the page index of the image.
    pub fn page_index(&self) -> usize {
        self.0.page_index
    }
}

impl Hash for ImageRepr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.document.hash(state);
        self.page_index.hash(state);
    }
}
