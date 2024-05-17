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
/// Steps are run in the order given by the successive [`PdfBuilder::with`]
/// calls. A step can read the current context (`S`) and write to the new part
/// of the context being built (`S::ToBuild`).
///
/// To combine the last state and what was built by the previous steps in a new
/// state, [`PdfBuilder::then`] is used.
///
/// A final step, that has direct access to the global reference allocator and
/// PDF document, can be run with [`PdfBuilder::export_with`].
struct PdfBuilder<S: State> {
    /// The context that has been accumulated so far.
    state: S,
    /// A new part of the context that is currently being built.
    building: S::ToBuild,
    /// A global bump allocator.
    alloc: Ref,
    /// The PDF document that is being written.
    pdf: Pdf,
}

/// Current state of a [`PdfBuilder`].
///
/// Depending how far in the process of building the PDF document we are, we
/// have collected more or less information. The different stages and data that
/// is available at this point are represented by implementors of this trait.
///
/// This design ensures that it is not possible to read data that has not
/// actually been initialized yet.
///
/// The differents states are implemented below, in the order they are executed.
trait State: Sized {
    /// The data that will be constructed by the next steps.
    type ToBuild;
    /// The next state, that is a combination of `Self` and `Self::ToBuild`.
    ///
    /// States form a chain, linked using this associated type. The last type of
    /// the chain points to itself.
    type Next: State;

    /// Start building the new data.
    fn start() -> Self::ToBuild;

    // TODO: add a step to allocate globals instead of having alloc here?
    /// Transition to the next state.
    fn next(self, alloc: &mut Ref, built: Self::ToBuild) -> Self::Next;
}

/// The initial state: we are exploring the document, collecting all resources
/// that will be necessary later.
///
/// The only step here is [`Pages`].
struct BuildContent<'a> {
    document: &'a Document,
}

impl<'a> State for BuildContent<'a> {
    type Next = AllocRefs<'a>;
    type ToBuild = Resources<'a>;

    fn start() -> Self::ToBuild {
        Resources::default()
    }

    fn next(self, alloc: &mut Ref, mut resources: Resources<'a>) -> Self::Next {
        resources.alloc_refs(alloc);
        AllocRefs {
            document: self.document,
            globals: GlobalRefs::new(alloc, self.document.pages.len()),
            resources,
        }
    }
}

/// All the resources that have been collected when traversing the document.
struct Resources<'a> {
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
    patterns: Option<Box<PatternRemapper<'a>>>,
    /// Deduplicates external graphics states used across the document.
    ext_gs: Remapper<ExtGState>,
    /// Deduplicates color glyphs.
    color_fonts: Option<Box<ColorFontMap<'a>>>,
}

impl<'a> Renumber for Resources<'a> {
    fn renumber(&mut self, _old: Ref, _new: Ref) {}
}

impl<'a> Default for Resources<'a> {
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

impl<'a> Resources<'a> {
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

    fn write_with_ref<P>(&self, global_ref: Ref, process: &mut P)
    where
        P: FnMut(&Self, Ref),
    {
        process(self, global_ref);
        if let Some(color_fonts) = &self.color_fonts {
            color_fonts
                .resources
                .write_with_ref(color_fonts.resources_ref.unwrap(), process)
        }
        if let Some(patterns) = &self.patterns {
            patterns
                .resources
                .write_with_ref(patterns.resources_ref.unwrap(), process)
        }
    }

    fn alloc_refs(&mut self, alloc: &mut Ref) {
        if let Some(color_fonts) = &mut self.color_fonts {
            color_fonts.resources_ref = Some(alloc.bump());
            color_fonts.resources.alloc_refs(alloc);
        }

        if let Some(patterns) = &mut self.patterns {
            patterns.resources_ref = Some(alloc.bump());
            patterns.resources.alloc_refs(alloc);
        }
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

impl<'a> State for AllocRefs<'a> {
    type ToBuild = References;

    type Next = WritePageTree<'a>;

    fn start() -> Self::ToBuild {
        References::default()
    }

    fn next(self, _alloc: &mut Ref, references: References) -> Self::Next {
        WritePageTree {
            document: self.document,
            resources: self.resources,
            globals: self.globals,
            references,
        }
    }
}

/// The references that have been assigned to each object.
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

/// At this point, the references have been assigned to all resources. The page
/// tree is going to be written, and given an ID. It is also at this point that
/// the page contents is actually written.
struct WritePageTree<'a> {
    globals: GlobalRefs,
    document: &'a Document,
    resources: Resources<'a>,
    references: References,
}

impl<'a> State for WritePageTree<'a> {
    type ToBuild = PageTreeRef;

    type Next = WriteResources<'a>;

    fn start() -> Self::ToBuild {
        PageTreeRef(Ref::new(1))
    }

    fn next(self, _alloc: &mut Ref, page_tree_ref: PageTreeRef) -> Self::Next {
        WriteResources {
            globals: self.globals,
            document: self.document,
            resources: self.resources,
            references: self.references,
            page_tree_ref: page_tree_ref.0,
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

impl<'a> State for WriteResources<'a> {
    type ToBuild = ();

    type Next = Self;

    fn start() -> Self::ToBuild {}

    fn next(self, _alloc: &mut Ref, (): ()) -> Self::Next {
        self
    }
}

/// An export step, that does not write anything to the file,
/// only collects data.
trait Step<S: State> {
    fn run(&self, state: &S, output: &mut S::ToBuild);
}

/// Same as [`Step`] but also writes a part of the final PDF file.
///
/// Because what is written is local to the current chunk only, the
/// output of this step will be renumbered before being saved.
trait WriteStep<S: State> {
    /// The kind of data that this steps exports.
    ///
    /// This generally corresponds to a field of `S::ToBuild` (or to
    /// all of it).
    type Output: Default + Renumber;

    /// Runs the step.
    ///
    /// References can be allocated using `PdfChunk::alloc`.
    /// The only way to share references with further steps is through
    /// `output`, which is renumbered before being shared.
    fn run(&self, state: &S, chunk: &mut PdfChunk, output: &mut Self::Output);

    /// Save the output of this step in the state, after it has been renumbered
    /// to use global references.
    ///
    /// This function is generally implemented as a single assignement to a
    /// field.
    fn save(context: &mut S::ToBuild, output: Self::Output);
}

/// This implementation exists to be able to call [`PdfBuilder::with`] on a
/// [`Step`].
impl<R: Default + Renumber, S: State<ToBuild = R>, T: Step<S>> WriteStep<S> for T {
    type Output = S::ToBuild;

    fn run(&self, state: &S, _chunk: &mut PdfChunk, output: &mut Self::Output) {
        self.run(state, output);
    }

    fn save(context: &mut <S as State>::ToBuild, output: Self::Output) {
        *context = output;
    }
}

/// A final step, that exports the PDF document.
///
/// It has direct access to the whole state, the PDF document, and the global
/// allocator.
trait FinalStep<S: State> {
    fn run(&self, state: S, pdf: &mut Pdf, alloc: &mut Ref);
}

impl<'a> PdfBuilder<BuildContent<'a>> {
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

impl<S: State> PdfBuilder<S> {
    /// Runs a step with the current state.
    fn with<W: WriteStep<S>>(mut self, step: W) -> Self {
        // Any reference below that value was already allocated before and
        // should not be rewritten. Anything above was allocated in the current
        // chunk, and should be remapped.
        //
        // This is a constant (large enough to avoid collisions) and not
        // dependant on self.alloc to allow for better memoization of steps, if
        // needed in the future.
        const TEMPORARY_REFS_START: i32 = 1_000_000_000;

        fn write<S: State, W: WriteStep<S>>(
            chunk: &mut PdfChunk,
            ctx: &S,
            output: &mut W::Output,
            step: &W,
        ) {
            step.run(ctx, chunk, output);
        }

        let mut output = Default::default();
        let mut chunk: PdfChunk = PdfChunk::new(TEMPORARY_REFS_START);
        let mut mapping = HashMap::new();
        write(&mut chunk, &self.state, &mut output, &step);

        chunk.renumber_into(&mut self.pdf, |r| {
            if r.get() < TEMPORARY_REFS_START {
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

    /// Transitions to the next state.
    fn then(mut self) -> PdfBuilder<S::Next> {
        PdfBuilder {
            state: self.state.next(&mut self.alloc, self.building),
            building: S::Next::start(),
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

// TODO: introduce GlobalRef for that purpose, instead of having PageTreeRef and
// others (+more type safety I guess)?
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
