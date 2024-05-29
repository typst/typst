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

use std::collections::HashMap;
use std::hash::Hash;
use std::ops::{Deref, DerefMut};

use base64::Engine;
use pdf_writer::{Chunk, Pdf, Ref};
use typst::foundations::{Datetime, Smart};
use typst::layout::{Abs, Em, PageRanges, Transform};
use typst::model::Document;
use typst::text::Font;
use typst::utils::Deferred;
use typst::visualize::Image;

use crate::catalog::write_catalog;
use crate::color::{alloc_color_functions_refs, ColorFunctionRefs};
use crate::color_font::{write_color_fonts, ColorFontSlice};
use crate::extg::{write_graphic_states, ExtGState};
use crate::font::write_fonts;
use crate::gradient::{write_gradients, PdfGradient};
use crate::image::write_images;
use crate::named_destination::{write_named_destinations, NamedDestinations};
use crate::page::{alloc_page_refs, traverse_pages, write_page_tree, EncodedPage};
use crate::pattern::{write_patterns, PdfPattern};
use crate::resources::{
    alloc_resources_refs, write_resource_dictionaries, Resources, ResourcesRefs,
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
///
/// The `page_ranges` option specifies which ranges of pages should be exported
/// in the PDF. When `None`, all pages should be exported.
#[typst_macros::time(name = "pdf")]
pub fn pdf(
    document: &Document,
    ident: Smart<&str>,
    timestamp: Option<Datetime>,
    page_ranges: Option<PageRanges>,
) -> Vec<u8> {
    PdfBuilder::new(document, page_ranges)
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
///
/// Phase after phase, this state will be transformed. Each phase corresponds to
/// a call to the [eponymous function](`PdfBuilder::phase`) and produces a new
/// part of the state, that will be aggregated with all other information, for
/// consumption during the next phase.
///
/// In other words: this struct follows the **typestate pattern**. This prevents
/// you from using data that is not yet available, at the type level.
///
/// Each phase consists of processes, that can read the state of the previous
/// phases, and construct a part of the new state.
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
struct WithDocument<'a> {
    /// The Typst document that is exported.
    document: &'a Document,
    /// Page ranges to export.
    /// When `None`, all pages are exported.
    exported_pages: Option<PageRanges>,
}

/// At this point, resources were listed, but they don't have any reference
/// associated with them.
///
/// This phase allocates some global references.
struct WithResources<'a> {
    document: &'a Document,
    exported_pages: Option<PageRanges>,
    /// The content of the pages encoded as PDF content streams.
    ///
    /// The pages are at the index corresponding to their page number, but they
    /// may be `None` if they are not in the range specified by
    /// `exported_pages`.
    pages: Vec<Option<EncodedPage>>,
    /// The PDF resources that are used in the content of the pages.
    resources: Resources<()>,
}

/// Global references.
struct GlobalRefs {
    /// References for color conversion functions.
    color_functions: ColorFunctionRefs,
    /// Reference for pages.
    ///
    /// Items of this vector are `None` if the corresponding page is not
    /// exported.
    pages: Vec<Option<Ref>>,
    /// References for the resource dictionaries.
    resources: ResourcesRefs,
}

impl<'a> From<(WithDocument<'a>, (Vec<Option<EncodedPage>>, Resources<()>))>
    for WithResources<'a>
{
    fn from(
        (previous, (pages, resources)): (
            WithDocument<'a>,
            (Vec<Option<EncodedPage>>, Resources<()>),
        ),
    ) -> Self {
        Self {
            document: previous.document,
            exported_pages: previous.exported_pages,
            pages,
            resources,
        }
    }
}

/// At this point, the resources have been collected, and global references have
/// been allocated.
///
/// We are now writing objects corresponding to resources, and giving them references,
/// that will be collected in [`References`].
struct WithGlobalRefs<'a> {
    document: &'a Document,
    exported_pages: Option<PageRanges>,
    pages: Vec<Option<EncodedPage>>,
    /// Resources are the same as in previous phases, but each dictionary now has a reference.
    resources: Resources,
    /// Global references that were just allocated.
    globals: GlobalRefs,
}

impl<'a> From<(WithResources<'a>, GlobalRefs)> for WithGlobalRefs<'a> {
    fn from((previous, globals): (WithResources<'a>, GlobalRefs)) -> Self {
        Self {
            document: previous.document,
            exported_pages: previous.exported_pages,
            pages: previous.pages,
            resources: previous.resources.with_refs(&globals.resources),
            globals,
        }
    }
}

/// The references that have been assigned to each object.
struct References {
    /// List of named destinations, each with an ID.
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
    patterns: HashMap<PdfPattern, Ref>,
    /// The IDs of written external graphics states.
    ext_gs: HashMap<ExtGState, Ref>,
}

/// At this point, the references have been assigned to all resources. The page
/// tree is going to be written, and given a reference. It is also at this point that
/// the page contents is actually written.
struct WithRefs<'a> {
    globals: GlobalRefs,
    document: &'a Document,
    pages: Vec<Option<EncodedPage>>,
    exported_pages: Option<PageRanges>,
    resources: Resources,
    /// References that were allocated for resources.
    references: References,
}

impl<'a> From<(WithGlobalRefs<'a>, References)> for WithRefs<'a> {
    fn from((previous, references): (WithGlobalRefs<'a>, References)) -> Self {
        Self {
            globals: previous.globals,
            exported_pages: previous.exported_pages,
            document: previous.document,
            pages: previous.pages,
            resources: previous.resources,
            references,
        }
    }
}

/// In this phase, we write resource dictionaries.
///
/// Each sub-resource gets its own isolated resource dictionary.
struct WithEverything<'a> {
    globals: GlobalRefs,
    document: &'a Document,
    pages: Vec<Option<EncodedPage>>,
    exported_pages: Option<PageRanges>,
    resources: Resources,
    references: References,
    /// Reference that was allocated for the page tree.
    page_tree_ref: Ref,
}

impl<'a> From<(WithEverything<'a>, ())> for WithEverything<'a> {
    fn from((this, _): (WithEverything<'a>, ())) -> Self {
        this
    }
}

impl<'a> From<(WithRefs<'a>, Ref)> for WithEverything<'a> {
    fn from((previous, page_tree_ref): (WithRefs<'a>, Ref)) -> Self {
        Self {
            exported_pages: previous.exported_pages,
            globals: previous.globals,
            document: previous.document,
            resources: previous.resources,
            references: previous.references,
            pages: previous.pages,
            page_tree_ref,
        }
    }
}

impl<'a> PdfBuilder<WithDocument<'a>> {
    /// Start building a PDF for a Typst document.
    fn new(document: &'a Document, exported_pages: Option<PageRanges>) -> Self {
        Self {
            alloc: Ref::new(1),
            pdf: Pdf::new(),
            state: WithDocument { document, exported_pages },
        }
    }
}

impl<S> PdfBuilder<S> {
    /// Start a new phase, and save its output in the global state.
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
        let (chunk, mut output) = process(&self.state);
        // Allocate a final reference for each temporary one
        let allocated = chunk.alloc.get() - TEMPORARY_REFS_START;
        let offset = TEMPORARY_REFS_START - self.alloc.get();

        // Merge the chunk into the PDF, using the new references
        chunk.renumber_into(&mut self.pdf, |mut r| {
            r.renumber(offset);

            r
        });

        // Also update the references in the output
        output.renumber(offset);

        self.alloc = Ref::new(self.alloc.get() + allocated);

        output
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
}

/// A reference or collection of references that can be re-numbered,
/// to become valid in a global scope.
trait Renumber {
    /// Renumber this value by shifting any references it contains by `offset`.
    fn renumber(&mut self, offset: i32);
}

impl Renumber for () {
    fn renumber(&mut self, _offset: i32) {}
}

impl Renumber for Ref {
    fn renumber(&mut self, offset: i32) {
        if self.get() >= TEMPORARY_REFS_START {
            *self = Ref::new(self.get() - offset);
        }
    }
}

impl<R: Renumber> Renumber for Vec<R> {
    fn renumber(&mut self, offset: i32) {
        for item in self {
            item.renumber(offset);
        }
    }
}

impl<T: Eq + Hash, R: Renumber> Renumber for HashMap<T, R> {
    fn renumber(&mut self, offset: i32) {
        for v in self.values_mut() {
            v.renumber(offset);
        }
    }
}

impl<R: Renumber> Renumber for Option<R> {
    fn renumber(&mut self, offset: i32) {
        if let Some(r) = self {
            r.renumber(offset)
        }
    }
}

impl<T, R: Renumber> Renumber for (T, R) {
    fn renumber(&mut self, offset: i32) {
        self.1.renumber(offset)
    }
}

/// A portion of a PDF file.
struct PdfChunk {
    /// The actual chunk.
    chunk: Chunk,
    /// A local allocator.
    alloc: Ref,
}

/// Any reference below that value was already allocated before and
/// should not be rewritten. Anything above was allocated in the current
/// chunk, and should be remapped.
///
/// This is a constant (large enough to avoid collisions) and not
/// dependant on self.alloc to allow for better memoization of steps, if
/// needed in the future.
const TEMPORARY_REFS_START: i32 = 1_000_000_000;

/// A part of a PDF document.
impl PdfChunk {
    /// Start writing a new part of the document.
    fn new() -> Self {
        PdfChunk {
            chunk: Chunk::new(),
            alloc: Ref::new(TEMPORARY_REFS_START),
        }
    }

    /// Allocate a reference that is valid in the context of this chunk.
    ///
    /// References allocated with this function should be [renumbered](`Renumber::renumber`)
    /// before being used in other chunks. This is done automatically if these
    /// references are stored in the global `PdfBuilder` state.
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
        .encode(typst::utils::hash128(value).to_be_bytes())
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
