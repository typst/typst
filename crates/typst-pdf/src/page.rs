use std::collections::HashMap;
use std::num::NonZeroUsize;

use ecow::EcoString;
use pdf_writer::{
    types::{ActionType, AnnotationFlags, AnnotationType, NumberingStyle},
    Filter, Finish, Name, Rect, Ref, Str,
};

use typst::foundations::Label;
use typst::introspection::Location;
use typst::layout::{Abs, Frame};
use typst::model::{Destination, Numbering};
use typst::text::Case;

use crate::{content, AbsExt, PdfChunk, Renumber, WithDocument, WithRefs, WithResources};
use crate::{font::improve_glyph_sets, Resources};

/// Construct page objects.
#[typst_macros::time(name = "construct pages")]
pub fn traverse_pages(
    state: &WithDocument,
) -> (PdfChunk, (Vec<EncodedPage>, Resources<()>)) {
    let mut resources = Resources::default();
    let mut pages = Vec::with_capacity(state.document.pages.len());
    for page in &state.document.pages {
        let mut encoded = construct_page(&mut resources, &page.frame);
        encoded.label = page
            .numbering
            .as_ref()
            .and_then(|num| PdfPageLabel::generate(num, page.number));
        pages.push(encoded);
    }

    improve_glyph_sets(&mut resources.glyph_sets);

    (PdfChunk::new(), (pages, resources))
}

/// Construct a page object.
#[typst_macros::time(name = "construct page")]
fn construct_page(out: &mut Resources<()>, frame: &Frame) -> EncodedPage {
    let content = content::build(out, frame);

    EncodedPage { content, label: None }
}

pub fn alloc_page_refs(context: &WithResources) -> (PdfChunk, Vec<Ref>) {
    let mut chunk = PdfChunk::new();
    let page_refs = context.document.pages.iter().map(|_| chunk.alloc()).collect();
    (chunk, page_refs)
}

pub struct PageTreeRef(pub Ref);

impl Renumber for PageTreeRef {
    fn renumber(&mut self, mapping: &HashMap<Ref, Ref>) {
        self.0.renumber(mapping)
    }
}

/// Write the page tree.
pub fn write_page_tree(ctx: &WithRefs) -> (PdfChunk, PageTreeRef) {
    let mut chunk = PdfChunk::new();
    let page_tree_ref = chunk.alloc.bump();

    for i in 0..ctx.pages.len() {
        let content_id = chunk.alloc.bump();
        write_page(
            &mut chunk,
            ctx,
            content_id,
            page_tree_ref,
            &ctx.references.named_destinations.loc_to_dest,
            i,
        );
    }

    chunk
        .pages(page_tree_ref)
        .count(ctx.pages.len() as i32)
        .kids(ctx.globals.pages.iter().copied());

    (chunk, PageTreeRef(page_tree_ref))
}

/// Write a page tree node.
fn write_page(
    chunk: &mut PdfChunk,
    ctx: &WithRefs,
    content_id: Ref,
    page_tree_ref: Ref,
    loc_to_dest: &HashMap<Location, Label>,
    i: usize,
) {
    let page = &ctx.pages[i];

    let global_resources_ref = ctx.resources.reference;
    let mut page_writer = chunk.page(ctx.globals.pages[i]);
    page_writer.parent(page_tree_ref);

    let w = page.content.size.x.to_f32();
    let h = page.content.size.y.to_f32();
    page_writer.media_box(Rect::new(0.0, 0.0, w, h));
    page_writer.contents(content_id);
    page_writer.pair(Name(b"Resources"), global_resources_ref);

    if page.content.uses_opacities {
        page_writer
            .group()
            .transparency()
            .isolated(false)
            .knockout(false)
            .color_space()
            .srgb();
    }

    let mut annotations = page_writer.annotations();
    for (dest, rect) in &page.content.links {
        let mut annotation = annotations.push();
        annotation.subtype(AnnotationType::Link).rect(*rect);
        annotation.border(0.0, 0.0, 0.0, None).flags(AnnotationFlags::PRINT);

        let pos = match dest {
            Destination::Url(uri) => {
                annotation
                    .action()
                    .action_type(ActionType::Uri)
                    .uri(Str(uri.as_bytes()));
                continue;
            }
            Destination::Position(pos) => *pos,
            Destination::Location(loc) => {
                if let Some(key) = loc_to_dest.get(loc) {
                    annotation
                        .action()
                        .action_type(ActionType::GoTo)
                        // `key` must be a `Str`, not a `Name`.
                        .pair(Name(b"D"), Str(key.as_str().as_bytes()));
                    continue;
                } else {
                    ctx.document.introspector.position(*loc)
                }
            }
        };

        let index = pos.page.get() - 1;
        let y = (pos.point.y - Abs::pt(10.0)).max(Abs::zero());

        if let Some(page) = ctx.pages.get(index) {
            annotation
                .action()
                .action_type(ActionType::GoTo)
                .destination()
                .page(ctx.globals.pages[index])
                .xyz(pos.point.x.to_f32(), (page.content.size.y - y).to_f32(), None);
        }
    }

    annotations.finish();
    page_writer.finish();

    chunk
        .stream(content_id, page.content.content.wait())
        .filter(Filter::FlateDecode);
}

/// Specification for a PDF page label.
#[derive(Debug, Clone, PartialEq, Hash, Default)]
pub(crate) struct PdfPageLabel {
    /// Can be any string or none. Will always be prepended to the numbering style.
    pub prefix: Option<EcoString>,
    /// Based on the numbering pattern.
    ///
    /// If `None` or numbering is a function, the field will be empty.
    pub style: Option<PdfPageLabelStyle>,
    /// Offset for the page label start.
    ///
    /// Describes where to start counting from when setting a style.
    /// (Has to be greater or equal than 1)
    pub offset: Option<NonZeroUsize>,
}

/// A PDF page label number style.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum PdfPageLabelStyle {
    /// Decimal arabic numerals (1, 2, 3).
    Arabic,
    /// Lowercase roman numerals (i, ii, iii).
    LowerRoman,
    /// Uppercase roman numerals (I, II, III).
    UpperRoman,
    /// Lowercase letters (`a` to `z` for the first 26 pages,
    /// `aa` to `zz` and so on for the next).
    LowerAlpha,
    /// Uppercase letters (`A` to `Z` for the first 26 pages,
    /// `AA` to `ZZ` and so on for the next).
    UpperAlpha,
}

impl PdfPageLabel {
    /// Create a new `PdfNumbering` from a `Numbering` applied to a page
    /// number.
    fn generate(numbering: &Numbering, number: usize) -> Option<PdfPageLabel> {
        let Numbering::Pattern(pat) = numbering else {
            return None;
        };

        let (prefix, kind, case) = pat.pieces.first()?;

        // If there is a suffix, we cannot use the common style optimisation,
        // since PDF does not provide a suffix field.
        let mut style = None;
        if pat.suffix.is_empty() {
            use {typst::model::NumberingKind as Kind, PdfPageLabelStyle as Style};
            match (kind, case) {
                (Kind::Arabic, _) => style = Some(Style::Arabic),
                (Kind::Roman, Case::Lower) => style = Some(Style::LowerRoman),
                (Kind::Roman, Case::Upper) => style = Some(Style::UpperRoman),
                (Kind::Letter, Case::Lower) if number <= 26 => {
                    style = Some(Style::LowerAlpha)
                }
                (Kind::Letter, Case::Upper) if number <= 26 => {
                    style = Some(Style::UpperAlpha)
                }
                _ => {}
            }
        }

        // Prefix and offset depend on the style: If it is supported by the PDF
        // spec, we use the given prefix and an offset. Otherwise, everything
        // goes into prefix.
        let prefix = if style.is_none() {
            Some(pat.apply(&[number]))
        } else {
            (!prefix.is_empty()).then(|| prefix.clone())
        };

        let offset = style.and(NonZeroUsize::new(number));
        Some(PdfPageLabel { prefix, style, offset })
    }
}

impl PdfPageLabelStyle {
    pub fn to_pdf_numbering_style(self) -> NumberingStyle {
        match self {
            PdfPageLabelStyle::Arabic => NumberingStyle::Arabic,
            PdfPageLabelStyle::LowerRoman => NumberingStyle::LowerRoman,
            PdfPageLabelStyle::UpperRoman => NumberingStyle::UpperRoman,
            PdfPageLabelStyle::LowerAlpha => NumberingStyle::LowerAlpha,
            PdfPageLabelStyle::UpperAlpha => NumberingStyle::UpperAlpha,
        }
    }
}

/// Data for an exported page.
pub struct EncodedPage {
    pub content: content::Encoded,
    pub label: Option<PdfPageLabel>,
}
