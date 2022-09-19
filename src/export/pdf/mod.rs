//! Exporting into PDF documents.

mod font;
mod image;
mod outline;
mod page;

use std::cmp::Eq;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use pdf_writer::types::Direction;
use pdf_writer::{Finish, Name, PdfWriter, Ref, TextStr};

use self::outline::{Heading, HeadingNode};
use self::page::Page;
use crate::font::{FontId, FontStore};
use crate::frame::Frame;
use crate::geom::{Dir, Em, Length};
use crate::image::{ImageId, ImageStore};
use crate::library::text::Lang;
use crate::Context;

/// Export a collection of frames into a PDF file.
///
/// This creates one page per frame. In addition to the frames, you need to pass
/// in the context used during compilation so that fonts and images can be
/// included in the PDF.
///
/// Returns the raw bytes making up the PDF file.
pub fn pdf(ctx: &Context, frames: &[Frame]) -> Vec<u8> {
    let mut ctx = PdfContext::new(ctx);
    page::construct_pages(&mut ctx, frames);
    font::write_fonts(&mut ctx);
    image::write_images(&mut ctx);
    page::write_page_tree(&mut ctx);
    write_catalog(&mut ctx);
    ctx.writer.finish()
}

/// Identifies the color space definitions.
const SRGB: Name<'static> = Name(b"srgb");
const D65_GRAY: Name<'static> = Name(b"d65gray");

/// Context for exporting a whole PDF document.
pub struct PdfContext<'a> {
    writer: PdfWriter,
    fonts: &'a FontStore,
    images: &'a ImageStore,
    pages: Vec<Page>,
    page_heights: Vec<f32>,
    alloc: Ref,
    page_tree_ref: Ref,
    font_refs: Vec<Ref>,
    image_refs: Vec<Ref>,
    page_refs: Vec<Ref>,
    font_map: Remapper<FontId>,
    image_map: Remapper<ImageId>,
    glyph_sets: HashMap<FontId, HashSet<u16>>,
    languages: HashMap<Lang, usize>,
    heading_tree: Vec<HeadingNode>,
}

impl<'a> PdfContext<'a> {
    fn new(ctx: &'a Context) -> Self {
        let mut alloc = Ref::new(1);
        let page_tree_ref = alloc.bump();
        Self {
            writer: PdfWriter::new(),
            fonts: &ctx.fonts,
            images: &ctx.images,
            pages: vec![],
            page_heights: vec![],
            alloc,
            page_tree_ref,
            page_refs: vec![],
            font_refs: vec![],
            image_refs: vec![],
            font_map: Remapper::new(),
            image_map: Remapper::new(),
            glyph_sets: HashMap::new(),
            languages: HashMap::new(),
            heading_tree: vec![],
        }
    }
}

/// Write the document catalog.
fn write_catalog(ctx: &mut PdfContext) {
    // Build the outline tree.
    let outline_root_id = (!ctx.heading_tree.is_empty()).then(|| ctx.alloc.bump());
    let outline_start_ref = ctx.alloc;
    let len = ctx.heading_tree.len();
    let mut prev_ref = None;

    for (i, node) in std::mem::take(&mut ctx.heading_tree).iter().enumerate() {
        prev_ref = Some(outline::write_outline_item(
            ctx,
            node,
            outline_root_id.unwrap(),
            prev_ref,
            i + 1 == len,
        ));
    }

    if let Some(outline_root_id) = outline_root_id {
        let mut outline_root = ctx.writer.outline(outline_root_id);
        outline_root.first(outline_start_ref);
        outline_root.last(Ref::new(ctx.alloc.get() - 1));
        outline_root.count(ctx.heading_tree.len() as i32);
    }

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

    // Write the document information.
    ctx.writer.document_info(ctx.alloc.bump()).creator(TextStr("Typst"));

    // Write the document catalog.
    let mut catalog = ctx.writer.catalog(ctx.alloc.bump());
    catalog.pages(ctx.page_tree_ref);
    catalog.viewer_preferences().direction(dir);

    if let Some(outline_root_id) = outline_root_id {
        catalog.outlines(outline_root_id);
    }

    if let Some(lang) = lang {
        catalog.lang(TextStr(lang.as_str()));
    }

    catalog.finish();
}

/// Compress data with the DEFLATE algorithm.
fn deflate(data: &[u8]) -> Vec<u8> {
    const COMPRESSION_LEVEL: u8 = 6;
    miniz_oxide::deflate::compress_to_vec_zlib(data, COMPRESSION_LEVEL)
}

/// Assigns new, consecutive PDF-internal indices to things.
struct Remapper<Index> {
    /// Forwards from the old indices to the new pdf indices.
    to_pdf: HashMap<Index, usize>,
    /// Backwards from the pdf indices to the old indices.
    to_layout: Vec<Index>,
}

impl<Index> Remapper<Index>
where
    Index: Copy + Eq + Hash,
{
    fn new() -> Self {
        Self {
            to_pdf: HashMap::new(),
            to_layout: vec![],
        }
    }

    fn insert(&mut self, index: Index) {
        let to_layout = &mut self.to_layout;
        self.to_pdf.entry(index).or_insert_with(|| {
            let pdf_index = to_layout.len();
            to_layout.push(index);
            pdf_index
        });
    }

    fn map(&self, index: Index) -> usize {
        self.to_pdf[&index]
    }

    fn pdf_indices<'a>(
        &'a self,
        refs: &'a [Ref],
    ) -> impl Iterator<Item = (Ref, usize)> + 'a {
        refs.iter().copied().zip(0 .. self.to_pdf.len())
    }

    fn layout_indices(&self) -> impl Iterator<Item = Index> + '_ {
        self.to_layout.iter().copied()
    }
}

/// Additional methods for [`Length`].
trait LengthExt {
    /// Convert an em length to a number of points.
    fn to_f32(self) -> f32;
}

impl LengthExt for Length {
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
