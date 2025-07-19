use crate::diag::LoadResult;
use crate::foundations::Bytes;
use crate::text::{FontStretch, FontStyle, FontVariant, FontWeight};
use crate::World;
use comemo::Tracked;
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
    pub fn new(data: Bytes, world: Tracked<dyn World + '_>) -> LoadResult<PdfDocument> {
        // TODO: Remove unwraps
        let pdf = Arc::new(Pdf::new(Arc::new(data.clone())).unwrap());
        let standard_fonts = get_standard_fonts(world.clone());

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
        // TODO: Don't allow loading if pdf-embedding feature is disabled.
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
fn get_standard_fonts(world: Tracked<dyn World + '_>) -> Arc<StandardFonts> {
    let book = world.book();

    let get_font = |name: &str, fallback_name: Option<&str>, variant: FontVariant| {
        book.select(name, variant)
            .or_else(|| {
                if let Some(fallback_name) = fallback_name {
                    book.select(fallback_name, variant)
                } else {
                    None
                }
            })
            .and_then(|i| world.font(i))
            .map(|font| font.data().clone())
    };

    let normal_variant = FontVariant::new(
        FontStyle::Normal,
        FontWeight::default(),
        FontStretch::default(),
    );
    let bold_variant =
        FontVariant::new(FontStyle::Normal, FontWeight::BOLD, FontStretch::default());
    let italic_variant = FontVariant::new(
        FontStyle::Italic,
        FontWeight::default(),
        FontStretch::default(),
    );
    let bold_italic_variant =
        FontVariant::new(FontStyle::Italic, FontWeight::BOLD, FontStretch::default());

    let helvetica = VariantFont {
        normal: get_font("helvetica", Some("liberation sans"), normal_variant),
        bold: get_font("helvetica", Some("liberation sans"), bold_variant),
        italic: get_font("helvetica", Some("liberation sans"), italic_variant),
        bold_italic: get_font("helvetica", Some("liberation sans"), bold_italic_variant),
    };

    let courier = VariantFont {
        normal: get_font("courier", Some("liberation mono"), normal_variant),
        bold: get_font("courier", Some("liberation mono"), bold_variant),
        italic: get_font("courier", Some("liberation mono"), italic_variant),
        bold_italic: get_font("courier", Some("liberation mono"), bold_italic_variant),
    };

    let times = VariantFont {
        normal: get_font("times", Some("liberation serif"), normal_variant),
        bold: get_font("times", Some("liberation serif"), bold_variant),
        italic: get_font("times", Some("liberation serif"), italic_variant),
        bold_italic: get_font("times", Some("liberation serif"), bold_italic_variant),
    };

    // TODO: Use Foxit fonts as fallback
    let symbol = get_font("symbol", None, normal_variant);
    let zapf_dingbats = get_font("zapf dingbats", None, normal_variant);

    Arc::new(StandardFonts { helvetica, courier, times, symbol, zapf_dingbats })
}

pub struct VariantFont {
    pub normal: Option<Bytes>,
    pub bold: Option<Bytes>,
    pub italic: Option<Bytes>,
    pub bold_italic: Option<Bytes>,
}

pub struct StandardFonts {
    pub helvetica: VariantFont,
    pub courier: VariantFont,
    pub times: VariantFont,
    pub symbol: Option<Bytes>,
    pub zapf_dingbats: Option<Bytes>,
}
