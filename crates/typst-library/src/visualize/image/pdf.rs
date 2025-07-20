use std::hash::{Hash, Hasher};
use std::sync::Arc;

use hayro_syntax::page::Page;
use hayro_syntax::{LoadPdfError, Pdf};

use crate::foundations::Bytes;

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
    pub fn new(data: Bytes) -> Result<PdfDocument, LoadPdfError> {
        let pdf = Arc::new(Pdf::new(Arc::new(data.clone()))?);
        let standard_fonts = get_standard_fonts();

        Ok(Self(Arc::new(DocumentRepr { data, pdf, standard_fonts })))
    }

    /// Return the number of pages in the PDF.
    pub fn len(&self) -> usize {
        self.0.pdf.pages().len()
    }
}

struct ImageRepr {
    document: PdfDocument,
    page_index: usize,
    width: f32,
    height: f32,
}

impl Hash for ImageRepr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.document.hash(state);
        self.page_index.hash(state);
    }
}

/// A specific page of a PDF acting as an image.
#[derive(Clone, Hash)]
pub struct PdfImage(Arc<ImageRepr>);

impl PdfImage {
    /// Create a new PDF image. 
    /// 
    /// Returns `None` if the page index is not valid.
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

    /// Returns the PDF page of the image.
    pub fn page(&self) -> &Page {
        &self.pdf().pages()[self.0.page_index]
    }

    /// Returns the underlying PDF document.
    pub fn pdf(&self) -> &Arc<Pdf> {
        &self.0.document.0.pdf
    }

    /// Returns the width of the image.
    pub fn width(&self) -> f32 {
        self.0.width
    }

    /// Returns the embedded standard fonts of the image.
    pub fn standard_fonts(&self) -> &Arc<StandardFonts> {
        &self.0.document.0.standard_fonts
    }

    /// Returns the height of the image.
    pub fn height(&self) -> f32 {
        self.0.height
    }

    /// Returns the page index of the image.
    pub fn page_index(&self) -> usize {
        self.0.page_index
    }

    /// Returns the underlying Typst PDF document.
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

/// A PDF font with multiple variants.
pub struct VariantFont {
    /// The normal variant.
    pub normal: Bytes,
    /// The bold variant.
    pub bold: Bytes,
    /// The italic variant.
    pub italic: Bytes,
    /// The bold-italic variant.
    pub bold_italic: Bytes,
}

/// A structure holding the raw data of all PDF standard fonts.
pub struct StandardFonts {
    /// The data for the `Helvetica` font family.
    pub helvetica: VariantFont,
    /// The data for the `Courier` font family.
    pub courier: VariantFont,
    /// The data for the `Times` font family.
    pub times: VariantFont,
    /// The data for the `Symbol` font family.
    pub symbol: Bytes,
    /// The data for the `Zapf Dingbats` font family.
    pub zapf_dingbats: Bytes,
}
