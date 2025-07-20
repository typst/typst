use crate::diag::LoadResult;
use crate::foundations::Bytes;
use hayro_syntax::page::Page;
use hayro_syntax::Pdf;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

struct DocumentRepr {
    pdf: Arc<Pdf>,
    data: Bytes,
    standard_fonts: Arc<StandardFonts>,
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
        let standard_fonts = get_standard_fonts();

        Ok(Self(Arc::new(DocumentRepr { data, pdf, standard_fonts })))
    }

    pub fn len(&self) -> usize {
        self.0.pdf.pages().len()
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
    /// Create a new PDF image. Returns `None` if the page index is not valid.
    #[comemo::memoize]
    pub fn new(document: PdfDocument, page: usize) -> Option<PdfImage> {
        // TODO: Remove Unwrap
        let dimensions = document.0.pdf.pages().get(page)?.render_dimensions();

        Some(Self(Arc::new(ImageRepr {
            document,
            page_index: page,
            width: dimensions.0,
            height: dimensions.1,
        })))
    }

    pub fn page(&self) -> &Page {
        &self.0.document.0.pdf.pages()[self.0.page_index]
    }

    pub fn pdf(&self) -> &Arc<Pdf> {
        &self.0.document.0.pdf
    }

    pub fn width(&self) -> f32 {
        self.0.width
    }

    pub fn standard_fonts(&self) -> &Arc<StandardFonts> {
        &self.0.document.0.standard_fonts
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

    pub fn document(&self) -> &PdfDocument {
        &self.0.document
    }
}

#[comemo::memoize]
fn get_standard_fonts() -> Arc<StandardFonts> {
    let helvetica = VariantFont {
        normal: Bytes::new(typst_assets::pdf::SANS),
        bold: Bytes::new(typst_assets::pdf::SANS_BOLD),
        italic: Bytes::new(typst_assets::pdf::SANS_ITALIC),
        bold_italic: Bytes::new(typst_assets::pdf::SANS_BOLD_ITALIC),
    };

    let courier = VariantFont {
        normal: Bytes::new(typst_assets::pdf::FIXED),
        bold: Bytes::new(typst_assets::pdf::FIXED_BOLD),
        italic: Bytes::new(typst_assets::pdf::FIXED_ITALIC),
        bold_italic: Bytes::new(typst_assets::pdf::FIXED_BOLD_ITALIC),
    };

    let times = VariantFont {
        normal: Bytes::new(typst_assets::pdf::SERIF),
        bold: Bytes::new(typst_assets::pdf::SERIF_BOLD),
        italic: Bytes::new(typst_assets::pdf::SERIF_ITALIC),
        bold_italic: Bytes::new(typst_assets::pdf::SERIF_BOLD_ITALIC),
    };

    let symbol = Bytes::new(typst_assets::pdf::SYMBOL);
    let zapf_dingbats = Bytes::new(typst_assets::pdf::DING_BATS);

    Arc::new(StandardFonts { helvetica, courier, times, symbol, zapf_dingbats })
}

pub struct VariantFont {
    pub normal: Bytes,
    pub bold: Bytes,
    pub italic: Bytes,
    pub bold_italic: Bytes,
}

pub struct StandardFonts {
    pub helvetica: VariantFont,
    pub courier: VariantFont,
    pub times: VariantFont,
    pub symbol: Bytes,
    pub zapf_dingbats: Bytes,
}
