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
use typst_library::layout::{
    GridCell, GridElem, HideElem, Point, Rect, RepeatElem, Size,
};
use typst_library::math::EquationElem;
use typst_library::model::{
    EmphElem, EnumElem, FigureCaption, FigureElem, FootnoteEntry, HeadingElem,
    LinkMarker, ListElem, Outlinable, OutlineEntry, ParElem, QuoteElem, StrongElem,
    TableCell, TableElem, TermsElem,
};
use typst_library::pdf::{ArtifactElem, ArtifactKind, PdfMarkerTag, PdfMarkerTagKind};
use typst_library::text::{
    HighlightElem, Lang, OverlineElem, RawElem, RawLine, ScriptKind, StrikeElem, SubElem,
    SuperElem, TextItem, UnderlineElem,
};
use typst_library::visualize::{Image, ImageElem, Shape};
use typst_syntax::Span;

use crate::convert::{FrameContext, GlobalContext};
use crate::link::LinkAnnotation;
use crate::tags::convert::ArtifactKindExt;
use crate::tags::grid::{GridCtx, TableCtx};
use crate::tags::list::ListCtx;
use crate::tags::outline::OutlineCtx;
use crate::tags::text::{ResolvedTextAttrs, TextAttr, TextDecoKind};
use crate::tags::util::{PropertyOptRef, PropertyValCloned, PropertyValCopied};

pub use context::*;

mod context;
mod convert;
mod grid;
mod list;
mod outline;
mod text;
mod util;

#[derive(Debug, Clone, PartialEq)]
pub enum TagNode {
    Group(TagGroup),
    Leaf(Identifier),
    /// Allows inserting a placeholder into the tag tree.
    /// Currently used for [`krilla::page::Page::add_tagged_annotation`].
    Placeholder(Placeholder),
    FootnoteEntry(Location),
    /// If the attributes are non-empty this will resolve to a [`Tag::Span`],
    /// otherwise the items are inserted directly.
    Text(ResolvedTextAttrs, Vec<Identifier>),
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

pub fn handle_start(
    gc: &mut GlobalContext,
    surface: &mut Surface,
    elem: &Content,
) -> SourceResult<()> {
    if gc.options.disable_tags {
        return Ok(());
    }

    if gc.tags.disable.is_some() {
        // Don't nest artifacts
        return Ok(());
    }

    #[allow(clippy::redundant_pattern_matching)]
    if let Some(_) = elem.to_packed::<HideElem>() {
        push_disable(gc, surface, elem, ArtifactKind::Other);
        return Ok(());
    } else if let Some(artifact) = elem.to_packed::<ArtifactElem>() {
        let kind = artifact.kind.val();
        push_disable(gc, surface, elem, kind);
        return Ok(());
    } else if let Some(_) = elem.to_packed::<RepeatElem>() {
        push_disable(gc, surface, elem, ArtifactKind::Other);
        return Ok(());
    }

    #[allow(clippy::redundant_pattern_matching)]
    let tag: TagKind = if let Some(tag) = elem.to_packed::<PdfMarkerTag>() {
        match &tag.kind {
            PdfMarkerTagKind::OutlineBody => {
                push_stack(gc, elem, StackEntryKind::Outline(OutlineCtx::new()));
                return Ok(());
            }
            PdfMarkerTagKind::FigureBody(alt) => {
                let alt = alt.as_ref().map(|s| s.to_string());
                push_stack(gc, elem, StackEntryKind::Figure(FigureCtx::new(alt)));
                return Ok(());
            }
            PdfMarkerTagKind::FootnoteRef(decl_loc) => {
                push_stack(gc, elem, StackEntryKind::FootnoteRef(*decl_loc));
                return Ok(());
            }
            PdfMarkerTagKind::Bibliography(numbered) => {
                let numbering =
                    if *numbered { ListNumbering::Decimal } else { ListNumbering::None };
                push_stack(gc, elem, StackEntryKind::List(ListCtx::new(numbering)));
                return Ok(());
            }
            PdfMarkerTagKind::BibEntry => {
                push_stack(gc, elem, StackEntryKind::BibEntry);
                return Ok(());
            }
            PdfMarkerTagKind::ListItemLabel => {
                push_stack(gc, elem, StackEntryKind::ListItemLabel);
                return Ok(());
            }
            PdfMarkerTagKind::ListItemBody => {
                push_stack(gc, elem, StackEntryKind::ListItemBody);
                return Ok(());
            }
            PdfMarkerTagKind::Label => Tag::Lbl.into(),
        }
    } else if let Some(entry) = elem.to_packed::<OutlineEntry>() {
        push_stack(gc, elem, StackEntryKind::OutlineEntry(entry.clone()));
        return Ok(());
    } else if let Some(_list) = elem.to_packed::<ListElem>() {
        let numbering = ListNumbering::Circle; // TODO: infer numbering from `list.marker`
        push_stack(gc, elem, StackEntryKind::List(ListCtx::new(numbering)));
        return Ok(());
    } else if let Some(_enumeration) = elem.to_packed::<EnumElem>() {
        let numbering = ListNumbering::Decimal; // TODO: infer numbering from `enum.numbering`
        push_stack(gc, elem, StackEntryKind::List(ListCtx::new(numbering)));
        return Ok(());
    } else if let Some(_terms) = elem.to_packed::<TermsElem>() {
        let numbering = ListNumbering::None;
        push_stack(gc, elem, StackEntryKind::List(ListCtx::new(numbering)));
        return Ok(());
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
        return Ok(());
    } else if let Some(equation) = elem.to_packed::<EquationElem>() {
        let alt = equation.alt.opt_ref().map(|s| s.to_string());
        if let Some(figure_ctx) = gc.tags.stack.parent_figure() {
            // Set alt text of outer figure tag, if not present.
            if figure_ctx.alt.is_none() {
                figure_ctx.alt = alt.clone();
            }
        }
        push_stack(gc, elem, StackEntryKind::Formula(FigureCtx::new(alt)));
        return Ok(());
    } else if let Some(table) = elem.to_packed::<TableElem>() {
        let table_id = gc.tags.next_table_id();
        let summary = table.summary.opt_ref().map(|s| s.to_string());
        let grid = table.grid.clone().unwrap();
        let ctx = TableCtx::new(grid, table_id, summary);
        push_stack(gc, elem, StackEntryKind::Table(ctx));
        return Ok(());
    } else if let Some(cell) = elem.to_packed::<TableCell>() {
        // Only repeated table headers and footer cells are laid out multiple
        // times. Mark duplicate headers as artifacts, since they have no
        // semantic meaning in the tag tree, which doesn't use page breaks for
        // it's semantic structure.
        if cell.is_repeated.val() {
            push_disable(gc, surface, elem, ArtifactKind::Other);
        } else {
            push_stack(gc, elem, StackEntryKind::TableCell(cell.clone()));
        }
        return Ok(());
    } else if let Some(grid) = elem.to_packed::<GridElem>() {
        let grid = grid.grid.clone().unwrap();
        let ctx = GridCtx::new(grid);
        push_stack(gc, elem, StackEntryKind::Grid(ctx));
        return Ok(());
    } else if let Some(cell) = elem.to_packed::<GridCell>() {
        // If there is no grid parent, this means a grid layouter is used
        // internally. Don't generate a stack entry.
        if gc.tags.stack.parent_grid().is_some() {
            // The grid cells are collected into a grid to ensure proper reading
            // order, even when using rowspans, which may be laid out later than
            // other cells in the same row.

            // Only repeated grid headers and footer cells are laid out multiple
            // times. Mark duplicate headers as artifacts, since they have no
            // semantic meaning in the tag tree, which doesn't use page breaks for
            // it's semantic structure.
            if cell.is_repeated.val() {
                push_disable(gc, surface, elem, ArtifactKind::Other);
            } else {
                push_stack(gc, elem, StackEntryKind::GridCell(cell.clone()));
            }
        }
        return Ok(());
    } else if let Some(heading) = elem.to_packed::<HeadingElem>() {
        let level = heading.level().try_into().unwrap_or(NonZeroU16::MAX);
        let name = heading.body.plain_text().to_string();
        Tag::Hn(level, Some(name)).into()
    } else if let Some(_) = elem.to_packed::<ParElem>() {
        Tag::P.into()
    } else if let Some(link) = elem.to_packed::<LinkMarker>() {
        let link_id = gc.tags.next_link_id();
        push_stack(gc, elem, StackEntryKind::Link(link_id, link.clone()));
        return Ok(());
    } else if let Some(entry) = elem.to_packed::<FootnoteEntry>() {
        let footnote_loc = entry.note.location().unwrap();
        push_stack(gc, elem, StackEntryKind::FootnoteEntry(footnote_loc));
        return Ok(());
    } else if let Some(quote) = elem.to_packed::<QuoteElem>() {
        // TODO: should the attribution be handled somehow?
        if quote.block.val() { Tag::BlockQuote.into() } else { Tag::InlineQuote.into() }
    } else if let Some(raw) = elem.to_packed::<RawElem>() {
        if raw.block.val() {
            push_stack(gc, elem, StackEntryKind::CodeBlock);
            return Ok(());
        } else {
            Tag::Code.into()
        }
    } else if let Some(_) = elem.to_packed::<RawLine>() {
        // If the raw element is inline, the content can be inserted directly.
        if gc.tags.stack.parent().is_some_and(|p| p.is_code_block()) {
            push_stack(gc, elem, StackEntryKind::CodeBlockLine);
        }
        return Ok(());
    } else if let Some(_) = elem.to_packed::<StrongElem>() {
        gc.tags.text_attrs.push(elem, TextAttr::Strong);
        return Ok(());
    } else if let Some(_) = elem.to_packed::<EmphElem>() {
        gc.tags.text_attrs.push(elem, TextAttr::Emph);
        return Ok(());
    } else if let Some(sub) = elem.to_packed::<SubElem>() {
        let baseline_shift = sub.baseline.val();
        let lineheight = sub.size.val();
        let kind = ScriptKind::Sub;
        gc.tags.text_attrs.push_script(elem, kind, baseline_shift, lineheight);
        return Ok(());
    } else if let Some(sup) = elem.to_packed::<SuperElem>() {
        let baseline_shift = sup.baseline.val();
        let lineheight = sup.size.val();
        let kind = ScriptKind::Super;
        gc.tags.text_attrs.push_script(elem, kind, baseline_shift, lineheight);
        return Ok(());
    } else if let Some(highlight) = elem.to_packed::<HighlightElem>() {
        let paint = highlight.fill.opt_ref();
        gc.tags.text_attrs.push_highlight(elem, paint);
        return Ok(());
    } else if let Some(underline) = elem.to_packed::<UnderlineElem>() {
        let kind = TextDecoKind::Underline;
        let stroke = underline.stroke.val_cloned();
        gc.tags.text_attrs.push_deco(gc.options, elem, kind, stroke)?;
        return Ok(());
    } else if let Some(overline) = elem.to_packed::<OverlineElem>() {
        let kind = TextDecoKind::Overline;
        let stroke = overline.stroke.val_cloned();
        gc.tags.text_attrs.push_deco(gc.options, elem, kind, stroke)?;
        return Ok(());
    } else if let Some(strike) = elem.to_packed::<StrikeElem>() {
        let kind = TextDecoKind::Strike;
        let stroke = strike.stroke.val_cloned();
        gc.tags.text_attrs.push_deco(gc.options, elem, kind, stroke)?;
        return Ok(());
    } else {
        return Ok(());
    };

    let tag = tag.with_location(Some(elem.span().into_raw()));
    push_stack(gc, elem, StackEntryKind::Standard(tag));

    Ok(())
}

fn push_stack(gc: &mut GlobalContext, elem: &Content, kind: StackEntryKind) {
    let loc = elem.location().expect("elem to be locatable");
    let span = elem.span();
    gc.tags
        .stack
        .push(StackEntry { loc, span, lang: None, kind, nodes: Vec::new() });
}

fn push_disable(
    gc: &mut GlobalContext,
    surface: &mut Surface,
    elem: &Content,
    kind: ArtifactKind,
) {
    let loc = elem.location().expect("elem to be locatable");
    surface.start_tagged(ContentTag::Artifact(kind.to_krilla()));
    gc.tags.disable = Some(Disable::Elem(loc, kind));
}

pub fn handle_end(
    gc: &mut GlobalContext,
    surface: &mut Surface,
    loc: Location,
) -> SourceResult<()> {
    if gc.options.disable_tags {
        return Ok(());
    }

    if let Some(Disable::Elem(l, _)) = gc.tags.disable
        && l == loc
    {
        surface.end_tagged();
        gc.tags.disable = None;
        return Ok(());
    }

    if let Some(entry) = gc.tags.stack.pop_if(|e| e.loc == loc) {
        // The tag nesting was properly closed.
        pop_stack(gc, entry);
        return Ok(());
    }

    if gc.tags.text_attrs.pop(loc) {
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
    let mut is_breakable = true;
    let mut non_breakable_span = Span::detached();
    for e in gc.tags.stack[idx + 1..].iter() {
        if e.kind.is_breakable(gc.options.is_pdf_ua()) {
            continue;
        }

        is_breakable = false;
        if !e.span.is_detached() {
            non_breakable_span = e.span;
            break;
        }
    }
    if !is_breakable {
        if gc.options.is_pdf_ua() {
            let validator = gc.options.standards.config.validator();
            let validator = validator.as_str();
            bail!(
                non_breakable_span,
                "{validator} error: invalid semantic structure, \
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
        StackEntryKind::Grid(ctx) => ctx.build_grid(contents),
        StackEntryKind::GridCell(cell) => {
            if let Some(grid_ctx) = gc.tags.stack.parent_grid() {
                grid_ctx.insert(&cell, contents);
                return;
            } else {
                // Avoid panicking, the nesting will be validated later.
                TagNode::group(Tag::Div, contents)
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
            let tag = Tag::Figure(ctx.alt).with_bbox(ctx.bbox.to_krilla());
            TagNode::group(tag, contents)
        }
        StackEntryKind::Formula(ctx) => {
            let tag = Tag::Formula(ctx.alt).with_bbox(ctx.bbox.to_krilla());
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

pub fn page_start(gc: &mut GlobalContext, surface: &mut Surface) {
    if gc.options.disable_tags {
        return;
    }

    if let Some(disable) = gc.tags.disable {
        let kind = match disable {
            Disable::Elem(_, kind) => kind,
            Disable::Tiling => ArtifactKind::Other,
        };
        surface.start_tagged(ContentTag::Artifact(kind.to_krilla()));
    }
}

pub fn page_end(gc: &mut GlobalContext, surface: &mut Surface) {
    if gc.options.disable_tags {
        return;
    }

    if gc.tags.disable.is_some() {
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

pub struct DisableHandle<'a, 'b, 'c, 'd> {
    gc: &'b mut GlobalContext<'a>,
    surface: &'d mut Surface<'c>,
    /// Whether this handle started the disabled range.
    started: bool,
}

impl Drop for DisableHandle<'_, '_, '_, '_> {
    fn drop(&mut self) {
        if self.started {
            self.gc.tags.disable = None;
            self.surface.end_tagged();
        }
    }
}

impl<'a, 'c> DisableHandle<'a, '_, 'c, '_> {
    pub fn reborrow<'s>(
        &'s mut self,
    ) -> (&'s mut GlobalContext<'a>, &'s mut Surface<'c>) {
        (self.gc, self.surface)
    }
}

pub fn disable<'a, 'b, 'c, 'd>(
    gc: &'b mut GlobalContext<'a>,
    surface: &'d mut Surface<'c>,
    kind: Disable,
) -> DisableHandle<'a, 'b, 'c, 'd> {
    let started = gc.tags.disable.is_none();
    if started {
        gc.tags.disable = Some(kind);
        surface.start_tagged(ContentTag::Artifact(ArtifactType::Other));
    }
    DisableHandle { gc, surface, started }
}

pub fn text<'a, 'b>(
    gc: &mut GlobalContext,
    fc: &FrameContext,
    surface: &'b mut Surface<'a>,
    text: &TextItem,
) -> TagHandle<'a, 'b> {
    if gc.options.disable_tags {
        return TagHandle { surface, started: false };
    }

    update_bbox(gc, fc, || text.bbox());

    if gc.tags.disable.is_some() {
        return TagHandle { surface, started: false };
    }

    let attrs = gc.tags.text_attrs.resolve(text);

    // Marked content
    let lang = gc.tags.try_set_lang(text.lang);
    let lang = lang.as_ref().map(Lang::as_str);
    let content = ContentTag::Span(SpanTag::empty().with_lang(lang));
    let id = surface.start_tagged(content);

    gc.tags.push_text(attrs, id);

    TagHandle { surface, started: true }
}

pub fn image<'a, 'b>(
    gc: &mut GlobalContext,
    fc: &FrameContext,
    surface: &'b mut Surface<'a>,
    image: &Image,
    size: Size,
) -> TagHandle<'a, 'b> {
    if gc.options.disable_tags {
        return TagHandle { surface, started: false };
    }

    update_bbox(gc, fc, || Rect::from_pos_size(Point::zero(), size));
    let content = ContentTag::Span(SpanTag::empty().with_alt_text(image.alt()));
    start_content(gc, surface, content)
}

pub fn shape<'a, 'b>(
    gc: &mut GlobalContext,
    fc: &FrameContext,
    surface: &'b mut Surface<'a>,
    shape: &Shape,
) -> TagHandle<'a, 'b> {
    if gc.options.disable_tags {
        return TagHandle { surface, started: false };
    }

    update_bbox(gc, fc, || shape.geometry.bbox());
    start_content(gc, surface, ContentTag::Artifact(ArtifactType::Other))
}

fn update_bbox(
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

fn start_content<'a, 'b>(
    gc: &mut GlobalContext,
    surface: &'b mut Surface<'a>,
    content: ContentTag,
) -> TagHandle<'a, 'b> {
    if gc.tags.disable.is_some() {
        return TagHandle { surface, started: false };
    }

    let artifact = matches!(content, ContentTag::Artifact(_));
    let id = surface.start_tagged(content);
    if !artifact {
        gc.tags.push(TagNode::Leaf(id));
    }
    TagHandle { surface, started: true }
}
