//! Exporting of Typst documents into PDFs.

mod catalog;
mod color;
mod color_font;
mod content;
mod extg;
mod font;
mod gradient;
mod image;
mod named_destination;
mod outline;
mod page;
mod pattern;
mod resources;

use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;
use std::ops::{Deref, DerefMut};

use base64::Engine;
use ecow::EcoString;
use pdf_writer::{Chunk, Pdf, Ref};

use typst::foundations::{Datetime, Label, Smart};
use typst::introspection::Location;
use typst::layout::{Abs, Em, Transform};
use typst::model::Document;
use typst::text::{Font, Lang};
use typst::util::Deferred;
use typst::visualize::Image;

use crate::catalog::Catalog;
use crate::color::ColorSpaces;
use crate::color_font::{ColorFontMap, ColorFonts};
use crate::extg::{ExtGState, ExtGraphicsState};
use crate::font::{improve_glyph_sets, Fonts};
use crate::gradient::{Gradients, PdfGradient};
use crate::image::{EncodedImage, Images};
use crate::named_destination::NamedDestinations;
use crate::page::{EncodedPage, PageTree, Pages};
use crate::pattern::{Patterns, PdfPattern, WrittenPattern};
use crate::resources::GlobalResources;

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
    PdfBuilder::new(document)
        .construct(Pages)
        .with_resource(ColorFonts)
        .with_resource(Fonts)
        .with_resource(Images)
        .with_resource(Gradients)
        .with_resource(ExtGraphicsState)
        .with_resource(Patterns)
        .with_resource(NamedDestinations)
        .write(PageTree)
        .write(GlobalResources)
        .write(Catalog { ident, timestamp })
        .export()
}

/// A struct to build a PDF following a fixed sequence of steps.
///
/// There are three different kind of steps:
/// - first, all resources that will be later be needed are collected with `construct`.
/// - then, each kind of resource is stored in the document using `with_resource`
/// - finally, some global information is written, with `write`
struct PdfBuilder<'a> {
    /// Some context about the current document: the different pages, images,
    /// fonts, and so on.
    context: PdfContext<'a>,
    /// A list of all references that were allocated for the resources of this
    /// PDF document.
    references: References,
    /// A global bump allocator.
    alloc: Ref,
    /// The PDF document that is being written.
    pdf: Pdf,
}

impl<'a> PdfBuilder<'a> {
    /// Start building a PDF for a Typst document.
    fn new(document: &'a Document) -> Self {
        Self {
            context: PdfContext::new(document),
            references: References::default(),
            alloc: Ref::new(1),
            pdf: Pdf::new(),
        }
    }

    /// Run a [`PdfConstructor`] in the context of this document.
    fn construct(mut self, constructor: impl PdfConstructor) -> Self {
        let mut chunk = PdfChunk::new();
        constructor.write(&mut self.context, &mut chunk);
        improve_glyph_sets(&mut self.context.glyph_sets);
        let mut mapping = HashMap::new();
        chunk.renumber_into(&mut self.pdf, |r| {
            if r.get() < 100 {
                r
            } else {
                let new = *mapping.entry(r).or_insert_with(|| self.alloc.bump());
                new
            }
        });
        self
    }

    /// Write data related to a [`PdfResource`] in the document.
    fn with_resource(mut self, resource: impl PdfResource) -> Self {
        let mut chunk = PdfChunk::new();
        let mut output = resource.write(&self.context, &mut chunk);
        let mut mapping = HashMap::new();
        chunk.renumber_into(&mut self.pdf, |r| {
            if r.get() < 100 {
                r
            } else {
                let new = *mapping.entry(r).or_insert_with(|| self.alloc.bump());
                output.renumber(r, new);
                new
            }
        });
        self
    }

    /// Write some global information in the document.
    fn write(mut self, writer: impl PdfWriter) -> Self {
        writer.write(&mut self.pdf, &mut self.alloc, &self.context, &self.references);
        self
    }

    /// The buffer that represents the finished PDF file.
    fn export(self) -> Vec<u8> {
        self.pdf.finish()
    }
}

#[derive(Default)]
struct References {
    /// A map between elements and their associated labels
    loc_to_dest: HashMap<Location, Label>,
    /// A sorted list of all named destinations.
    dests: Vec<(Label, Ref)>,
    /// The IDs of written fonts.
    fonts: Vec<Ref>,
    /// The IDs of written color fonts.
    color_fonts: Vec<Ref>,
    /// The IDs of written images.
    images: Vec<Ref>,
    /// The IDs of written gradients.
    gradients: Vec<Ref>,
    /// The IDs of written patterns.
    patterns: Vec<WrittenPattern>,
    /// The IDs of written external graphics states.
    ext_gs: Vec<Ref>,
}

struct PdfContext<'a> {
    /// The document that we're currently exporting.
    document: &'a Document,
    /// Content of exported pages.
    pages: Vec<EncodedPage>,
    /// The number of glyphs for all referenced languages in the document.
    /// We keep track of this to determine the main document language.
    /// BTreeMap is used to write sorted list of languages to metadata.
    languages: BTreeMap<Lang, usize>,

    /// For each font a mapping from used glyphs to their text representation.
    /// May contain multiple chars in case of ligatures or similar things. The
    /// same glyph can have a different text representation within one document,
    /// then we just save the first one. The resulting strings are used for the
    /// PDF's /ToUnicode map for glyphs that don't have an entry in the font's
    /// cmap. This is important for copy-paste and searching.
    glyph_sets: HashMap<Font, BTreeMap<u16, EcoString>>,

    /// Global references.
    ///
    /// These references are allocated at the very begining of the PDF export process,
    /// and can be used in the whole document without ever needing remapping.
    globals: GlobalRefs,

    /// Handles color space writing.
    colors: ColorSpaces,

    /// Deduplicates fonts used across the document.
    fonts: Remapper<Font>,
    /// Deduplicates images used across the document.
    images: Remapper<Image>,
    /// Handles to deferred image conversions.
    deferred_images: HashMap<usize, Deferred<EncodedImage>>,
    /// Deduplicates gradients used across the document.
    gradients: Remapper<PdfGradient>,
    /// Deduplicates patterns used across the document.
    patterns: Remapper<PdfPattern<usize>>,
    remapped_patterns: Vec<PdfPattern<Ref>>,
    /// Deduplicates external graphics states used across the document.
    ext_gs: Remapper<ExtGState>,
    /// Deduplicates color glyphs.
    color_fonts: Option<Box<ColorFontMap<'a>>>,
}

impl<'a> PdfContext<'a> {
    fn new(document: &'a Document) -> Self {
        Self {
            document,
            globals: GlobalRefs::new(document.pages.len()),
            pages: vec![],
            glyph_sets: HashMap::new(),
            languages: BTreeMap::new(),
            colors: ColorSpaces::default(),
            fonts: Remapper::new(),
            images: Remapper::new(),
            deferred_images: HashMap::new(),
            gradients: Remapper::new(),
            patterns: Remapper::new(),
            remapped_patterns: Vec::new(),
            ext_gs: Remapper::new(),
            color_fonts: Some(Box::new(ColorFontMap::new(&document))),
        }
    }
}

/// Collects all objects that will have to be embedded in the final PDF.
///
/// This can be pages, images, fonts, gradients, etc. They should all be saved
/// in the `PdfContext` that is being passed to the `write` function.
/// This function can write to the final document by using the given `PdfChunk`.
trait PdfConstructor {
    fn write(&self, context: &mut PdfContext, chunk: &mut PdfChunk);
}

/// A specific kind of resource that is present in a PDF document.
trait PdfResource {
    type Output: Renumber;

    /// Write all data related to this kind of resource in the document.
    ///
    /// This function can return references that are local to `chunk`, they
    /// will be correctly re-numbered before being saved for later steps.
    fn write(&self, context: &PdfContext, chunk: &mut PdfChunk) -> Self::Output;

    /// Save references that this step exported.
    fn save(context: &mut References, output: Self::Output);
}

/// Write global information about the PDF document.
trait PdfWriter {
    fn write(&self, pdf: &mut Pdf, alloc: &mut Ref, ctx: &PdfContext, refs: &References);
}

/// A reference or collection of references that can be re-numbered,
/// to become valid in a global scope.
trait Renumber {
    fn renumber(&mut self, old: Ref, new: Ref);
}

impl Renumber for () {
    fn renumber(&mut self, _old: Ref, _new: Ref) {}
}

impl Renumber for Ref {
    fn renumber(&mut self, old: Ref, new: Ref) {
        if *self == old {
            *self = new
        }
    }
}

impl<R: Renumber> Renumber for Vec<R> {
    fn renumber(&mut self, old: Ref, new: Ref) {
        for item in self {
            item.renumber(old, new);
        }
    }
}

/// Global references
#[derive(Debug)]
struct GlobalRefs {
    // Color spaces
    oklab: Ref,
    d65_gray: Ref,
    srgb: Ref,
    // Resources
    resources: Ref,
    // Page tree and pages
    page_tree: Ref,
    pages: Vec<Ref>,
}

impl GlobalRefs {
    fn new(page_count: usize) -> Self {
        let mut alloc = Ref::new(1);
        GlobalRefs {
            resources: alloc.bump(),
            page_tree: alloc.bump(),
            pages: std::iter::repeat_with(|| alloc.bump()).take(page_count).collect(),
            oklab: alloc.bump(),
            d65_gray: alloc.bump(),
            srgb: alloc.bump(),
        }
    }
}

/// A portion of a PDF file.
struct PdfChunk {
    /// The actual chunk.
    chunk: Chunk,
    /// A local allocator.
    alloc: Ref,
}

impl PdfChunk {
    fn new() -> Self {
        PdfChunk {
            chunk: Chunk::new(),
            alloc: Ref::new(100), // TODO
        }
    }
}

impl PdfChunk {
    fn alloc(&mut self) -> Ref {
        self.alloc.bump()
    }
}

impl Deref for PdfChunk {
    type Target = Chunk;

    fn deref(&self) -> &Self::Target {
        &self.chunk
    }
}

impl DerefMut for PdfChunk {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.chunk
    }
}

/// Compress data with the DEFLATE algorithm.
fn deflate(data: &[u8]) -> Vec<u8> {
    const COMPRESSION_LEVEL: u8 = 6;
    miniz_oxide::deflate::compress_to_vec_zlib(data, COMPRESSION_LEVEL)
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

/// Assigns new, consecutive PDF-internal indices to items.
#[derive(Clone)]
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
        refs: impl IntoIterator<Item = &'a Ref> + 'a,
    ) -> impl Iterator<Item = (Ref, usize)> + 'a {
        refs.into_iter().copied().zip(0..self.to_pdf.len())
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
