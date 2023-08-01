//! Exporting into PDF documents.

mod external_graphics_state;
mod font;
mod image;
mod outline;
mod page;

use std::cmp::Eq;
use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;

use ecow::EcoString;
use pdf_writer::types::Direction;
use pdf_writer::{Finish, Name, PdfWriter, Ref, TextStr};
use xmp_writer::{LangId, RenditionClass, XmpWriter};

use self::page::Page;
use crate::doc::{Document, Lang};
use crate::font::Font;
use crate::geom::{Abs, Dir, Em};
use crate::image::Image;
use crate::model::Introspector;

use external_graphics_state::ExternalGraphicsState;

/// Export a document into a PDF file.
///
/// Returns the raw bytes making up the PDF file.
#[tracing::instrument(skip_all)]
pub fn pdf(document: &Document) -> Vec<u8> {
    let mut ctx = PdfContext::new(document);
    page::construct_pages(&mut ctx, &document.pages);
    font::write_fonts(&mut ctx);
    image::write_images(&mut ctx);
    external_graphics_state::write_external_graphics_states(&mut ctx);
    page::write_page_tree(&mut ctx);
    write_catalog(&mut ctx);
    ctx.writer.finish()
}

/// Identifies the color space definitions.
const SRGB: Name<'static> = Name(b"srgb");
const D65_GRAY: Name<'static> = Name(b"d65gray");

/// Context for exporting a whole PDF document.
pub struct PdfContext<'a> {
    document: &'a Document,
    introspector: Introspector,
    writer: PdfWriter,
    pages: Vec<Page>,
    page_heights: Vec<f32>,
    alloc: Ref,
    page_tree_ref: Ref,
    font_refs: Vec<Ref>,
    image_refs: Vec<Ref>,
    ext_gs_refs: Vec<Ref>,
    page_refs: Vec<Ref>,
    font_map: Remapper<Font>,
    image_map: Remapper<Image>,
    ext_gs_map: Remapper<ExternalGraphicsState>,
    /// For each font a mapping from used glyphs to their text representation.
    /// May contain multiple chars in case of ligatures or similar things. The
    /// same glyph can have a different text representation within one document,
    /// then we just save the first one. The resulting strings are used for the
    /// PDF's /ToUnicode map for glyphs that don't have an entry in the font's
    /// cmap. This is important for copy-paste and searching.
    glyph_sets: HashMap<Font, BTreeMap<u16, EcoString>>,
    languages: HashMap<Lang, usize>,
}

impl<'a> PdfContext<'a> {
    fn new(document: &'a Document) -> Self {
        let mut alloc = Ref::new(1);
        let page_tree_ref = alloc.bump();
        Self {
            document,
            introspector: Introspector::new(&document.pages),
            writer: PdfWriter::new(),
            pages: vec![],
            page_heights: vec![],
            alloc,
            page_tree_ref,
            page_refs: vec![],
            font_refs: vec![],
            image_refs: vec![],
            ext_gs_refs: vec![],
            font_map: Remapper::new(),
            image_map: Remapper::new(),
            ext_gs_map: Remapper::new(),
            glyph_sets: HashMap::new(),
            languages: HashMap::new(),
        }
    }
}

/// Write the document catalog.
#[tracing::instrument(skip_all)]
fn write_catalog(ctx: &mut PdfContext) {
    let lang = ctx
        .languages
        .iter()
        .max_by_key(|(&lang, &count)| (count, lang))
        .map(|(&k, _)| k);

    let dir = if lang.map(Lang::dir) == Some(Dir::RTL) {
        Direction::R2L
    } else {
        Direction::L2R
    };

    // Write the outline tree.
    let outline_root_id = outline::write_outline(ctx);

    // Write the document information.
    let mut info = ctx.writer.document_info(ctx.alloc.bump());
    let mut xmp = XmpWriter::new();
    if let Some(title) = &ctx.document.title {
        info.title(TextStr(title));
        xmp.title([(None, title.as_str())]);
    }

    let authors = &ctx.document.author;
    if !authors.is_empty() {
        info.author(TextStr(&authors.join(", ")));
        xmp.creator(authors.iter().map(|s| s.as_str()));
    }
    info.creator(TextStr("Typst"));
    info.finish();
    xmp.creator_tool("Typst");
    xmp.num_pages(ctx.document.pages.len() as u32);
    xmp.format("application/pdf");
    xmp.language(ctx.languages.keys().map(|lang| LangId(lang.as_str())));
    xmp.rendition_class(RenditionClass::Proof);
    xmp.pdf_version("1.7");

    let xmp_buf = xmp.finish(None);
    let meta_ref = ctx.alloc.bump();
    let mut meta_stream = ctx.writer.stream(meta_ref, xmp_buf.as_bytes());
    meta_stream.pair(Name(b"Type"), Name(b"Metadata"));
    meta_stream.pair(Name(b"Subtype"), Name(b"XML"));
    meta_stream.finish();

    // Write the document catalog.
    let mut catalog = ctx.writer.catalog(ctx.alloc.bump());
    catalog.pages(ctx.page_tree_ref);
    catalog.viewer_preferences().direction(dir);
    catalog.pair(Name(b"Metadata"), meta_ref);

    if let Some(outline_root_id) = outline_root_id {
        catalog.outlines(outline_root_id);
    }

    if let Some(lang) = lang {
        catalog.lang(TextStr(lang.as_str()));
    }
}

/// Compress data with the DEFLATE algorithm.
#[tracing::instrument(skip_all)]
fn deflate(data: &[u8]) -> Vec<u8> {
    const COMPRESSION_LEVEL: u8 = 6;
    miniz_oxide::deflate::compress_to_vec_zlib(data, COMPRESSION_LEVEL)
}

/// Assigns new, consecutive PDF-internal indices to items.
struct Remapper<T> {
    /// Forwards from the items to the pdf indices.
    to_pdf: HashMap<T, usize>,
    /// Backwards from the pdf indices to the items.
    to_items: Vec<T>,
}

impl<T> Remapper<T>
where
    T: Eq + Hash + Clone,
{
    fn new() -> Self {
        Self { to_pdf: HashMap::new(), to_items: vec![] }
    }

    fn insert(&mut self, item: T) {
        let to_layout = &mut self.to_items;
        self.to_pdf.entry(item.clone()).or_insert_with(|| {
            let pdf_index = to_layout.len();
            to_layout.push(item);
            pdf_index
        });
    }

    fn map(&self, item: T) -> usize {
        self.to_pdf[&item]
    }

    fn pdf_indices<'a>(
        &'a self,
        refs: &'a [Ref],
    ) -> impl Iterator<Item = (Ref, usize)> + 'a {
        refs.iter().copied().zip(0..self.to_pdf.len())
    }

    fn items(&self) -> impl Iterator<Item = &T> + '_ {
        self.to_items.iter()
    }
}

/// Additional methods for [`Abs`].
trait AbsExt {
    /// Convert an to a number of points.
    fn to_f32(self) -> f32;
}

impl AbsExt for Abs {
    fn to_f32(self) -> f32 {
        self.to_pt() as f32
    }
}

/// Additional methods for [`Em`].
trait EmExt {
    /// Convert an em length to a number of PDF font units.
    fn to_font_units(self) -> f32;
}

impl EmExt for Em {
    fn to_font_units(self) -> f32 {
        1000.0 * self.get() as f32
    }
}

/// Additional methods for [`Ref`].
trait RefExt {
    /// Bump the reference up by one and return the previous one.
    fn bump(&mut self) -> Self;
}

impl RefExt for Ref {
    fn bump(&mut self) -> Self {
        let prev = *self;
        *self = Self::new(prev.get() + 1);
        prev
    }
}
