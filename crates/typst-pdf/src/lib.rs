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

use typst::foundations::{Datetime, Smart};
use typst::layout::{Abs, Em, Transform};
use typst::model::Document;
use typst::text::{Font, Lang};
use typst::util::Deferred;
use typst::visualize::Image;

use crate::catalog::write_catalog;
use crate::color::{alloc_color_functions_refs, ColorFunctionRefs, ColorSpaces};
use crate::color_font::{write_color_fonts, ColorFontMap, ColorFontSlice};
use crate::extg::{write_graphic_states, ExtGState};
use crate::font::write_fonts;
use crate::gradient::{write_gradients, PdfGradient};
use crate::image::{write_images, EncodedImage};
use crate::named_destination::{write_named_destinations, NamedDestinations};
use crate::page::{
    alloc_page_refs, traverse_pages, write_page_tree, EncodedPage, PageTreeRef,
};
use crate::pattern::{write_patterns, PatternRemapper, PdfPattern, WrittenPattern};
use crate::resources::{
    alloc_resources_refs, write_resource_dictionaries, ResourcesRefs,
};

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
        .phase(|builder| builder.run(traverse_pages))
        .phase(|builder| GlobalRefs {
            color_functions: builder.run(alloc_color_functions_refs),
            pages: builder.run(alloc_page_refs),
            resources: builder.run(alloc_resources_refs),
        })
        .phase(|builder| References {
            named_destinations: builder.run(write_named_destinations),
            fonts: builder.run(write_fonts),
            color_fonts: builder.run(write_color_fonts),
            images: builder.run(write_images),
            gradients: builder.run(write_gradients),
            patterns: builder.run(write_patterns),
            ext_gs: builder.run(write_graphic_states),
        })
        .phase(|builder| builder.run(write_page_tree))
        .phase(|builder| builder.run(write_resource_dictionaries))
        .export_with(ident, timestamp, write_catalog)
}

/// A struct to build a PDF following a fixed succession of phases.
///
/// This type uses generics to represent its current state. `S` (for "state") is
/// all data that was produced by the previous phases, that is now read-only.
/// `B` (for "building") is a new set of data, that is currently being produced
/// and that should be considered write-only.
///
/// In other words: this struct follows the **typestate pattern**. This prevents
/// you from using data that is not yet available, at the type level.
///
/// Each phase consists of processes, that can read the state of the previous
/// phases (`S`), and write to the new state (`B`). These processes are run
/// in the order [`PdfBuilder::run`] is called on them.
///
/// To combine the last state and what was built by the previous phases in a new
/// state, [`PdfBuilder::transition`] is used. A new phase is then entered.
///
/// A final step, that has direct access to the global reference allocator and
/// PDF document, can be run with [`PdfBuilder::export_with`].
struct PdfBuilder<S> {
    /// The context that has been accumulated so far.
    state: S,
    /// A global bump allocator.
    alloc: Ref,
    /// The PDF document that is being written.
    pdf: Pdf,
}

/// The initial state: we are exploring the document, collecting all resources
/// that will be necessary later. The content of the pages is also built during
/// this phase.
struct BuildContent<'a> {
    document: &'a Document,
}

/// All the resources that have been collected when traversing the document.
///
/// This does not allocate references to resources, only track what was used
/// and deduplicate what can be deduplicated.
///
/// You may notice that this structure is a tree: [`PatternRemapper`] and
/// [`ColorFontMap`] (that are present in the fields of [`Resources`]),
/// themselves contain [`Resources`] (that will be called "sub-resources" from
/// now on). Because color glyphs and patterns are defined using content
/// streams, just like pages, they can refer to resources too, which are tracked
/// by the respective sub-resources.
///
/// Each instance of this structure will become a `/Resources` dictionary in
/// the final PDF. It is not possible to use a single shared dictionary for all
/// pages, patterns and color fonts, because if a resource is listed in its own
/// `/Resources` dictionary, some PDF readers will fail to open the document.
///
/// Because we need to lazily initialize sub-resources (we don't know how deep
/// the tree will be before reading the document), and that this is done in a
/// context where no PDF reference allocator is available, `Resources` are
/// originally created with the type parameter `R = ()`. The reference for each
/// dictionary will only be allocated in the next phase, once we know the shape
/// of the tree, at which point `R` becomes `Ref`. No other value of `R` should
/// ever exist.
struct Resources<R = Ref> {
    /// The global reference to this resource dictionary, or `()` if it has not
    /// been allocated yet.
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
    patterns: Option<Box<PatternRemapper<R>>>,
    /// Deduplicates external graphics states used across the document.
    ext_gs: Remapper<ExtGState>,
    /// Deduplicates color glyphs.
    color_fonts: Option<Box<ColorFontMap<R>>>,
}

impl<R: Renumber> Renumber for Resources<R> {
    fn renumber(&mut self, mapping: &HashMap<Ref, Ref>) {
        self.reference.renumber(mapping);

        if let Some(color_fonts) = &mut self.color_fonts {
            color_fonts.resources.renumber(mapping);
        }

        if let Some(patterns) = &mut self.patterns {
            patterns.resources.renumber(mapping);
        }
    }
}

impl Default for Resources<()> {
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

impl Resources<()> {
    /// Associate a reference with this resource dictionary (and do so
    /// recursively for sub-resources).
    fn with_refs(self, refs: &ResourcesRefs) -> Resources<Ref> {
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

impl<R> Resources<R> {
    /// Run a function on this resource dictionary and all
    /// of its sub-resources.
    fn traverse<P>(&self, process: &mut P)
    where
        P: FnMut(&Self),
    {
        process(self);
        if let Some(color_fonts) = &self.color_fonts {
            color_fonts.resources.traverse(process)
        }
        if let Some(patterns) = &self.patterns {
            patterns.resources.traverse(process)
        }
    }
}

/// At this point, resources were listed, but they don't have any reference
/// associated with them.
///
/// This phase allocates some global references.
struct AllocGlobalRefs<'a> {
    document: &'a Document,
    resources: Resources<()>,
}

/// Global references
struct GlobalRefs {
    color_functions: ColorFunctionRefs,
    pages: Vec<Ref>,
    resources: ResourcesRefs,
}

impl<'a> From<(BuildContent<'a>, Resources<()>)> for AllocGlobalRefs<'a> {
    fn from((previous, resources): (BuildContent<'a>, Resources<()>)) -> Self {
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
    resources: Resources,
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
/// tree is going to be written, and given a reference. It is also at this point that
/// the page contents is actually written.
struct WritePageTree<'a> {
    globals: GlobalRefs,
    document: &'a Document,
    resources: Resources,
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

/// In this phase, we write resource dictionaries.
///
/// Each sub-resource gets its own isolated resource dictionary.
struct WriteResources<'a> {
    globals: GlobalRefs,
    document: &'a Document,
    resources: Resources,
    references: References,
    page_tree_ref: Ref,
}

impl<'a> From<(WriteResources<'a>, ())> for WriteResources<'a> {
    fn from((this, _): (WriteResources<'a>, ())) -> Self {
        this
    }
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

impl<'a> PdfBuilder<BuildContent<'a>> {
    /// Start building a PDF for a Typst document.
    fn new(document: &'a Document) -> Self {
        Self {
            alloc: Ref::new(1),
            pdf: Pdf::new(),
            state: BuildContent { document },
        }
    }
}

impl<S> PdfBuilder<S> {
    fn phase<NS, B, O>(mut self, builder: B) -> PdfBuilder<NS>
    where
        // New state
        NS: From<(S, O)>,
        // Builder
        B: Fn(&mut Self) -> O,
    {
        let output = builder(&mut self);
        PdfBuilder {
            state: NS::from((self.state, output)),
            alloc: self.alloc,
            pdf: self.pdf,
        }
    }

    /// Runs a step with the current state, merge its output in the PDF file,
    /// and renumber any references it returned.
    fn run<P, O>(&mut self, process: P) -> O
    where
        // Process
        P: Fn(&S) -> (PdfChunk, O),
        // Output
        O: Renumber,
    {
        let (chunk, output) = process(&self.state);
        self.renumber(chunk, output)
    }

    /// Finalize the PDF export and returns the buffer representing the
    /// document.
    fn export_with<P>(
        mut self,
        ident: Smart<&str>,
        timestamp: Option<Datetime>,
        process: P,
    ) -> Vec<u8>
    where
        P: Fn(S, Smart<&str>, Option<Datetime>, &mut Pdf, &mut Ref),
    {
        process(self.state, ident, timestamp, &mut self.pdf, &mut self.alloc);
        self.pdf.finish()
    }

    fn renumber<O>(&mut self, chunk: PdfChunk, mut output: O) -> O
    where
        O: Renumber,
    {
        // Allocate a final reference for each temporary one
        let allocated = chunk.alloc.get() - TEMPORARY_REFS_START;
        let mapping: HashMap<_, _> = (0..allocated)
            .map(|i| (Ref::new(TEMPORARY_REFS_START + i), self.alloc.bump()))
            .collect();

        // Merge the chunk into the PDF, using the new references
        chunk.renumber_into(&mut self.pdf, |r| *mapping.get(&r).unwrap_or(&r));

        // Also update the references in the output
        output.renumber(&mapping);

        output
    }
}

/// A reference or collection of references that can be re-numbered,
/// to become valid in a global scope.
trait Renumber {
    fn renumber(&mut self, mapping: &HashMap<Ref, Ref>);
}

impl Renumber for () {
    fn renumber(&mut self, _mapping: &HashMap<Ref, Ref>) {}
}

impl Renumber for Ref {
    fn renumber(&mut self, mapping: &HashMap<Ref, Ref>) {
        if let Some(new) = mapping.get(self) {
            *self = *new
        }
    }
}

impl<R: Renumber> Renumber for Vec<R> {
    fn renumber(&mut self, mapping: &HashMap<Ref, Ref>) {
        for item in self {
            item.renumber(mapping);
        }
    }
}

impl<T: Eq + Hash, R: Renumber> Renumber for HashMap<T, R> {
    fn renumber(&mut self, mapping: &HashMap<Ref, Ref>) {
        for v in self.values_mut() {
            v.renumber(mapping);
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

// Any reference below that value was already allocated before and
// should not be rewritten. Anything above was allocated in the current
// chunk, and should be remapped.
//
// This is a constant (large enough to avoid collisions) and not
// dependant on self.alloc to allow for better memoization of steps, if
// needed in the future.
const TEMPORARY_REFS_START: i32 = 1_000_000_000;

impl PdfChunk {
    fn new() -> Self {
        PdfChunk {
            chunk: Chunk::new(),
            alloc: Ref::new(TEMPORARY_REFS_START),
        }
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
