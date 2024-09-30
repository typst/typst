use std::collections::HashMap;
use std::num::NonZeroUsize;

use ecow::EcoString;
use pdf_writer::types::{ActionType, AnnotationFlags, AnnotationType, NumberingStyle};
use pdf_writer::{Filter, Finish, Name, Rect, Ref, Str};
use typst::diag::SourceResult;
use typst::foundations::Label;
use typst::introspection::Location;
use typst::layout::{Abs, Page};
use typst::model::{Destination, Numbering};

use crate::content;
use crate::{
    AbsExt, PdfChunk, PdfOptions, Resources, WithDocument, WithRefs, WithResources,
};

/// Construct page objects.
#[typst_macros::time(name = "construct pages")]
#[allow(clippy::type_complexity)]
pub fn traverse_pages(
    state: &WithDocument,
) -> SourceResult<(PdfChunk, (Vec<Option<EncodedPage>>, Resources<()>))> {
    let mut resources = Resources::default();
    let mut pages = Vec::with_capacity(state.document.pages.len());
    let mut skipped_pages = 0;
    for (i, page) in state.document.pages.iter().enumerate() {
        if state
            .options
            .page_ranges
            .as_ref()
            .is_some_and(|ranges| !ranges.includes_page_index(i))
        {
            // Don't export this page.
            pages.push(None);
            skipped_pages += 1;
        } else {
            let mut encoded = construct_page(state.options, &mut resources, page)?;
            encoded.label = page
                .numbering
                .as_ref()
                .and_then(|num| PdfPageLabel::generate(num, page.number))
                .or_else(|| {
                    // When some pages were ignored from export, we show a page label with
                    // the correct real (not logical) page number.
                    // This is for consistency with normal output when pages have no numbering
                    // and all are exported: the final PDF page numbers always correspond to
                    // the real (not logical) page numbers. Here, the final PDF page number
                    // will differ, but we can at least use labels to indicate what was
                    // the corresponding real page number in the Typst document.
                    (skipped_pages > 0).then(|| PdfPageLabel::arabic(i + 1))
                });
            pages.push(Some(encoded));
        }
    }

    Ok((PdfChunk::new(), (pages, resources)))
}

/// Construct a page object.
#[typst_macros::time(name = "construct page")]
fn construct_page(
    options: &PdfOptions,
    out: &mut Resources<()>,
    page: &Page,
) -> SourceResult<EncodedPage> {
    Ok(EncodedPage {
        content: content::build(
            options,
            out,
            &page.frame,
            page.fill_or_transparent(),
            None,
        )?,
        label: None,
    })
}

/// Allocate a reference for each exported page.
pub fn alloc_page_refs(
    context: &WithResources,
) -> SourceResult<(PdfChunk, Vec<Option<Ref>>)> {
    let mut chunk = PdfChunk::new();
    let page_refs = context
        .pages
        .iter()
        .map(|p| p.as_ref().map(|_| chunk.alloc()))
        .collect();
    Ok((chunk, page_refs))
}

/// Write the page tree.
pub fn write_page_tree(ctx: &WithRefs) -> SourceResult<(PdfChunk, Ref)> {
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
        .kids(ctx.globals.pages.iter().filter_map(Option::as_ref).copied());

    Ok((chunk, page_tree_ref))
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
    let Some((page, page_ref)) = ctx.pages[i].as_ref().zip(ctx.globals.pages[i]) else {
        // Page excluded from export.
        return;
    };

    let mut annotations = Vec::with_capacity(page.content.links.len());
    for (dest, rect) in &page.content.links {
        let id = chunk.alloc();
        annotations.push(id);

        let mut annotation = chunk.annotation(id);
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

        // Don't add links to non-exported pages.
        if let Some((Some(page), Some(page_ref))) =
            ctx.pages.get(index).zip(ctx.globals.pages.get(index))
        {
            annotation
                .action()
                .action_type(ActionType::GoTo)
                .destination()
                .page(*page_ref)
                .xyz(pos.point.x.to_f32(), (page.content.size.y - y).to_f32(), None);
        }
    }

    let mut page_writer = chunk.page(page_ref);
    page_writer.parent(page_tree_ref);

    let w = page.content.size.x.to_f32();
    let h = page.content.size.y.to_f32();
    page_writer.media_box(Rect::new(0.0, 0.0, w, h));
    page_writer.contents(content_id);
    page_writer.pair(Name(b"Resources"), ctx.resources.reference);

    if page.content.uses_opacities {
        page_writer
            .group()
            .transparency()
            .isolated(false)
            .knockout(false)
            .color_space()
            .srgb();
    }

    page_writer.annotations(annotations);

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

        let (prefix, kind) = pat.pieces.first()?;

        // If there is a suffix, we cannot use the common style optimisation,
        // since PDF does not provide a suffix field.
        let style = if pat.suffix.is_empty() {
            use {typst::model::NumberingKind as Kind, PdfPageLabelStyle as Style};
            match kind {
                Kind::Arabic => Some(Style::Arabic),
                Kind::LowerRoman => Some(Style::LowerRoman),
                Kind::UpperRoman => Some(Style::UpperRoman),
                Kind::LowerLatin if number <= 26 => Some(Style::LowerAlpha),
                Kind::LowerLatin if number <= 26 => Some(Style::UpperAlpha),
                _ => None,
            }
        } else {
            None
        };

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

    /// Creates an arabic page label with the specified page number.
    /// For example, this will display page label `11` when given the page
    /// number 11.
    fn arabic(number: usize) -> PdfPageLabel {
        PdfPageLabel {
            prefix: None,
            style: Some(PdfPageLabelStyle::Arabic),
            offset: NonZeroUsize::new(number),
        }
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
