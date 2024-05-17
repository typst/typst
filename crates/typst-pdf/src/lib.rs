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
use color::{alloc_color_functions_refs, ColorFunctionRefs};
use color_font::{write_color_fonts, ColorFontSlice};
use ecow::EcoString;
use extg::write_graphic_states;
use font::write_fonts;
use gradient::write_gradients;
use image::write_images;
use named_destination::write_named_destinations;
use page::{alloc_page_refs, traverse_pages, write_page_tree, PageTreeRef};
use pattern::{write_patterns, PatternRemapper};
use pdf_writer::{Chunk, Pdf, Ref};

use resources::{alloc_resources_refs, write_global_resources, ResourcesRefs};
use typst::foundations::{Datetime, Smart};
use typst::layout::{Abs, Em, Transform};
use typst::model::Document;
use typst::text::{Font, Lang};
use typst::util::Deferred;
use typst::visualize::Image;

use crate::catalog::Catalog;
use crate::color::ColorSpaces;
use crate::color_font::ColorFontMap;
use crate::extg::ExtGState;
use crate::gradient::PdfGradient;
use crate::image::EncodedImage;
use crate::named_destination::NamedDestinations;
use crate::page::EncodedPage;
use crate::pattern::{PdfPattern, WrittenPattern};

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
        .with(traverse_pages)
        .start_building::<GlobalRefs, AllocGlobalRefs>()
        .with(alloc_page_refs)
        .with(alloc_color_functions_refs)
        .with(alloc_resources_refs)
        .start_building::<References, AllocRefs>()
        .with(write_color_fonts)
        .with(write_fonts)
        .with(write_images)
        .with(write_gradients)
        .with(write_graphic_states)
        .with(write_patterns)
        .with(write_named_destinations)
        .start_building::<PageTreeRef, WritePageTree>()
        .with(write_page_tree)
        .start_building::<(), WriteResources>()
        .with(write_global_resources)
        .export_with(Catalog { ident, timestamp })
}

/// A struct to build a PDF following a fixed sequence of steps.
///
/// Steps are run in the order given by the successive [`PdfBuilder::with`]
/// calls. A step can read the current context (`S`) and write to the new part
/// of the context being built (`S::ToBuild`).
///
/// To combine the last state and what was built by the previous steps in a new
/// state, [`PdfBuilder::then`] is used.
///
/// A final step, that has direct access to the global reference allocator and
/// PDF document, can be run with [`PdfBuilder::export_with`].
struct PdfBuilder<S, B> {
    /// The context that has been accumulated so far.
    state: S,
    /// A new part of the context that is currently being built.
    building: B,
    /// A global bump allocator.
    alloc: Ref,
    /// The PDF document that is being written.
    pdf: Pdf,
}

/// The initial state: we are exploring the document, collecting all resources
/// that will be necessary later.
///
/// The only step here is [`Pages`].
struct BuildContent<'a> {
    document: &'a Document,
}

/// All the resources that have been collected when traversing the document.
struct Resources<'a, R = Ref> {
    reference: R,
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
    patterns: Option<Box<PatternRemapper<'a, R>>>,
    /// Deduplicates external graphics states used across the document.
    ext_gs: Remapper<ExtGState>,
    /// Deduplicates color glyphs.
    color_fonts: Option<Box<ColorFontMap<'a, R>>>,
}

impl<'a, R: Renumber> Renumber for Resources<'a, R> {
    fn renumber(&mut self, old: Ref, new: Ref) {
        self.reference.renumber(old, new);

        if let Some(color_fonts) = &mut self.color_fonts {
            color_fonts.resources.renumber(old, new);
        }

        if let Some(patterns) = &mut self.patterns {
            patterns.resources.renumber(old, new);
        }
    }
}

impl<'a> Default for Resources<'a, ()> {
    fn default() -> Self {
        Resources {
            reference: (),
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

impl<'a> Resources<'a, ()> {
    fn with_refs(self, refs: &ResourcesRefs) -> Resources<'a, Ref> {
        Resources {
            reference: refs.reference,
            pages: self.pages,
            languages: self.languages,
            glyph_sets: self.glyph_sets,
            colors: self.colors,
            fonts: self.fonts,
            images: self.images,
            deferred_images: self.deferred_images,
            gradients: self.gradients,
            patterns: self
                .patterns
                .zip(refs.patterns.as_ref())
                .map(|(p, r)| Box::new(p.with_refs(r))),
            ext_gs: self.ext_gs,
            color_fonts: self
                .color_fonts
                .zip(refs.color_fonts.as_ref())
                .map(|(c, r)| Box::new(c.with_refs(r))),
        }
    }
}

impl<'a> Default for Resources<'a> {
    fn default() -> Self {
        Resources {
            reference: Ref::new(1),
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

impl<'a, R> Resources<'a, R> {
    fn write<P>(&self, process: &mut P)
    where
        P: FnMut(&Self),
    {
        process(self);
        if let Some(color_fonts) = &self.color_fonts {
            color_fonts.resources.write(process)
        }
        if let Some(patterns) = &self.patterns {
            patterns.resources.write(process)
        }
    }
}

struct AllocGlobalRefs<'a> {
    document: &'a Document,
    resources: Resources<'a, ()>,
}

impl<'a> From<(BuildContent<'a>, Resources<'a, ()>)> for AllocGlobalRefs<'a> {
    fn from((previous, resources): (BuildContent<'a>, Resources<'a, ()>)) -> Self {
        Self { document: previous.document, resources }
    }
}

/// At this point, the resources have been collected, and global references have
/// been allocated.
///
/// We are now writing objects corresponding to resources, and giving them references,
/// that will be collected in [`References`].
struct AllocRefs<'a> {
    document: &'a Document,
    globals: GlobalRefs,
    resources: Resources<'a>,
}

impl<'a> From<(AllocGlobalRefs<'a>, GlobalRefs)> for AllocRefs<'a> {
    fn from((previous, globals): (AllocGlobalRefs<'a>, GlobalRefs)) -> Self {
        Self {
            document: previous.document,
            resources: previous.resources.with_refs(&globals.resources),
            globals,
        }
    }
}

/// The references that have been assigned to each object.
#[derive(Default)]
struct References {
    named_destinations: NamedDestinations,
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

/// At this point, the references have been assigned to all resources. The page
/// tree is going to be written, and given an ID. It is also at this point that
/// the page contents is actually written.
struct WritePageTree<'a> {
    globals: GlobalRefs,
    document: &'a Document,
    resources: Resources<'a>,
    references: References,
}

impl<'a> From<(AllocRefs<'a>, References)> for WritePageTree<'a> {
    fn from((previous, references): (AllocRefs<'a>, References)) -> Self {
        Self {
            globals: previous.globals,
            document: previous.document,
            resources: previous.resources,
            references,
        }
    }
}

/// The final step: write global resources dictionnaries.
///
/// Each subcontext gets its own isolated resource dictionnary.
struct WriteResources<'a> {
    globals: GlobalRefs,
    document: &'a Document,
    resources: Resources<'a>,
    references: References,
    page_tree_ref: Ref,
}

impl<'a> From<(WritePageTree<'a>, PageTreeRef)> for WriteResources<'a> {
    fn from((previous, page_tree_ref): (WritePageTree<'a>, PageTreeRef)) -> Self {
        Self {
            globals: previous.globals,
            document: previous.document,
            resources: previous.resources,
            references: previous.references,
            page_tree_ref: page_tree_ref.0,
        }
    }
}

/// A final step, that exports the PDF document.
///
/// It has direct access to the whole state, the PDF document, and the global
/// allocator.
trait FinalStep<S> {
    fn run(&self, state: S, pdf: &mut Pdf, alloc: &mut Ref);
}

impl<'a> PdfBuilder<BuildContent<'a>, Resources<'a, ()>> {
    /// Start building a PDF for a Typst document.
    fn new(document: &'a Document) -> Self {
        Self {
            alloc: Ref::new(1),
            pdf: Pdf::new(),
            state: BuildContent { document },
            building: Resources::default(),
        }
    }
}

impl<S, B> PdfBuilder<S, B> {
    /// Runs a step with the current state.
    fn with<P, O, F>(mut self, process: P) -> Self
    where
        // Process
        P: Fn(&S, &mut PdfChunk, &mut O) -> F,
        // Output
        O: Default + Renumber,
        // Field access
        F: for<'a> Fn(&'a mut B) -> &'a mut O,
    {
        // Any reference below that value was already allocated before and
        // should not be rewritten. Anything above was allocated in the current
        // chunk, and should be remapped.
        //
        // This is a constant (large enough to avoid collisions) and not
        // dependant on self.alloc to allow for better memoization of steps, if
        // needed in the future.
        const TEMPORARY_REFS_START: i32 = 1_000_000_000;

        let mut output = Default::default();
        let mut chunk: PdfChunk = PdfChunk::new(TEMPORARY_REFS_START);
        let save = process(&self.state, &mut chunk, &mut output);

        // Allocate a final reference for each temporary one
        let allocated = chunk.alloc.get() - TEMPORARY_REFS_START;
        let mapping: HashMap<_, _> = (0..allocated)
            .map(|i| (Ref::new(TEMPORARY_REFS_START + i), self.alloc.bump()))
            .collect();

        // Merge the chunk into the PDF, using the new references
        chunk.renumber_into(&mut self.pdf, |r| *mapping.get(&r).unwrap_or(&r));

        // Also update the references in the output
        for (old, new) in mapping {
            output.renumber(old, new);
        }

        *save(&mut self.building) = output;

        self
    }

    /// Transitions to the next state.
    fn start_building<NB: Default, NS: From<(S, B)>>(self) -> PdfBuilder<NS, NB> {
        PdfBuilder {
            state: NS::from((self.state, self.building)),
            building: NB::default(),
            alloc: self.alloc,
            pdf: self.pdf,
        }
    }

    /// Finalize the PDF export and returns the buffer representing the
    /// document.
    fn export_with(mut self, step: impl FinalStep<S>) -> Vec<u8> {
        step.run(self.state, &mut self.pdf, &mut self.alloc);
        self.pdf.finish()
    }
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
        for v in self.values_mut() {
            v.renumber(old, new);
        }
    }
}

/// Global references
#[derive(Default)]
struct GlobalRefs {
    color_functions: ColorFunctionRefs,
    pages: Vec<Ref>,
    resources: ResourcesRefs,
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
