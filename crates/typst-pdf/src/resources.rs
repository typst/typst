//! PDF resources.
//!
//! Resources are defined in dictionaries. They map identifiers such as `Im0` to
//! a PDF reference. Each [content stream] is associated with a resource dictionary.
//! The identifiers defined in the resources can then be used in content streams.
//!
//! [content stream]: `crate::content`

use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;

use ecow::{eco_format, EcoString};
use pdf_writer::{Dict, Finish, Name, Ref};
use typst::text::Lang;
use typst::{text::Font, utils::Deferred, visualize::Image};

use crate::{
    color::ColorSpaces, color_font::ColorFontMap, extg::ExtGState, gradient::PdfGradient,
    image::EncodedImage, pattern::PatternRemapper, PdfChunk, Renumber, WithEverything,
    WithResources,
};

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
pub struct Resources<R = Ref> {
    /// The global reference to this resource dictionary, or `()` if it has not
    /// been allocated yet.
    pub reference: R,

    /// Handles color space writing.
    pub colors: ColorSpaces,

    /// Deduplicates fonts used across the document.
    pub fonts: Remapper<Font>,
    /// Deduplicates images used across the document.
    pub images: Remapper<Image>,
    /// Handles to deferred image conversions.
    pub deferred_images: HashMap<usize, Deferred<EncodedImage>>,
    /// Deduplicates gradients used across the document.
    pub gradients: Remapper<PdfGradient>,
    /// Deduplicates patterns used across the document.
    pub patterns: Option<Box<PatternRemapper<R>>>,
    /// Deduplicates external graphics states used across the document.
    pub ext_gs: Remapper<ExtGState>,
    /// Deduplicates color glyphs.
    pub color_fonts: Option<Box<ColorFontMap<R>>>,

    // The fields below do not correspond to actual resources that will be
    // written in a dictionary, but are more meta-data about resources that
    // can't really live somewhere else.
    /// The number of glyphs for all referenced languages in the content stream.
    /// We keep track of this to determine the main document language.
    /// BTreeMap is used to write sorted list of languages to metadata.
    pub languages: BTreeMap<Lang, usize>,

    /// For each font a mapping from used glyphs to their text representation.
    /// May contain multiple chars in case of ligatures or similar things. The
    /// same glyph can have a different text representation within one document,
    /// then we just save the first one. The resulting strings are used for the
    /// PDF's /ToUnicode map for glyphs that don't have an entry in the font's
    /// cmap. This is important for copy-paste and searching.
    pub glyph_sets: HashMap<Font, BTreeMap<u16, EcoString>>,
}

impl<R: Renumber> Renumber for Resources<R> {
    fn renumber(&mut self, offset: i32) {
        self.reference.renumber(offset);

        if let Some(color_fonts) = &mut self.color_fonts {
            color_fonts.resources.renumber(offset);
        }

        if let Some(patterns) = &mut self.patterns {
            patterns.resources.renumber(offset);
        }
    }
}

impl Default for Resources<()> {
    fn default() -> Self {
        Resources {
            reference: (),
            colors: ColorSpaces::default(),
            fonts: Remapper::new("F"),
            images: Remapper::new("Im"),
            deferred_images: HashMap::new(),
            gradients: Remapper::new("Gr"),
            patterns: None,
            ext_gs: Remapper::new("Gs"),
            color_fonts: None,
            languages: BTreeMap::new(),
            glyph_sets: HashMap::new(),
        }
    }
}

impl Resources<()> {
    /// Associate a reference with this resource dictionary (and do so
    /// recursively for sub-resources).
    pub fn with_refs(self, refs: &ResourcesRefs) -> Resources<Ref> {
        Resources {
            reference: refs.reference,
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
            languages: self.languages,
            glyph_sets: self.glyph_sets,
        }
    }
}

impl<R> Resources<R> {
    /// Run a function on this resource dictionary and all
    /// of its sub-resources.
    pub fn traverse<P>(&self, process: &mut P)
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

/// References for a resource tree.
///
/// This structure is a tree too, that should have the same structure as the
/// corresponding `Resources`.
pub struct ResourcesRefs {
    pub reference: Ref,
    pub color_fonts: Option<Box<ResourcesRefs>>,
    pub patterns: Option<Box<ResourcesRefs>>,
}

impl Renumber for ResourcesRefs {
    fn renumber(&mut self, offset: i32) {
        self.reference.renumber(offset);
        if let Some(color_fonts) = &mut self.color_fonts {
            color_fonts.renumber(offset);
        }
        if let Some(patterns) = &mut self.patterns {
            patterns.renumber(offset);
        }
    }
}

/// Allocate references for all resource dictionaries.
pub fn alloc_resources_refs(context: &WithResources) -> (PdfChunk, ResourcesRefs) {
    let mut chunk = PdfChunk::new();
    /// Recursively explore resource dictionaries and assign them references.
    fn refs_for(resources: &Resources<()>, chunk: &mut PdfChunk) -> ResourcesRefs {
        ResourcesRefs {
            reference: chunk.alloc(),
            color_fonts: resources
                .color_fonts
                .as_ref()
                .map(|c| Box::new(refs_for(&c.resources, chunk))),
            patterns: resources
                .patterns
                .as_ref()
                .map(|p| Box::new(refs_for(&p.resources, chunk))),
        }
    }

    let refs = refs_for(&context.resources, &mut chunk);
    (chunk, refs)
}

/// Write the resource dictionaries that will be referenced by all pages.
///
/// We add a reference to this dictionary to each page individually instead of
/// to the root node of the page tree because using the resource inheritance
/// feature breaks PDF merging with Apple Preview.
///
/// Also write resource dictionaries for Type3 fonts and patterns.
pub fn write_resource_dictionaries(ctx: &WithEverything) -> (PdfChunk, ()) {
    let mut chunk = PdfChunk::new();
    let mut used_color_spaces = ColorSpaces::default();

    ctx.resources.traverse(&mut |resources| {
        used_color_spaces.merge(&resources.colors);

        let images_ref = chunk.alloc.bump();
        let patterns_ref = chunk.alloc.bump();
        let ext_gs_states_ref = chunk.alloc.bump();
        let color_spaces_ref = chunk.alloc.bump();

        let mut color_font_slices = Vec::new();
        let mut color_font_numbers = HashMap::new();
        if let Some(color_fonts) = &resources.color_fonts {
            for (_, font_slice) in color_fonts.iter() {
                color_font_numbers.insert(font_slice.clone(), color_font_slices.len());
                color_font_slices.push(font_slice);
            }
        }
        let color_font_remapper = Remapper {
            prefix: "Cf",
            to_pdf: color_font_numbers,
            to_items: color_font_slices,
        };

        resources
            .images
            .write(&ctx.references.images, &mut chunk.indirect(images_ref).dict());

        let mut patterns_dict = chunk.indirect(patterns_ref).dict();
        resources
            .gradients
            .write(&ctx.references.gradients, &mut patterns_dict);
        if let Some(p) = &resources.patterns {
            p.remapper.write(&ctx.references.patterns, &mut patterns_dict);
        }
        patterns_dict.finish();

        resources
            .ext_gs
            .write(&ctx.references.ext_gs, &mut chunk.indirect(ext_gs_states_ref).dict());

        let mut res_dict = chunk
            .indirect(resources.reference)
            .start::<pdf_writer::writers::Resources>();
        res_dict.pair(Name(b"XObject"), images_ref);
        res_dict.pair(Name(b"Pattern"), patterns_ref);
        res_dict.pair(Name(b"ExtGState"), ext_gs_states_ref);
        res_dict.pair(Name(b"ColorSpace"), color_spaces_ref);

        // TODO: can't this be an indirect reference too?
        let mut fonts_dict = res_dict.fonts();
        resources.fonts.write(&ctx.references.fonts, &mut fonts_dict);
        color_font_remapper.write(&ctx.references.color_fonts, &mut fonts_dict);
        fonts_dict.finish();

        res_dict.finish();

        let color_spaces = chunk.indirect(color_spaces_ref).dict();
        resources
            .colors
            .write_color_spaces(color_spaces, &ctx.globals.color_functions);
    });

    used_color_spaces.write_functions(&mut chunk, &ctx.globals.color_functions);

    (chunk, ())
}

/// Assigns new, consecutive PDF-internal indices to items.
pub struct Remapper<T> {
    /// The prefix to use when naming these resources.
    prefix: &'static str,
    /// Forwards from the items to the pdf indices.
    to_pdf: HashMap<T, usize>,
    /// Backwards from the pdf indices to the items.
    to_items: Vec<T>,
}

impl<T> Remapper<T>
where
    T: Eq + Hash + Clone,
{
    /// Create an empty mapping.
    pub fn new(prefix: &'static str) -> Self {
        Self { prefix, to_pdf: HashMap::new(), to_items: vec![] }
    }

    /// Insert an item in the mapping if it was not already present.
    pub fn insert(&mut self, item: T) -> usize {
        let to_layout = &mut self.to_items;
        *self.to_pdf.entry(item.clone()).or_insert_with(|| {
            let pdf_index = to_layout.len();
            to_layout.push(item);
            pdf_index
        })
    }

    /// All items in this
    pub fn items(&self) -> impl Iterator<Item = &T> + '_ {
        self.to_items.iter()
    }

    /// Write this list of items in a Resource dictionary.
    fn write(&self, mapping: &HashMap<T, Ref>, dict: &mut Dict) {
        for (number, item) in self.items().enumerate() {
            let name = eco_format!("{}{}", self.prefix, number);
            let reference = mapping[item];
            dict.pair(Name(name.as_bytes()), reference);
        }
    }
}
