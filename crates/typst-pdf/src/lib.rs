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
use color_font::ColorFontSlice;
use ecow::EcoString;
use page::PageTreeRef;
use pattern::PatternRemapper;
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
use crate::font::Fonts;
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
        .with(Pages)
        .then()
        .with(ColorFonts)
        .with(Fonts)
        .with(Images)
        .with(Gradients)
        .with(ExtGraphicsState)
        .with(Patterns)
        .with(NamedDestinations)
        .then()
        .with(PageTree)
        .then()
        .with(GlobalResources)
        .export_with(Catalog { ident, timestamp })
}

/// A struct to build a PDF following a fixed sequence of steps.
///
/// There are three different kind of steps:
/// - first, all resources that will be later be needed are collected with `construct`.
/// - then, each kind of resource is stored in the document using `with_resource`
/// - finally, some global information is written, with `write`
struct PdfBuilder<S: State> {
    /// Some context about the current document: the different pages, images,
    /// fonts, and so on.
    state: S,
    building: S::ToBuild,
    /// A global bump allocator.
    alloc: Ref,
    /// The PDF document that is being written.
    pdf: Pdf,
}

trait State: Sized {
    type ToBuild;
    type Next: State;

    fn start() -> Self::ToBuild;

    fn next(self, alloc: &mut Ref, built: Self::ToBuild) -> Self::Next;

    fn subcontexts<'a>(&'a self) -> impl Iterator<Item = &'a Self>;
}

struct Resources<S: State> {
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
    patterns: Option<Box<PatternRemapper<S>>>,
    /// Deduplicates external graphics states used across the document.
    ext_gs: Remapper<ExtGState>,
    /// Deduplicates color glyphs.
    color_fonts: Option<Box<ColorFontMap<S>>>,
}

impl<S: State> Renumber for Resources<S> {
    fn renumber(&mut self, _old: Ref, _new: Ref) {}
}

impl<S: State> Default for Resources<S> {
    fn default() -> Self {
        Resources {
            pages: Vec::new(),
            glyph_sets: HashMap::new(),
            languages: BTreeMap::new(),
            colors: ColorSpaces::default(),
            fonts: Remapper::new(),
            images: Remapper::new(),
            deferred_images: HashMap::new(),
            gradients: Remapper::new(),
            patterns: None,
            ext_gs: Remapper::new(),
            color_fonts: None,
        }
    }
}

impl<S: State> Resources<S> {
    fn next(self, alloc: &mut Ref) -> Resources<S::Next> {
        Resources::<S::Next> {
            pages: self.pages,
            languages: self.languages,
            glyph_sets: self.glyph_sets,
            colors: self.colors,
            fonts: self.fonts,
            images: self.images,
            deferred_images: self.deferred_images,
            gradients: self.gradients,
            patterns: self.patterns.map(|p| Box::new((*p).next(alloc))),
            ext_gs: self.ext_gs,
            color_fonts: self.color_fonts.map(|c| Box::new((*c).next(alloc))),
        }
    }
}

impl<'a> BuildContent<'a> {
    fn new(document: &'a Document) -> Self {
        BuildContent { document }
    }
}

struct BuildContent<'a> {
    /// The document that we're currently exporting.
    document: &'a Document,
}
struct AllocRefs<'a> {
    document: &'a Document,
    globals: GlobalRefs,
    resources: Resources<Self>,
}
struct WritePageTree<'a> {
    globals: GlobalRefs,
    document: &'a Document,
    resources: Resources<Self>,
    references: References,
}

struct WriteResources<'a> {
    globals: GlobalRefs,
    document: &'a Document,
    resources: Resources<Self>,
    references: References,
    page_tree_ref: Ref,
}

impl<'a> State for BuildContent<'a> {
    type Next = AllocRefs<'a>;
    type ToBuild = Resources<Self>;

    fn start() -> Self::ToBuild {
        Resources::default()
    }

    fn next(self, alloc: &mut Ref, resources: Resources<Self>) -> Self::Next {
        AllocRefs {
            document: &self.document,
            globals: GlobalRefs::new(alloc, self.document.pages.len()),
            resources: resources.next(alloc),
        }
    }

    fn subcontexts(&self) -> impl Iterator<Item = &Self> {
        std::iter::empty()
    }
}

impl<'a> State for AllocRefs<'a> {
    type ToBuild = References;

    type Next = WritePageTree<'a>;

    fn start() -> Self::ToBuild {
        References::default()
    }

    fn next(self, alloc: &mut Ref, references: References) -> Self::Next {
        WritePageTree {
            document: &self.document,
            resources: self.resources.next(alloc),
            globals: self.globals,
            references,
        }
    }

    fn subcontexts<'b>(&'b self) -> impl Iterator<Item = &'b Self> {
        let x = self.resources.patterns.as_ref().map(|p| &p.ctx).into_iter();
        let y = self.resources.color_fonts.as_ref().map(|c| &c.ctx).into_iter();
        y.chain(x)
    }
}

impl<'a> State for WritePageTree<'a> {
    type ToBuild = PageTreeRef;

    type Next = WriteResources<'a>;

    fn start() -> Self::ToBuild {
        PageTreeRef(Ref::new(1))
    }

    fn next(self, alloc: &mut Ref, page_tree_ref: PageTreeRef) -> Self::Next {
        WriteResources {
            globals: self.globals,
            document: self.document,
            resources: self.resources.next(alloc),
            references: self.references,
            page_tree_ref: page_tree_ref.0,
        }
    }

    fn subcontexts<'b>(&'b self) -> impl Iterator<Item = &'b Self> {
        std::iter::empty()
    }
}

impl<'a> State for WriteResources<'a> {
    type ToBuild = ();

    type Next = Self;

    fn start() -> Self::ToBuild {
        ()
    }

    fn next(self, _alloc: &mut Ref, (): ()) -> Self::Next {
        self
    }

    fn subcontexts<'b>(&'b self) -> impl Iterator<Item = &'b Self> {
        // HACK: because WriteStep::save is only called on the top level
        // context, subcontexts always have empty References, which crashes the
        // GlobalResources step.
        //
        // For the moment, we do as if there were no subcontext here, and
        // manually recurse (but always with the root references) in
        // GlobalResources::run.
        std::iter::empty()
    }
}

trait Step<S: State> {
    fn run(&self, state: &S, output: &mut S::ToBuild);
}

/// Same as [`Step`] but also writes a part of the final PDF file.
///
/// Because what is written is local to the current chunk only, the
/// output of this step will be renumbered before being saved.
trait WriteStep<S: State> {
    type Output: Default + Renumber;

    fn run(&self, state: &S, chunk: &mut PdfChunk, output: &mut Self::Output);

    fn save(context: &mut S::ToBuild, output: Self::Output);
}

impl<R: Default + Renumber, S: State<ToBuild = R>, T: Step<S>> WriteStep<S> for T {
    type Output = S::ToBuild;

    fn run(&self, state: &S, _chunk: &mut PdfChunk, output: &mut Self::Output) {
        self.run(state, output);
    }

    fn save(context: &mut <S as State>::ToBuild, output: Self::Output) {
        *context = output;
    }
}

trait FinalStep<S: State> {
    fn run(&self, state: S, pdf: &mut Pdf, alloc: &mut Ref);
}

impl<'a> PdfBuilder<BuildContent<'a>> {
    /// Start building a PDF for a Typst document.
    fn new(document: &'a Document) -> Self {
        Self {
            alloc: Ref::new(1),
            pdf: Pdf::new(),
            state: BuildContent::new(document),
            building: Resources::default(),
        }
    }
}

impl<S: State> PdfBuilder<S> {
    fn with<W: WriteStep<S>>(mut self, step: W) -> Self {
        fn write<S: State, W: WriteStep<S>>(
            chunk: &mut PdfChunk,
            mapping: &mut HashMap<Ref, Ref>,
            ctx: &S,
            output: &mut W::Output,
            step: &W,
        ) {
            step.run(ctx, chunk, output);

            for subcontext in ctx.subcontexts() {
                write(chunk, mapping, subcontext, output, step);
            }
        }

        let mut output = Default::default();
        let mut chunk: PdfChunk = PdfChunk::new(ALLOC_SECTION_SIZE);
        let mut mapping = HashMap::new();
        write(&mut chunk, &mut mapping, &self.state, &mut output, &step);

        chunk.renumber_into(&mut self.pdf, |r| {
            if r.get() < ALLOC_SECTION_SIZE {
                return r;
            }
            *mapping.entry(r).or_insert_with(|| self.alloc.bump())
        });

        for (old, new) in mapping {
            output.renumber(old, new);
        }

        W::save(&mut self.building, output);

        self
    }

    fn then(mut self) -> PdfBuilder<S::Next> {
        PdfBuilder {
            state: self.state.next(&mut self.alloc, self.building),
            building: S::Next::start(),
            alloc: self.alloc,
            pdf: self.pdf,
        }
    }

    fn export_with(mut self, step: impl FinalStep<S>) -> Vec<u8> {
        step.run(self.state, &mut self.pdf, &mut self.alloc);
        self.pdf.finish()
    }
}

#[derive(Default, Debug)]
struct References {
    /// A map between elements and their associated labels
    loc_to_dest: HashMap<Location, Label>,
    /// A sorted list of all named destinations.
    dests: Vec<(Label, Ref)>,
    /// The IDs of written fonts.
    fonts: HashMap<Font, Ref>,
    /// The IDs of written color fonts.
    color_fonts: HashMap<ColorFontSlice, Ref>,
    /// The IDs of written images.
    images: HashMap<Image, Ref>,
    /// The IDs of written gradients.
    gradients: HashMap<PdfGradient, Ref>,
    /// The IDs of written patterns.
    patterns: HashMap<PdfPattern, WrittenPattern>,
    /// The IDs of written external graphics states.
    ext_gs: HashMap<ExtGState, Ref>,
}

const ALLOC_SECTION_SIZE: i32 = 1_000_000;

/// Collects all objects that will have to be embedded in the final PDF.
///
/// This can be pages, images, fonts, gradients, etc. They should all be saved
/// in the `PdfContext` that is being passed to the `write` function.
/// This function can write to the final document by using the given `PdfChunk`.
trait PdfConstructor {
    fn write(&self, context: &mut BuildContent);
}

/// A specific kind of resource that is present in a PDF document.
trait PdfResource {
    type Output: Renumber + Default;

    /// Write all data related to this kind of resource in the document.
    ///
    /// This function can return references that are local to `chunk`, they
    /// will be correctly re-numbered before being saved for later steps.
    fn write(&self, context: &AllocRefs, chunk: &mut PdfChunk, out: &mut Self::Output);

    /// Save references that this step exported.
    fn save(context: &mut References, output: Self::Output);
}

/// Write global information about the PDF document.
trait PdfWriter {
    fn write(
        &self,
        pdf: &mut Pdf,
        alloc: &mut Ref,
        ctx: &WritePageTree,
        refs: &mut References,
    );
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

impl<T: Eq + Hash, R: Renumber> Renumber for HashMap<T, R> {
    fn renumber(&mut self, old: Ref, new: Ref) {
        for (_, v) in self {
            v.renumber(old, new);
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
    pages: Vec<Ref>,
}

impl GlobalRefs {
    fn new(alloc: &mut Ref, page_count: usize) -> Self {
        GlobalRefs {
            resources: alloc.bump(),
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
    fn new(alloc_start: i32) -> Self {
        PdfChunk { chunk: Chunk::new(), alloc: Ref::new(alloc_start) }
    }

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
