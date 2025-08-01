use std::num::NonZeroU16;

use krilla::configure::Validator;
use krilla::page::Page;
use krilla::surface::Surface;
use krilla::tagging as kt;
use krilla::tagging::{
    ArtifactType, ContentTag, Identifier, ListNumbering, Node, SpanTag, Tag, TagKind,
};
use typst_library::diag::{SourceResult, bail};
use typst_library::foundations::Content;
use typst_library::introspection::Location;
use typst_library::layout::{Rect, RepeatElem};
use typst_library::math::EquationElem;
use typst_library::model::{
    EnumElem, FigureCaption, FigureElem, FootnoteEntry, HeadingElem, LinkMarker,
    ListElem, Outlinable, OutlineEntry, ParElem, QuoteElem, TableCell, TableElem,
    TermsElem,
};
use typst_library::pdf::{ArtifactElem, ArtifactKind, PdfMarkerTag, PdfMarkerTagKind};
use typst_library::text::{Lang, RawElem, RawLine};
use typst_library::visualize::ImageElem;
use typst_syntax::Span;

use crate::convert::{FrameContext, GlobalContext};
use crate::link::LinkAnnotation;
use crate::tags::list::ListCtx;
use crate::tags::outline::OutlineCtx;
use crate::tags::table::TableCtx;
use crate::tags::util::{PropertyOptRef, PropertyValCopied};

pub use context::*;

mod context;
mod list;
mod outline;
mod table;
mod util;

#[derive(Debug, Clone, PartialEq)]
pub enum TagNode {
    Group(TagGroup),
    Leaf(Identifier),
    /// Allows inserting a placeholder into the tag tree.
    /// Currently used for [`krilla::page::Page::add_tagged_annotation`].
    Placeholder(Placeholder),
    FootnoteEntry(Location),
}

impl TagNode {
    pub fn group(tag: impl Into<TagKind>, contents: GroupContents) -> Self {
        let lang = contents.lang.map(|l| l.as_str().to_string());
        let tag = tag
            .into()
            .with_lang(lang)
            .with_location(Some(contents.span.into_raw()));
        TagNode::Group(TagGroup { tag, nodes: contents.nodes })
    }

    /// A tag group not directly related to a typst element, generated to
    /// accomodate the tag structure.
    pub fn virtual_group(tag: impl Into<TagKind>, nodes: Vec<TagNode>) -> Self {
        let tag = tag.into();
        TagNode::Group(TagGroup { tag, nodes })
    }

    pub fn empty_group(tag: impl Into<TagKind>) -> Self {
        Self::virtual_group(tag, Vec::new())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TagGroup {
    tag: TagKind,
    nodes: Vec<TagNode>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GroupContents {
    span: Span,
    lang: Option<Lang>,
    nodes: Vec<TagNode>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Placeholder(usize);

pub fn handle_start(gc: &mut GlobalContext, surface: &mut Surface, elem: &Content) {
    if gc.options.disable_tags {
        return;
    }

    if gc.tags.in_artifact.is_some() {
        // Don't nest artifacts
        return;
    }

    if let Some(artifact) = elem.to_packed::<ArtifactElem>() {
        let kind = artifact.kind.val();
        push_artifact(gc, surface, elem, kind);
        return;
    } else if let Some(_) = elem.to_packed::<RepeatElem>() {
        push_artifact(gc, surface, elem, ArtifactKind::Other);
        return;
    }

    let tag: TagKind = if let Some(tag) = elem.to_packed::<PdfMarkerTag>() {
        match &tag.kind {
            PdfMarkerTagKind::OutlineBody => {
                push_stack(gc, elem, StackEntryKind::Outline(OutlineCtx::new()));
                return;
            }
            PdfMarkerTagKind::FigureBody(alt) => {
                let alt = alt.as_ref().map(|s| s.to_string());
                push_stack(gc, elem, StackEntryKind::Figure(FigureCtx::new(alt)));
                return;
            }
            PdfMarkerTagKind::FootnoteRef(decl_loc) => {
                push_stack(gc, elem, StackEntryKind::FootnoteRef(*decl_loc));
                return;
            }
            PdfMarkerTagKind::Bibliography(numbered) => {
                let numbering =
                    if *numbered { ListNumbering::Decimal } else { ListNumbering::None };
                push_stack(gc, elem, StackEntryKind::List(ListCtx::new(numbering)));
                return;
            }
            PdfMarkerTagKind::BibEntry => {
                push_stack(gc, elem, StackEntryKind::BibEntry);
                return;
            }
            PdfMarkerTagKind::ListItemLabel => {
                push_stack(gc, elem, StackEntryKind::ListItemLabel);
                return;
            }
            PdfMarkerTagKind::ListItemBody => {
                push_stack(gc, elem, StackEntryKind::ListItemBody);
                return;
            }
            PdfMarkerTagKind::Label => Tag::Lbl.into(),
        }
    } else if let Some(entry) = elem.to_packed::<OutlineEntry>() {
        push_stack(gc, elem, StackEntryKind::OutlineEntry(entry.clone()));
        return;
    } else if let Some(_list) = elem.to_packed::<ListElem>() {
        let numbering = ListNumbering::Circle; // TODO: infer numbering from `list.marker`
        push_stack(gc, elem, StackEntryKind::List(ListCtx::new(numbering)));
        return;
    } else if let Some(_enumeration) = elem.to_packed::<EnumElem>() {
        let numbering = ListNumbering::Decimal; // TODO: infer numbering from `enum.numbering`
        push_stack(gc, elem, StackEntryKind::List(ListCtx::new(numbering)));
        return;
    } else if let Some(_terms) = elem.to_packed::<TermsElem>() {
        let numbering = ListNumbering::None;
        push_stack(gc, elem, StackEntryKind::List(ListCtx::new(numbering)));
        return;
    } else if let Some(_) = elem.to_packed::<FigureElem>() {
        // Wrap the figure tag and the sibling caption in a container, if the
        // caption is contained within the figure like recommended for tables
        // screen readers might ignore it.
        Tag::NonStruct.into()
    } else if let Some(_) = elem.to_packed::<FigureCaption>() {
        Tag::Caption.into()
    } else if let Some(image) = elem.to_packed::<ImageElem>() {
        let alt = image.alt.opt_ref().map(|s| s.to_string());

        if let Some(figure_ctx) = gc.tags.stack.parent_figure() {
            // Set alt text of outer figure tag, if not present.
            if figure_ctx.alt.is_none() {
                figure_ctx.alt = alt;
            }
        } else {
            push_stack(gc, elem, StackEntryKind::Figure(FigureCtx::new(alt)));
        }
        return;
    } else if let Some(equation) = elem.to_packed::<EquationElem>() {
        let alt = equation.alt.opt_ref().map(|s| s.to_string());
        if let Some(figure_ctx) = gc.tags.stack.parent_figure() {
            // Set alt text of outer figure tag, if not present.
            if figure_ctx.alt.is_none() {
                figure_ctx.alt = alt.clone();
            }
        }
        push_stack(gc, elem, StackEntryKind::Formula(FigureCtx::new(alt)));
        return;
    } else if let Some(table) = elem.to_packed::<TableElem>() {
        let table_id = gc.tags.next_table_id();
        let summary = table.summary.opt_ref().map(|s| s.to_string());
        let ctx = TableCtx::new(table_id, summary);
        push_stack(gc, elem, StackEntryKind::Table(ctx));
        return;
    } else if let Some(cell) = elem.to_packed::<TableCell>() {
        let table_ctx = gc.tags.stack.parent_table();

        // Only repeated table headers and footer cells are laid out multiple
        // times. Mark duplicate headers as artifacts, since they have no
        // semantic meaning in the tag tree, which doesn't use page breaks for
        // it's semantic structure.
        if cell.is_repeated.val() || table_ctx.is_some_and(|ctx| ctx.contains(cell)) {
            push_artifact(gc, surface, elem, ArtifactKind::Other);
        } else {
            push_stack(gc, elem, StackEntryKind::TableCell(cell.clone()));
        }
        return;
    } else if let Some(heading) = elem.to_packed::<HeadingElem>() {
        let level = heading.level().try_into().unwrap_or(NonZeroU16::MAX);
        let name = heading.body.plain_text().to_string();
        Tag::Hn(level, Some(name)).into()
    } else if let Some(_) = elem.to_packed::<ParElem>() {
        Tag::P.into()
    } else if let Some(link) = elem.to_packed::<LinkMarker>() {
        let link_id = gc.tags.next_link_id();
        push_stack(gc, elem, StackEntryKind::Link(link_id, link.clone()));
        return;
    } else if let Some(entry) = elem.to_packed::<FootnoteEntry>() {
        let footnote_loc = entry.note.location().unwrap();
        push_stack(gc, elem, StackEntryKind::FootnoteEntry(footnote_loc));
        return;
    } else if let Some(quote) = elem.to_packed::<QuoteElem>() {
        // TODO: should the attribution be handled somehow?
        if quote.block.val() { Tag::BlockQuote.into() } else { Tag::InlineQuote.into() }
    } else if let Some(raw) = elem.to_packed::<RawElem>() {
        if raw.block.val() {
            push_stack(gc, elem, StackEntryKind::CodeBlock);
            return;
        } else {
            Tag::Code.into()
        }
    } else if let Some(_) = elem.to_packed::<RawLine>() {
        // If the raw element is inline, the content can be inserted directly.
        if gc.tags.stack.parent().is_some_and(|p| p.is_code_block()) {
            push_stack(gc, elem, StackEntryKind::CodeBlockLine);
        }
        return;
    } else {
        return;
    };

    let tag = tag.with_location(Some(elem.span().into_raw()));
    push_stack(gc, elem, StackEntryKind::Standard(tag));
}

fn push_stack(gc: &mut GlobalContext, elem: &Content, kind: StackEntryKind) {
    let loc = elem.location().expect("elem to be locatable");
    let span = elem.span();
    gc.tags
        .stack
        .push(StackEntry { loc, span, lang: None, kind, nodes: Vec::new() });
}

fn push_artifact(
    gc: &mut GlobalContext,
    surface: &mut Surface,
    elem: &Content,
    kind: ArtifactKind,
) {
    let loc = elem.location().expect("elem to be locatable");
    let ty = artifact_type(kind);
    let id = surface.start_tagged(ContentTag::Artifact(ty));
    gc.tags.push(TagNode::Leaf(id));
    gc.tags.in_artifact = Some((loc, kind));
}

pub fn handle_end(
    gc: &mut GlobalContext,
    surface: &mut Surface,
    loc: Location,
) -> SourceResult<()> {
    if gc.options.disable_tags {
        return Ok(());
    }

    if let Some((l, _)) = gc.tags.in_artifact
        && l == loc
    {
        pop_artifact(gc, surface);
        return Ok(());
    }

    if let Some(entry) = gc.tags.stack.pop_if(|e| e.loc == loc) {
        // The tag nesting was properly closed.
        pop_stack(gc, entry);
        return Ok(());
    }

    // Search for an improperly nested starting tag, that is being closed.
    let Some(idx) = (gc.tags.stack.iter().enumerate())
        .rev()
        .find_map(|(i, e)| (e.loc == loc).then_some(i))
    else {
        // The start tag isn't in the tag stack, just ignore the end tag.
        return Ok(());
    };

    // There are overlapping tags in the tag tree. Figure whether breaking
    // up the current tag stack is semantically ok.
    let is_pdf_ua = gc.options.standards.config.validator() == Validator::UA1;
    let mut is_breakable = true;
    let mut non_breakable_span = Span::detached();
    for e in gc.tags.stack[idx + 1..].iter() {
        if e.kind.is_breakable(is_pdf_ua) {
            continue;
        }

        is_breakable = false;
        if !e.span.is_detached() {
            non_breakable_span = e.span;
            break;
        }
    }
    if !is_breakable {
        let validator = gc.options.standards.config.validator();
        if is_pdf_ua {
            let ua1 = validator.as_str();
            bail!(
                non_breakable_span,
                "{ua1} error: invalid semantic structure, \
                    this element's tag would be split up";
                hint: "maybe this is caused by a `parbreak`, `colbreak`, or `pagebreak`"
            );
        } else {
            bail!(
                non_breakable_span,
                "invalid semantic structure, \
                this element's tag would be split up";
                hint: "maybe this is caused by a `parbreak`, `colbreak`, or `pagebreak`";
                hint: "disable tagged pdf by passing `--no-pdf-tags`"
            );
        }
    }

    // Close all children tags and reopen them after the currently enclosing element.
    let mut broken_entries = Vec::new();
    for _ in idx + 1..gc.tags.stack.len() {
        let entry = gc.tags.stack.pop().unwrap();

        let mut kind = entry.kind.clone();
        if let StackEntryKind::Link(id, _) = &mut kind {
            // Assign a new link id, so a new link annotation will be created.
            *id = gc.tags.next_link_id();
        }
        if let Some(bbox) = kind.bbox_mut() {
            bbox.reset();
        }

        broken_entries.push(StackEntry {
            loc: entry.loc,
            span: entry.span,
            lang: None,
            kind,
            nodes: Vec::new(),
        });
        pop_stack(gc, entry);
    }

    // Pop the closed entry off the stack.
    let closed = gc.tags.stack.pop().unwrap();
    pop_stack(gc, closed);

    // Push all broken and afterwards duplicated entries back on.
    gc.tags.stack.extend(broken_entries);

    Ok(())
}

fn pop_stack(gc: &mut GlobalContext, entry: StackEntry) {
    // Try to propagate the tag language to the parent tag, or the document.
    // If successfull omit the language attribute on this tag.
    let lang = entry.lang.and_then(|lang| {
        let parent_lang = (gc.tags.stack.last_mut())
            .map(|e| &mut e.lang)
            .unwrap_or(&mut gc.tags.doc_lang);
        if parent_lang.is_none_or(|l| l == lang) {
            *parent_lang = Some(lang);
            return None;
        }
        Some(lang)
    });

    let contents = GroupContents { span: entry.span, lang, nodes: entry.nodes };
    let node = match entry.kind {
        StackEntryKind::Standard(tag) => TagNode::group(tag, contents),
        StackEntryKind::Outline(ctx) => ctx.build_outline(contents),
        StackEntryKind::OutlineEntry(outline_entry) => {
            if let Some((outline_ctx, outline_nodes)) = gc.tags.stack.parent_outline() {
                outline_ctx.insert(outline_nodes, outline_entry, contents);
                return;
            } else {
                // Avoid panicking, the nesting will be validated later.
                TagNode::group(Tag::TOCI, contents)
            }
        }
        StackEntryKind::Table(ctx) => ctx.build_table(contents),
        StackEntryKind::TableCell(cell) => {
            if let Some(table_ctx) = gc.tags.stack.parent_table() {
                table_ctx.insert(&cell, contents);
                return;
            } else {
                // Avoid panicking, the nesting will be validated later.
                TagNode::group(Tag::TD, contents)
            }
        }
        StackEntryKind::List(list) => list.build_list(contents),
        StackEntryKind::ListItemLabel => {
            let list_ctx = gc.tags.stack.parent_list().expect("parent list");
            list_ctx.push_label(contents);
            return;
        }
        StackEntryKind::ListItemBody => {
            let list_ctx = gc.tags.stack.parent_list().expect("parent list");
            list_ctx.push_body(contents);
            return;
        }
        StackEntryKind::BibEntry => {
            let list_ctx = gc.tags.stack.parent_list().expect("parent list");
            list_ctx.push_bib_entry(contents);
            return;
        }
        StackEntryKind::Figure(ctx) => {
            let tag = Tag::Figure(ctx.alt).with_bbox(ctx.bbox.get());
            TagNode::group(tag, contents)
        }
        StackEntryKind::Formula(ctx) => {
            let tag = Tag::Formula(ctx.alt).with_bbox(ctx.bbox.get());
            TagNode::group(tag, contents)
        }
        StackEntryKind::Link(_, _) => {
            let mut node = TagNode::group(Tag::Link, contents);
            // Wrap link in reference tag if inside an outline entry.
            if gc.tags.stack.parent_outline_entry().is_some() {
                node = TagNode::virtual_group(Tag::Reference, vec![node]);
            }
            node
        }
        StackEntryKind::FootnoteRef(decl_loc) => {
            // transparently insert all children.
            gc.tags.extend(contents.nodes);

            let ctx = gc.tags.footnotes.entry(decl_loc).or_insert(FootnoteCtx::new());

            // Only insert the footnote entry once after the first reference.
            if !ctx.is_referenced {
                ctx.is_referenced = true;
                gc.tags.push(TagNode::FootnoteEntry(decl_loc));
            }
            return;
        }
        StackEntryKind::FootnoteEntry(footnote_loc) => {
            // Store footnotes separately so they can be inserted directly after
            // the footnote reference in the reading order.
            let tag = TagNode::group(Tag::Note, contents);
            let ctx = gc.tags.footnotes.entry(footnote_loc).or_insert(FootnoteCtx::new());
            ctx.entry = Some(tag);
            return;
        }
        StackEntryKind::CodeBlock => {
            TagNode::group(Tag::Code.with_placement(Some(kt::Placement::Block)), contents)
        }
        StackEntryKind::CodeBlockLine => {
            // If the raw element is a block, wrap each line in a BLSE, so the
            // individual lines are properly wrapped and indented when reflowed.
            TagNode::group(Tag::P, contents)
        }
    };

    gc.tags.push(node);
}

fn pop_artifact(gc: &mut GlobalContext, surface: &mut Surface) {
    surface.end_tagged();
    gc.tags.in_artifact = None;
}

pub fn page_start(gc: &mut GlobalContext, surface: &mut Surface) {
    if gc.options.disable_tags {
        return;
    }

    if let Some((_, kind)) = gc.tags.in_artifact {
        let ty = artifact_type(kind);
        let id = surface.start_tagged(ContentTag::Artifact(ty));
        gc.tags.push(TagNode::Leaf(id));
    }
}

pub fn page_end(gc: &mut GlobalContext, surface: &mut Surface) {
    if gc.options.disable_tags {
        return;
    }

    if gc.tags.in_artifact.is_some() {
        surface.end_tagged();
    }
}

/// Add all annotations that were found in the page frame.
pub fn add_link_annotations(
    gc: &mut GlobalContext,
    page: &mut Page,
    annotations: Vec<LinkAnnotation>,
) {
    for a in annotations.into_iter() {
        let annotation = krilla::annotation::Annotation::new_link(
            krilla::annotation::LinkAnnotation::new_with_quad_points(
                a.quad_points,
                a.target,
            ),
            a.alt,
        )
        .with_location(Some(a.span.into_raw()));

        if gc.options.disable_tags {
            page.add_annotation(annotation);
        } else {
            let annot_id = page.add_tagged_annotation(annotation);
            gc.tags.placeholders.init(a.placeholder, Node::Leaf(annot_id));
        }
    }
}

pub fn update_bbox(
    gc: &mut GlobalContext,
    fc: &FrameContext,
    compute_bbox: impl FnOnce() -> Rect,
) {
    if let Some(bbox) = gc.tags.stack.find_parent_bbox()
        && gc.options.standards.config.validator() == Validator::UA1
    {
        bbox.expand_frame(fc, compute_bbox());
    }
}

/// Automatically calls [`Surface::end_tagged`] when dropped.
pub struct TagHandle<'a, 'b> {
    surface: &'b mut Surface<'a>,
    /// Whether this tag handle started the marked content sequence, and should
    /// thus end it when it is dropped.
    started: bool,
}

impl Drop for TagHandle<'_, '_> {
    fn drop(&mut self) {
        if self.started {
            self.surface.end_tagged();
        }
    }
}

impl<'a> TagHandle<'a, '_> {
    pub fn surface<'c>(&'c mut self) -> &'c mut Surface<'a> {
        self.surface
    }
}

/// Returns a [`TagHandle`] that automatically calls [`Surface::end_tagged`]
/// when dropped.
pub fn start_span<'a, 'b>(
    gc: &mut GlobalContext,
    surface: &'b mut Surface<'a>,
    span: SpanTag,
) -> TagHandle<'a, 'b> {
    start_content(gc, surface, ContentTag::Span(span))
}

/// Returns a [`TagHandle`] that automatically calls [`Surface::end_tagged`]
/// when dropped.
pub fn start_artifact<'a, 'b>(
    gc: &mut GlobalContext,
    surface: &'b mut Surface<'a>,
    kind: ArtifactKind,
) -> TagHandle<'a, 'b> {
    let ty = artifact_type(kind);
    start_content(gc, surface, ContentTag::Artifact(ty))
}

fn start_content<'a, 'b>(
    gc: &mut GlobalContext,
    surface: &'b mut Surface<'a>,
    content: ContentTag,
) -> TagHandle<'a, 'b> {
    if gc.options.disable_tags {
        return TagHandle { surface, started: false };
    }

    let content = if gc.tags.in_artifact.is_some() {
        return TagHandle { surface, started: false };
    } else {
        content
    };
    let id = surface.start_tagged(content);
    gc.tags.push(TagNode::Leaf(id));
    TagHandle { surface, started: true }
}

fn artifact_type(kind: ArtifactKind) -> ArtifactType {
    match kind {
        ArtifactKind::Header => ArtifactType::Header,
        ArtifactKind::Footer => ArtifactType::Footer,
        ArtifactKind::Page => ArtifactType::Page,
        ArtifactKind::Other => ArtifactType::Other,
    }
}
