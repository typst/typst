use std::num::NonZeroUsize;

use crate::{append_chunk, content, AbsExt, PdfContext};
use ecow::{eco_format, EcoString};
use pdf_writer::types::{ActionType, AnnotationFlags, AnnotationType, NumberingStyle};
use pdf_writer::writers::{PageLabel, Resources};
use pdf_writer::{Chunk, Filter, Finish, Name, Rect, Ref, Str, TextStr};
use typst::layout::{Abs, Frame, Page};
use typst::model::{Destination, Numbering};
use typst::text::Case;

/// Construct page objects.
#[typst_macros::time(name = "construct pages")]
pub(crate) fn construct_pages<'a>(ctx: &'a mut PdfContext, pages: &[Page]) {
    let mut alloc = Ref::new(1);
    for page in pages {
        let (page_ref, mut encoded) = construct_page(ctx, &mut alloc, &page.frame);
        encoded.label = page
            .numbering
            .as_ref()
            .and_then(|num| PdfPageLabel::generate(num, page.number));
        ctx.page_refs.push(page_ref);
        ctx.pages.push(encoded);
    }
}

/// Construct a page object.
#[typst_macros::time(name = "construct page")]
pub(crate) fn construct_page(
    ctx: &mut PdfContext,
    alloc: &mut Ref,
    frame: &Frame,
) -> (Ref, EncodedPage) {
    let page_ref = alloc.bump();

    let content = content::build(ctx, frame);

    let page = EncodedPage { content, id: page_ref, label: None };

    (page_ref, page)
}

/// Write the page tree.
#[must_use]
pub(crate) fn write_page_tree(ctx: &mut PdfContext<Ref>) -> Chunk {
    let mut chunk = Chunk::new();
    let mut alloc = Ref::new(1);

    for i in 0..ctx.pages.len() {
        let page_chunk = write_page(ctx, i);
        append_chunk(&mut alloc, &mut chunk, page_chunk);
    }

    chunk
        .pages(ctx.page_tree_ref)
        .count(ctx.page_refs.len() as i32)
        .kids(ctx.page_refs.iter().copied());
    chunk
}

/// Write the global resource dictionary that will be referenced by all pages.
///
/// We add a reference to this dictionary to each page individually instead of
/// to the root node of the page tree because using the resource inheritance
/// feature breaks PDF merging with Apple Preview.
#[must_use]
pub(crate) fn write_global_resources(ctx: &mut PdfContext<Ref>) -> Chunk {
    let mut chunk = Chunk::new();
    let mut alloc = Ref::new(1);
    let images_ref = alloc.bump();
    let patterns_ref = alloc.bump();
    let ext_gs_states_ref = alloc.bump();
    let color_spaces_ref = alloc.bump();

    let mut images = chunk.indirect(images_ref).dict();
    for (image_ref, im) in ctx.image_map.pdf_indices(&ctx.image_refs) {
        let name = eco_format!("Im{}", im);
        images.pair(Name(name.as_bytes()), image_ref);
    }
    images.finish();

    let mut patterns = chunk.indirect(patterns_ref).dict();
    for (gradient_ref, gr) in ctx.gradient_map.pdf_indices(&ctx.gradient_refs) {
        let name = eco_format!("Gr{}", gr);
        patterns.pair(Name(name.as_bytes()), gradient_ref);
    }

    for (pattern_ref, p) in ctx.pattern_map.pdf_indices(&ctx.pattern_refs) {
        let name = eco_format!("P{}", p);
        patterns.pair(Name(name.as_bytes()), pattern_ref);
    }
    patterns.finish();

    let mut ext_gs_states = chunk.indirect(ext_gs_states_ref).dict();
    for (gs_ref, gs) in ctx.extg_map.pdf_indices(&ctx.ext_gs_refs) {
        let name = eco_format!("Gs{}", gs);
        ext_gs_states.pair(Name(name.as_bytes()), gs_ref);
    }
    ext_gs_states.finish();

    let color_spaces = chunk.indirect(color_spaces_ref).dict();
    ctx.colors.write_color_spaces(color_spaces);

    let mut resources = chunk.indirect(ctx.global_resources_ref).start::<Resources>();
    resources.pair(Name(b"XObject"), images_ref);
    resources.pair(Name(b"Pattern"), patterns_ref);
    resources.pair(Name(b"ExtGState"), ext_gs_states_ref);
    resources.pair(Name(b"ColorSpace"), color_spaces_ref);

    let mut fonts = resources.fonts();
    for (font_ref, f) in ctx.font_map.pdf_indices(&ctx.font_refs) {
        let name = eco_format!("F{}", f);
        fonts.pair(Name(name.as_bytes()), font_ref);
    }

    for font in &ctx.color_font_map.all_refs {
        let name = eco_format!("Cf{}", font.get());
        fonts.pair(Name(name.as_bytes()), font);
    }
    fonts.finish();

    resources.finish();

    // Also write the resources for Type3 fonts, that only contains images,
    // color spaces and regular fonts (COLR glyphs depend on them).
    if !ctx.color_font_map.all_refs.is_empty() {
        let mut resources =
            chunk.indirect(ctx.type3_font_resources_ref).start::<Resources>();
        resources.pair(Name(b"XObject"), images_ref);
        resources.pair(Name(b"Pattern"), patterns_ref);
        resources.pair(Name(b"ExtGState"), ext_gs_states_ref);
        resources.pair(Name(b"ColorSpace"), color_spaces_ref);

        let mut fonts = resources.fonts();
        for (font_ref, f) in ctx.font_map.pdf_indices(&ctx.font_refs) {
            let name = eco_format!("F{}", f);
            fonts.pair(Name(name.as_bytes()), font_ref);
        }
        fonts.finish();

        resources.finish();
    }

    // Write all of the functions used by the document.
    ctx.colors.write_functions(&mut chunk);

    chunk
}

/// Write a page tree node.
#[must_use]
fn write_page(ctx: &PdfContext<Ref>, i: usize) -> Chunk {
    let mut chunk = Chunk::new();
    let mut alloc = Ref::new(1);
    let page = &ctx.pages[i];
    let content_id = alloc.bump();

    let mut page_writer = chunk.page(page.id);
    page_writer.parent(ctx.page_tree_ref);

    let w = page.content.size.x.to_f32();
    let h = page.content.size.y.to_f32();
    page_writer.media_box(Rect::new(0.0, 0.0, w, h));
    page_writer.contents(content_id);
    page_writer.pair(Name(b"Resources"), ctx.global_resources_ref);

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
                if let Some(key) = ctx.loc_to_dest.get(loc) {
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
                .page(page.id)
                .xyz(pos.point.x.to_f32(), (page.content.size.y - y).to_f32(), None);
        }
    }

    annotations.finish();
    page_writer.finish();

    chunk
        .stream(content_id, page.content.content.wait())
        .filter(Filter::FlateDecode);

    chunk
}

/// Write the page labels.
pub(crate) fn write_page_labels(
    ctx: &mut PdfContext<Ref>,
) -> (Chunk, Vec<(NonZeroUsize, Ref)>) {
    let mut chunk = Chunk::new();
    let mut alloc = Ref::new(1);

    // If there is no page labeled, we skip the writing
    if !ctx.pages.iter().any(|p| {
        p.label
            .as_ref()
            .is_some_and(|l| l.prefix.is_some() || l.style.is_some())
    }) {
        return (chunk, Vec::new());
    }

    let mut result = vec![];
    let empty_label = PdfPageLabel::default();
    let mut prev: Option<&PdfPageLabel> = None;

    for (i, page) in ctx.pages.iter().enumerate() {
        let nr = NonZeroUsize::new(1 + i).unwrap();
        // If there are pages with empty labels between labeled pages, we must
        // write empty PageLabel entries.
        let label = page.label.as_ref().unwrap_or(&empty_label);

        if let Some(pre) = prev {
            if label.prefix == pre.prefix
                && label.style == pre.style
                && label.offset == pre.offset.map(|n| n.saturating_add(1))
            {
                prev = Some(label);
                continue;
            }
        }

        let id = alloc.bump();
        let mut entry = chunk.indirect(id).start::<PageLabel>();

        // Only add what is actually provided. Don't add empty prefix string if
        // it wasn't given for example.
        if let Some(prefix) = &label.prefix {
            entry.prefix(TextStr(prefix));
        }

        if let Some(style) = label.style {
            entry.style(to_pdf_numbering_style(style));
        }

        if let Some(offset) = label.offset {
            entry.offset(offset.get() as i32);
        }

        result.push((nr, id));
        prev = Some(label);
    }

    (chunk, result)
}

/// Specification for a PDF page label.
#[derive(Debug, Clone, PartialEq, Hash, Default)]
struct PdfPageLabel {
    /// Can be any string or none. Will always be prepended to the numbering style.
    prefix: Option<EcoString>,
    /// Based on the numbering pattern.
    ///
    /// If `None` or numbering is a function, the field will be empty.
    style: Option<PdfPageLabelStyle>,
    /// Offset for the page label start.
    ///
    /// Describes where to start counting from when setting a style.
    /// (Has to be greater or equal than 1)
    offset: Option<NonZeroUsize>,
}

/// A PDF page label number style.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
enum PdfPageLabelStyle {
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

/// Data for an exported page.
pub struct EncodedPage {
    /// The indirect object id of the page.
    pub id: Ref,
    pub content: content::Encoded,
    label: Option<PdfPageLabel>,
}

fn to_pdf_numbering_style(style: PdfPageLabelStyle) -> NumberingStyle {
    match style {
        PdfPageLabelStyle::Arabic => NumberingStyle::Arabic,
        PdfPageLabelStyle::LowerRoman => NumberingStyle::LowerRoman,
        PdfPageLabelStyle::UpperRoman => NumberingStyle::UpperRoman,
        PdfPageLabelStyle::LowerAlpha => NumberingStyle::LowerAlpha,
        PdfPageLabelStyle::UpperAlpha => NumberingStyle::UpperAlpha,
    }
}
