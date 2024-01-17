//! Exporting into PDF documents.

mod color;
mod extg;
mod font;
mod gradient;
mod image;
mod outline;
mod page;
mod pattern;

use std::cmp::Eq;
use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;
use std::sync::Arc;

use base64::Engine;
use ecow::{eco_format, EcoString};
use pdf_writer::types::Direction;
use pdf_writer::{Finish, Name, Pdf, Ref, TextStr};
use typst::foundations::Datetime;
use typst::layout::{Abs, Dir, Em, Transform};
use typst::model::Document;
use typst::text::{Font, Lang};
use typst::util::Deferred;
use typst::visualize::Image;
use xmp_writer::{DateTime, LangId, RenditionClass, Timezone, XmpWriter};

use crate::color::ColorSpaces;
use crate::extg::ExtGState;
use crate::gradient::PdfGradient;
use crate::image::EncodedImage;
use crate::page::EncodedPage;
use crate::pattern::PdfPattern;

/// Export a document into a PDF file.
///
/// Returns the raw bytes making up the PDF file.
///
/// The `ident` parameter shall be a string that uniquely and stably identifies
/// the document. It should not change between compilations of the same
/// document. Its hash will be used to create a PDF document identifier (the
/// identifier itself is not leaked). If `ident` is `None`, a hash of the
/// document is used instead (which means that it _will_ change across
/// compilations).
///
/// The `timestamp`, if given, is expected to be the creation date of the
/// document as a UTC datetime. It will only be used if `set document(date: ..)`
/// is `auto`.
#[typst_macros::time(name = "pdf")]
pub fn pdf(
    document: &Document,
    ident: Option<&str>,
    timestamp: Option<Datetime>,
) -> Vec<u8> {
    let mut ctx = PdfContext::new(document);
    page::construct_pages(&mut ctx, &document.pages);
    font::write_fonts(&mut ctx);
    image::write_images(&mut ctx);
    gradient::write_gradients(&mut ctx);
    extg::write_external_graphics_states(&mut ctx);
    pattern::write_patterns(&mut ctx);
    page::write_page_tree(&mut ctx);
    write_catalog(&mut ctx, ident, timestamp);
    ctx.pdf.finish()
}

/// Context for exporting a whole PDF document.
struct PdfContext<'a> {
    /// The document that we're currently exporting.
    document: &'a Document,
    /// The writer we are writing the PDF into.
    pdf: Pdf,
    /// Content of exported pages.
    pages: Vec<EncodedPage>,
    /// For each font a mapping from used glyphs to their text representation.
    /// May contain multiple chars in case of ligatures or similar things. The
    /// same glyph can have a different text representation within one document,
    /// then we just save the first one. The resulting strings are used for the
    /// PDF's /ToUnicode map for glyphs that don't have an entry in the font's
    /// cmap. This is important for copy-paste and searching.
    glyph_sets: HashMap<Font, BTreeMap<u16, EcoString>>,
    /// The number of glyphs for all referenced languages in the document.
    /// We keep track of this to determine the main document language.
    languages: HashMap<Lang, usize>,

    /// Allocator for indirect reference IDs.
    alloc: Ref,
    /// The ID of the page tree.
    page_tree_ref: Ref,
    /// The IDs of written pages.
    page_refs: Vec<Ref>,
    /// The IDs of written fonts.
    font_refs: Vec<Ref>,
    /// The IDs of written images.
    image_refs: Vec<Ref>,
    /// The IDs of written gradients.
    gradient_refs: Vec<Ref>,
    /// The IDs of written patterns.
    pattern_refs: Vec<Ref>,
    /// The IDs of written external graphics states.
    ext_gs_refs: Vec<Ref>,
    /// Handles color space writing.
    colors: ColorSpaces,

    /// Deduplicates fonts used across the document.
    font_map: Remapper<Font>,
    /// Deduplicates images used across the document.
    image_map: Remapper<Image>,
    /// Handles to deferred image conversions.
    image_deferred_map: HashMap<usize, Deferred<EncodedImage>>,
    /// Deduplicates gradients used across the document.
    gradient_map: Remapper<PdfGradient>,
    /// Deduplicates patterns used across the document.
    pattern_map: Remapper<PdfPattern>,
    /// Deduplicates external graphics states used across the document.
    extg_map: Remapper<ExtGState>,
}

impl<'a> PdfContext<'a> {
    fn new(document: &'a Document) -> Self {
        let mut alloc = Ref::new(1);
        let page_tree_ref = alloc.bump();
        Self {
            document,
            pdf: Pdf::new(),
            pages: vec![],
            glyph_sets: HashMap::new(),
            languages: HashMap::new(),
            alloc,
            page_tree_ref,
            page_refs: vec![],
            font_refs: vec![],
            image_refs: vec![],
            gradient_refs: vec![],
            pattern_refs: vec![],
            ext_gs_refs: vec![],
            colors: ColorSpaces::default(),
            font_map: Remapper::new(),
            image_map: Remapper::new(),
            image_deferred_map: HashMap::default(),
            gradient_map: Remapper::new(),
            pattern_map: Remapper::new(),
            extg_map: Remapper::new(),
        }
    }
}

/// Write the document catalog.
fn write_catalog(ctx: &mut PdfContext, ident: Option<&str>, timestamp: Option<Datetime>) {
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

    // Write the page labels.
    let page_labels = page::write_page_labels(ctx);

    // Write the document information.
    let mut info = ctx.pdf.document_info(ctx.alloc.bump());
    let mut xmp = XmpWriter::new();
    if let Some(title) = &ctx.document.title {
        info.title(TextStr(title));
        xmp.title([(None, title.as_str())]);
    }

    let authors = &ctx.document.author;
    if !authors.is_empty() {
        // Turns out that if the authors are given in both the document
        // information dictionary and the XMP metadata, Acrobat takes a little
        // bit of both: The first author from the document information
        // dictionary and the remaining authors from the XMP metadata.
        //
        // To fix this for Acrobat, we could omit the remaining authors or all
        // metadata from the document information catalog (it is optional) and
        // only write XMP. However, not all other tools (including Apple
        // Preview) read the XMP data. This means we do want to include all
        // authors in the document information dictionary.
        //
        // Thus, the only alternative is to fold all authors into a single
        // `<rdf:li>` in the XMP metadata. This is, in fact, exactly what the
        // PDF/A spec Part 1 section 6.7.3 has to say about the matter. It's a
        // bit weird to not use the array (and it makes Acrobat show the author
        // list in quotes), but there's not much we can do about that.
        let joined = authors.join(", ");
        info.author(TextStr(&joined));
        xmp.creator([joined.as_str()]);
    }

    let creator = eco_format!("Typst {}", env!("CARGO_PKG_VERSION"));
    info.creator(TextStr(&creator));
    xmp.creator_tool(&creator);

    let keywords = &ctx.document.keywords;
    if !keywords.is_empty() {
        let joined = keywords.join(", ");
        info.keywords(TextStr(&joined));
        xmp.pdf_keywords(&joined);
    }

    if let Some(date) = ctx.document.date.unwrap_or(timestamp) {
        let tz = ctx.document.date.is_auto();
        if let Some(pdf_date) = pdf_date(date, tz) {
            info.creation_date(pdf_date);
            info.modified_date(pdf_date);
        }
        if let Some(xmp_date) = xmp_date(date, tz) {
            xmp.create_date(xmp_date);
            xmp.modify_date(xmp_date);
        }
    }

    info.finish();
    xmp.num_pages(ctx.document.pages.len() as u32);
    xmp.format("application/pdf");
    xmp.language(ctx.languages.keys().map(|lang| LangId(lang.as_str())));

    // A unique ID for this instance of the document. Changes if anything
    // changes in the frames.
    let instance_id = hash_base64(&ctx.pdf.as_bytes());

    if let Some(ident) = ident {
        // A unique ID for the document that stays stable across compilations.
        let doc_id = hash_base64(&("PDF-1.7", ident));
        xmp.document_id(&doc_id);
        xmp.instance_id(&instance_id);
        ctx.pdf
            .set_file_id((doc_id.clone().into_bytes(), instance_id.into_bytes()));
    } else {
        // This is not spec-compliant, but some PDF readers really want an ID.
        let bytes = instance_id.into_bytes();
        ctx.pdf.set_file_id((bytes.clone(), bytes));
    }

    xmp.rendition_class(RenditionClass::Proof);
    xmp.pdf_version("1.7");

    let xmp_buf = xmp.finish(None);
    let meta_ref = ctx.alloc.bump();
    ctx.pdf
        .stream(meta_ref, xmp_buf.as_bytes())
        .pair(Name(b"Type"), Name(b"Metadata"))
        .pair(Name(b"Subtype"), Name(b"XML"));

    // Write the document catalog.
    let mut catalog = ctx.pdf.catalog(ctx.alloc.bump());
    catalog.pages(ctx.page_tree_ref);
    catalog.viewer_preferences().direction(dir);
    catalog.metadata(meta_ref);

    // Insert the page labels.
    if !page_labels.is_empty() {
        let mut num_tree = catalog.page_labels();
        let mut entries = num_tree.nums();
        for (n, r) in &page_labels {
            entries.insert(n.get() as i32 - 1, *r);
        }
    }

    if let Some(outline_root_id) = outline_root_id {
        catalog.outlines(outline_root_id);
    }

    if let Some(lang) = lang {
        catalog.lang(TextStr(lang.as_str()));
    }
}

/// Compress data with the DEFLATE algorithm.
fn deflate(data: &[u8]) -> Vec<u8> {
    const COMPRESSION_LEVEL: u8 = 6;
    miniz_oxide::deflate::compress_to_vec_zlib(data, COMPRESSION_LEVEL)
}

/// Memoized version of [`deflate`] specialized for a page's content stream.
#[comemo::memoize]
fn deflate_memoized(content: &[u8]) -> Arc<Vec<u8>> {
    Arc::new(deflate(content))
}

/// Memoized and deferred version of [`deflate`] specialized for a page's content
/// stream.
#[comemo::memoize]
fn deflate_deferred(content: Vec<u8>) -> Deferred<Vec<u8>> {
    Deferred::new(move || deflate(&content))
}

/// Create a base64-encoded hash of the value.
fn hash_base64<T: Hash>(value: &T) -> String {
    base64::engine::general_purpose::STANDARD
        .encode(typst::util::hash128(value).to_be_bytes())
}

/// Converts a datetime to a pdf-writer date.
fn pdf_date(datetime: Datetime, tz: bool) -> Option<pdf_writer::Date> {
    let year = datetime.year().filter(|&y| y >= 0)? as u16;

    let mut pdf_date = pdf_writer::Date::new(year);

    if let Some(month) = datetime.month() {
        pdf_date = pdf_date.month(month);
    }

    if let Some(day) = datetime.day() {
        pdf_date = pdf_date.day(day);
    }

    if let Some(h) = datetime.hour() {
        pdf_date = pdf_date.hour(h);
    }

    if let Some(m) = datetime.minute() {
        pdf_date = pdf_date.minute(m);
    }

    if let Some(s) = datetime.second() {
        pdf_date = pdf_date.second(s);
    }

    if tz {
        pdf_date = pdf_date.utc_offset_hour(0).utc_offset_minute(0);
    }

    Some(pdf_date)
}

/// Converts a datetime to an xmp-writer datetime.
fn xmp_date(datetime: Datetime, tz: bool) -> Option<xmp_writer::DateTime> {
    let year = datetime.year().filter(|&y| y >= 0)? as u16;
    Some(DateTime {
        year,
        month: datetime.month(),
        day: datetime.day(),
        hour: datetime.hour(),
        minute: datetime.minute(),
        second: datetime.second(),
        timezone: if tz { Some(Timezone::Utc) } else { None },
    })
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

    fn insert(&mut self, item: T) -> usize {
        let to_layout = &mut self.to_items;
        *self.to_pdf.entry(item.clone()).or_insert_with(|| {
            let pdf_index = to_layout.len();
            to_layout.push(item);
            pdf_index
        })
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

/// Convert to an array of floats.
fn transform_to_array(ts: Transform) -> [f32; 6] {
    [
        ts.sx.get() as f32,
        ts.ky.get() as f32,
        ts.kx.get() as f32,
        ts.sy.get() as f32,
        ts.tx.to_f32(),
        ts.ty.to_f32(),
    ]
}
