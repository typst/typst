//! Exporting of Typst documents into PDFs.

mod color;
mod extg;
mod font;
mod gradient;
mod image;
mod outline;
mod page;
mod pattern;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::Hash;
use std::sync::Arc;

use base64::Engine;
use ecow::{eco_format, EcoString};
use indexmap::IndexMap;
use pdf_writer::types::Direction;
use pdf_writer::writers::Destination;
use pdf_writer::{Finish, Name, Pdf, Rect, Ref, Str, TextStr};
use typst::foundations::{Datetime, Label, NativeElement, Smart};
use typst::introspection::Location;
use typst::layout::{Abs, Dir, Em, Frame, Transform};
use typst::model::{Document, HeadingElem};
use typst::text::color::frame_for_glyph;
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
/// The `ident` parameter, if given, shall be a string that uniquely and stably
/// identifies the document. It should not change between compilations of the
/// same document.  **If you cannot provide such a stable identifier, just pass
/// `Smart::Auto` rather than trying to come up with one.** The CLI, for
/// example, does not have a well-defined notion of a long-lived project and as
/// such just passes `Smart::Auto`.
///
/// If an `ident` is given, the hash of it will be used to create a PDF document
/// identifier (the identifier itself is not leaked). If `ident` is `Auto`, a
/// hash of the document's title and author is used instead (which is reasonably
/// unique and stable).
///
/// The `timestamp`, if given, is expected to be the creation date of the
/// document as a UTC datetime. It will only be used if `set document(date: ..)`
/// is `auto`.
#[typst_macros::time(name = "pdf")]
pub fn pdf(
    document: &Document,
    ident: Smart<&str>,
    timestamp: Option<Datetime>,
) -> Vec<u8> {
    let mut ctx = PdfContext::new(document);
    page::construct_pages(&mut ctx, &document.pages);
    font::write_fonts(&mut ctx);
    image::write_images(&mut ctx);
    gradient::write_gradients(&mut ctx);
    extg::write_external_graphics_states(&mut ctx);
    pattern::write_patterns(&mut ctx);
    write_named_destinations(&mut ctx);
    page::write_page_tree(&mut ctx);
    page::write_global_resources(&mut ctx);
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
    /// BTreeMap is used to write sorted list of languages to metadata.
    languages: BTreeMap<Lang, usize>,

    /// Allocator for indirect reference IDs.
    alloc: Ref,
    /// The ID of the page tree.
    page_tree_ref: Ref,
    /// The ID of the globally shared Resources dictionary.
    global_resources_ref: Ref,
    /// The ID of the resource dictionary shared by Type3 fonts.
    ///
    /// Type3 fonts cannot use the global resources, as it would create some
    /// kind of infinite recursion (they are themselves present in that
    /// dictionary), which Acrobat doesn't appreciate (it fails to parse the
    /// font) even if the specification seems to allow it.
    type3_font_resources_ref: Ref,
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
    /// Deduplicates color glyphs.
    color_font_map: ColorFontMap,

    /// A sorted list of all named destinations.
    dests: Vec<(Label, Ref)>,
    /// Maps from locations to named destinations that point to them.
    loc_to_dest: HashMap<Location, Label>,
}

impl<'a> PdfContext<'a> {
    fn new(document: &'a Document) -> Self {
        let mut alloc = Ref::new(1);
        let page_tree_ref = alloc.bump();
        let global_resources_ref = alloc.bump();
        let type3_font_resources_ref = alloc.bump();
        Self {
            document,
            pdf: Pdf::new(),
            pages: vec![],
            glyph_sets: HashMap::new(),
            languages: BTreeMap::new(),
            alloc,
            page_tree_ref,
            global_resources_ref,
            type3_font_resources_ref,
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
            color_font_map: ColorFontMap::new(),
            dests: vec![],
            loc_to_dest: HashMap::new(),
        }
    }
}

/// Write the document catalog.
fn write_catalog(ctx: &mut PdfContext, ident: Smart<&str>, timestamp: Option<Datetime>) {
    let lang = ctx.languages.iter().max_by_key(|(_, &count)| count).map(|(&l, _)| l);

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

    // Determine the document's ID. It should be as stable as possible.
    const PDF_VERSION: &str = "PDF-1.7";
    let doc_id = if let Smart::Custom(ident) = ident {
        // We were provided with a stable ID. Yay!
        hash_base64(&(PDF_VERSION, ident))
    } else if ctx.document.title.is_some() && !ctx.document.author.is_empty() {
        // If not provided from the outside, but title and author were given, we
        // compute a hash of them, which should be reasonably stable and unique.
        hash_base64(&(PDF_VERSION, &ctx.document.title, &ctx.document.author))
    } else {
        // The user provided no usable metadata which we can use as an `/ID`.
        instance_id.clone()
    };

    // Write IDs.
    xmp.document_id(&doc_id);
    xmp.instance_id(&instance_id);
    ctx.pdf
        .set_file_id((doc_id.clone().into_bytes(), instance_id.into_bytes()));

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

    // Write the named destination tree.
    let mut name_dict = catalog.names();
    let mut dests_name_tree = name_dict.destinations();
    let mut names = dests_name_tree.names();
    for &(name, dest_ref, ..) in &ctx.dests {
        names.insert(Str(name.as_str().as_bytes()), dest_ref);
    }
    names.finish();
    dests_name_tree.finish();
    name_dict.finish();

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

    catalog.finish();
}

/// Fills in the map and vector for named destinations and writes the indirect
/// destination objects.
fn write_named_destinations(ctx: &mut PdfContext) {
    let mut seen = HashSet::new();

    // Find all headings that have a label and are the first among other
    // headings with the same label.
    let mut matches: Vec<_> = ctx
        .document
        .introspector
        .query(&HeadingElem::elem().select())
        .iter()
        .filter_map(|elem| elem.location().zip(elem.label()))
        .filter(|&(_, label)| seen.insert(label))
        .collect();

    // Named destinations must be sorted by key.
    matches.sort_by_key(|&(_, label)| label);

    for (loc, label) in matches {
        let pos = ctx.document.introspector.position(loc);
        let index = pos.page.get() - 1;
        let y = (pos.point.y - Abs::pt(10.0)).max(Abs::zero());

        if let Some(page) = ctx.pages.get(index) {
            let dest_ref = ctx.alloc.bump();
            let x = pos.point.x.to_f32();
            let y = (page.size.y - y).to_f32();
            ctx.dests.push((label, dest_ref));
            ctx.loc_to_dest.insert(loc, label);
            ctx.pdf
                .indirect(dest_ref)
                .start::<Destination>()
                .page(page.id)
                .xyz(x, y, None);
        }
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

/// A mapping between `Font`s and all the corresponding `ColorFont`s.
///
/// This mapping is one-to-many because there can only be 256 glyphs in a Type 3
/// font, and fonts generally have more color glyphs than that.
struct ColorFontMap {
    /// The mapping itself
    map: IndexMap<Font, ColorFont>,
    /// A list of all PDF indirect references to Type3 font objects.
    all_refs: Vec<Ref>,
}

/// A collection of Type3 font, belonging to the same TTF font.
struct ColorFont {
    /// A list of references to Type3 font objects for this font family.
    refs: Vec<Ref>,
    /// The list of all color glyphs in this family.
    ///
    /// The index in this vector modulo 256 corresponds to the index in one of
    /// the Type3 fonts in `refs` (the `n`-th in the vector, where `n` is the
    /// quotient of the index divided by 256).
    glyphs: Vec<ColorGlyph>,
    /// The global bounding box of the font.
    bbox: Rect,
    /// A mapping between glyph IDs and character indices in the `glyphs`
    /// vector.
    glyph_indices: HashMap<u16, usize>,
}

/// A single color glyph.
struct ColorGlyph {
    /// The ID of the glyph.
    gid: u16,
    /// A frame that contains the glyph.
    frame: Frame,
}

impl ColorFontMap {
    /// Creates a new empty mapping
    fn new() -> Self {
        Self { map: IndexMap::new(), all_refs: Vec::new() }
    }

    /// Takes the contents of the mapping.
    ///
    /// After calling this function, the mapping will be empty.
    fn take_map(&mut self) -> IndexMap<Font, ColorFont> {
        std::mem::take(&mut self.map)
    }

    /// Obtains the reference to a Type3 font, and an index in this font
    /// that can be used to draw a color glyph.
    ///
    /// The glyphs will be de-duplicated if needed.
    fn get(&mut self, alloc: &mut Ref, font: &Font, gid: u16) -> (Ref, u8) {
        let color_font = self.map.entry(font.clone()).or_insert_with(|| {
            let global_bbox = font.ttf().global_bounding_box();
            let bbox = Rect::new(
                font.to_em(global_bbox.x_min).to_font_units(),
                font.to_em(global_bbox.y_min).to_font_units(),
                font.to_em(global_bbox.x_max).to_font_units(),
                font.to_em(global_bbox.y_max).to_font_units(),
            );
            ColorFont {
                bbox,
                refs: Vec::new(),
                glyphs: Vec::new(),
                glyph_indices: HashMap::new(),
            }
        });

        if let Some(index_of_glyph) = color_font.glyph_indices.get(&gid) {
            // If we already know this glyph, return it.
            (color_font.refs[index_of_glyph / 256], *index_of_glyph as u8)
        } else {
            // Otherwise, allocate a new ColorGlyph in the font, and a new Type3 font
            // if needed
            let index = color_font.glyphs.len();
            if index % 256 == 0 {
                let new_ref = alloc.bump();
                self.all_refs.push(new_ref);
                color_font.refs.push(new_ref);
            }

            let instructions = frame_for_glyph(font, gid);
            color_font.glyphs.push(ColorGlyph { gid, frame: instructions });
            color_font.glyph_indices.insert(gid, index);

            (color_font.refs[index / 256], index as u8)
        }
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
