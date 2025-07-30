use std::cell::OnceCell;
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::slice::SliceIndex;

use ecow::EcoString;
use krilla::configure::Validator;
use krilla::geom as kg;
use krilla::page::Page;
use krilla::surface::Surface;
use krilla::tagging::{
    ArtifactType, BBox, ContentTag, Identifier, ListNumbering, Node, SpanTag, Tag,
    TagGroup, TagKind, TagTree,
};
use typst_library::diag::{SourceResult, bail};
use typst_library::foundations::{
    Content, LinkMarker, NativeElement, Packed, RefableProperty, Settable,
    SettableProperty, StyleChain,
};
use typst_library::introspection::Location;
use typst_library::layout::{Abs, Point, Rect, RepeatElem};
use typst_library::math::EquationElem;
use typst_library::model::{
    Destination, EnumElem, FigureCaption, FigureElem, FootnoteEntry, HeadingElem,
    ListElem, Outlinable, OutlineEntry, ParElem, QuoteElem, TableCell, TableElem,
    TermsElem,
};
use typst_library::pdf::{ArtifactElem, ArtifactKind, PdfMarkerTag, PdfMarkerTagKind};
use typst_library::visualize::ImageElem;
use typst_syntax::Span;

use crate::convert::{FrameContext, GlobalContext};
use crate::link::LinkAnnotation;
use crate::tags::list::ListCtx;
use crate::tags::outline::OutlineCtx;
use crate::tags::table::TableCtx;
use crate::util::AbsExt;

mod list;
mod outline;
mod table;

pub(crate) fn handle_start(
    gc: &mut GlobalContext,
    surface: &mut Surface,
    elem: &Content,
) -> SourceResult<()> {
    if gc.options.disable_tags {
        return Ok(());
    }

    if gc.tags.in_artifact.is_some() {
        // Don't nest artifacts
        return Ok(());
    }

    if let Some(artifact) = elem.to_packed::<ArtifactElem>() {
        let kind = artifact.kind.get(StyleChain::default());
        push_artifact(gc, surface, elem, kind);
        return Ok(());
    } else if let Some(_) = elem.to_packed::<RepeatElem>() {
        push_artifact(gc, surface, elem, ArtifactKind::Other);
        return Ok(());
    }

    let mut tag: TagKind = if let Some(tag) = elem.to_packed::<PdfMarkerTag>() {
        match &tag.kind {
            PdfMarkerTagKind::OutlineBody => {
                push_stack(gc, elem, StackEntryKind::Outline(OutlineCtx::new()))?;
                return Ok(());
            }
            PdfMarkerTagKind::FigureBody(alt) => {
                let alt = alt.as_ref().map(|s| s.to_string());
                push_stack(gc, elem, StackEntryKind::Figure(FigureCtx::new(alt)))?;
                return Ok(());
            }
            PdfMarkerTagKind::FootnoteRef(decl_loc) => {
                push_stack(gc, elem, StackEntryKind::FootnoteRef(*decl_loc))?;
                return Ok(());
            }
            PdfMarkerTagKind::Bibliography(numbered) => {
                let numbering =
                    if *numbered { ListNumbering::Decimal } else { ListNumbering::None };
                push_stack(gc, elem, StackEntryKind::List(ListCtx::new(numbering)))?;
                return Ok(());
            }
            PdfMarkerTagKind::BibEntry => {
                push_stack(gc, elem, StackEntryKind::BibEntry)?;
                return Ok(());
            }
            PdfMarkerTagKind::ListItemLabel => {
                push_stack(gc, elem, StackEntryKind::ListItemLabel)?;
                return Ok(());
            }
            PdfMarkerTagKind::ListItemBody => {
                push_stack(gc, elem, StackEntryKind::ListItemBody)?;
                return Ok(());
            }
            PdfMarkerTagKind::Label => Tag::Lbl.into(),
        }
    } else if let Some(entry) = elem.to_packed::<OutlineEntry>() {
        push_stack(gc, elem, StackEntryKind::OutlineEntry(entry.clone()))?;
        return Ok(());
    } else if let Some(_list) = elem.to_packed::<ListElem>() {
        let numbering = ListNumbering::Circle; // TODO: infer numbering from `list.marker`
        push_stack(gc, elem, StackEntryKind::List(ListCtx::new(numbering)))?;
        return Ok(());
    } else if let Some(_enumeration) = elem.to_packed::<EnumElem>() {
        let numbering = ListNumbering::Decimal; // TODO: infer numbering from `enum.numbering`
        push_stack(gc, elem, StackEntryKind::List(ListCtx::new(numbering)))?;
        return Ok(());
    } else if let Some(_enumeration) = elem.to_packed::<TermsElem>() {
        let numbering = ListNumbering::None;
        push_stack(gc, elem, StackEntryKind::List(ListCtx::new(numbering)))?;
        return Ok(());
    } else if let Some(_) = elem.to_packed::<FigureElem>() {
        // Wrap the figure tag and the sibling caption in a container, if the
        // caption is contained within the figure like recommended for tables
        // screen readers might ignore it.
        Tag::NonStruct.into()
    } else if let Some(_) = elem.to_packed::<FigureCaption>() {
        Tag::Caption.into()
    } else if let Some(image) = elem.to_packed::<ImageElem>() {
        let alt = image.alt.get_as_ref().map(|s| s.to_string());

        if let Some(figure_ctx) = gc.tags.stack.parent_figure() {
            // Set alt text of outer figure tag, if not present.
            if figure_ctx.alt.is_none() {
                figure_ctx.alt = alt;
            }
            return Ok(());
        } else {
            push_stack(gc, elem, StackEntryKind::Figure(FigureCtx::new(alt)))?;
            return Ok(());
        }
    } else if let Some(equation) = elem.to_packed::<EquationElem>() {
        let alt = equation.alt.get_as_ref().map(|s| s.to_string());
        push_stack(gc, elem, StackEntryKind::Formula(FigureCtx::new(alt)))?;
        return Ok(());
    } else if let Some(table) = elem.to_packed::<TableElem>() {
        let table_id = gc.tags.next_table_id();
        let summary = table.summary.get_as_ref().map(|s| s.to_string());
        let ctx = TableCtx::new(table_id, summary);
        push_stack(gc, elem, StackEntryKind::Table(ctx))?;
        return Ok(());
    } else if let Some(cell) = elem.to_packed::<TableCell>() {
        let table_ctx = gc.tags.stack.parent_table();

        // Only repeated table headers and footer cells are layed out multiple
        // times. Mark duplicate headers as artifacts, since they have no
        // semantic meaning in the tag tree, which doesn't use page breaks for
        // it's semantic structure.
        if table_ctx.is_some_and(|ctx| ctx.contains(cell)) {
            // TODO: currently the first layouted cell is picked to be part of
            // the tag tree, for repeating footers this will be the cell on the
            // first page. Maybe it should be the cell on the last page, but that
            // would require more changes in the layouting code, or a pre-pass
            // on the frames to figure out if there are other footers following.
            push_artifact(gc, surface, elem, ArtifactKind::Other);
        } else {
            push_stack(gc, elem, StackEntryKind::TableCell(cell.clone()))?;
        }
        return Ok(());
    } else if let Some(heading) = elem.to_packed::<HeadingElem>() {
        let level = heading.level().try_into().unwrap_or(NonZeroU32::MAX);
        let name = heading.body.plain_text().to_string();
        Tag::Hn(level, Some(name)).into()
    } else if let Some(_) = elem.to_packed::<ParElem>() {
        Tag::P.into()
    } else if let Some(link) = elem.to_packed::<LinkMarker>() {
        let link_id = gc.tags.next_link_id();
        push_stack(gc, elem, StackEntryKind::Link(link_id, link.clone()))?;
        return Ok(());
    } else if let Some(entry) = elem.to_packed::<FootnoteEntry>() {
        let footnote_loc = entry.note.location().unwrap();
        push_stack(gc, elem, StackEntryKind::FootnoteEntry(footnote_loc))?;
        return Ok(());
    } else if let Some(quote) = elem.to_packed::<QuoteElem>() {
        // TODO: should the attribution be handled somehow?
        if quote.block.get(StyleChain::default()) {
            Tag::BlockQuote.into()
        } else {
            Tag::InlineQuote.into()
        }
    } else {
        return Ok(());
    };

    tag.set_location(Some(elem.span().into_raw()));
    push_stack(gc, elem, StackEntryKind::Standard(tag))?;

    Ok(())
}

fn push_stack(
    gc: &mut GlobalContext,
    elem: &Content,
    kind: StackEntryKind,
) -> SourceResult<()> {
    let loc = elem.location().expect("elem to be locatable");
    let span = elem.span();

    if !gc.tags.context_supports(&kind) {
        if gc.options.standards.config.validator() == Validator::UA1 {
            // TODO: error
        } else {
            // TODO: warning
        }
    }

    gc.tags.stack.push(StackEntry { loc, span, kind, nodes: Vec::new() });

    Ok(())
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

pub(crate) fn handle_end(
    gc: &mut GlobalContext,
    surface: &mut Surface,
    loc: Location,
) -> SourceResult<()> {
    if gc.options.disable_tags {
        return Ok(());
    }

    // FIXME: Overlapping tags in artifacts will break the close mechanism.
    if let Some((l, _)) = gc.tags.in_artifact {
        if l == loc {
            pop_artifact(gc, surface);
        }
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
    let non_breakable = gc.tags.stack[idx + 1..]
        .iter()
        .find(|e| !e.kind.is_breakable(is_pdf_ua));
    if let Some(entry) = non_breakable {
        let validator = gc.options.standards.config.validator();
        if is_pdf_ua {
            let ua1 = validator.as_str();
            bail!(
                entry.span,
                "{ua1} error: invalid semantic structure, \
                    this element's tag would be split up";
                hint: "maybe there is a `parbreak`, `colbreak`, or `pagebreak`"
            );
        } else {
            bail!(
                entry.span,
                "invalid semantic structure, \
                this element's tag would be split up";
                hint: "maybe there is a `parbreak`, `colbreak`, or `pagebreak`";
                hint: "disable tagged pdf by passing `--disable-pdf-tags`"
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
    let node = match entry.kind {
        StackEntryKind::Standard(tag) => TagNode::Group(tag, entry.nodes),
        StackEntryKind::Outline(ctx) => ctx.build_outline(entry.nodes),
        StackEntryKind::OutlineEntry(outline_entry) => {
            let Some((outline_ctx, outline_nodes)) = gc.tags.stack.parent_outline()
            else {
                // PDF/UA compliance of the structure hierarchy is checked
                // elsewhere. While this doesn't make a lot of sense, just
                // avoid crashing here.
                gc.tags.push(TagNode::Group(Tag::TOCI.into(), entry.nodes));
                return;
            };

            outline_ctx.insert(outline_nodes, outline_entry, entry.nodes);
            return;
        }
        StackEntryKind::Table(ctx) => ctx.build_table(entry.nodes),
        StackEntryKind::TableCell(cell) => {
            let Some(table_ctx) = gc.tags.stack.parent_table() else {
                // PDF/UA compliance of the structure hierarchy is checked
                // elsewhere. While this doesn't make a lot of sense, just
                // avoid crashing here.
                let tag = Tag::TD.with_location(Some(cell.span().into_raw()));
                gc.tags.push(TagNode::Group(tag.into(), entry.nodes));
                return;
            };

            table_ctx.insert(&cell, entry.nodes);
            return;
        }
        StackEntryKind::List(list) => list.build_list(entry.nodes),
        StackEntryKind::ListItemLabel => {
            let list_ctx = gc.tags.stack.parent_list().expect("parent list");
            list_ctx.push_label(entry.nodes);
            return;
        }
        StackEntryKind::ListItemBody => {
            let list_ctx = gc.tags.stack.parent_list().expect("parent list");
            list_ctx.push_body(entry.nodes);
            return;
        }
        StackEntryKind::BibEntry => {
            let list_ctx = gc.tags.stack.parent_list().expect("parent list");
            list_ctx.push_bib_entry(entry.nodes);
            return;
        }
        StackEntryKind::Figure(ctx) => {
            let tag = Tag::Figure(ctx.alt).with_bbox(ctx.bbox.get());
            TagNode::Group(tag.into(), entry.nodes)
        }
        StackEntryKind::Formula(ctx) => {
            let tag = Tag::Formula(ctx.alt).with_bbox(ctx.bbox.get());
            TagNode::Group(tag.into(), entry.nodes)
        }
        StackEntryKind::Link(_, link) => {
            let alt = link.alt.as_ref().map(EcoString::to_string);
            let tag = Tag::Link.with_alt_text(alt);
            let mut node = TagNode::Group(tag.into(), entry.nodes);
            // Wrap link in reference tag, if it's not a url.
            if let Destination::Position(_) | Destination::Location(_) = link.dest {
                node = TagNode::Group(Tag::Reference.into(), vec![node]);
            }
            node
        }
        StackEntryKind::FootnoteRef(decl_loc) => {
            // transparently insert all children.
            gc.tags.extend(entry.nodes);

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
            let tag = TagNode::Group(Tag::Note.into(), entry.nodes);
            let ctx = gc.tags.footnotes.entry(footnote_loc).or_insert(FootnoteCtx::new());
            ctx.entry = Some(tag);
            return;
        }
    };

    gc.tags.push(node);
}

fn pop_artifact(gc: &mut GlobalContext, surface: &mut Surface) {
    surface.end_tagged();
    gc.tags.in_artifact = None;
}

pub(crate) fn page_start(gc: &mut GlobalContext, surface: &mut Surface) {
    if gc.options.disable_tags {
        return;
    }

    if let Some((_, kind)) = gc.tags.in_artifact {
        let ty = artifact_type(kind);
        let id = surface.start_tagged(ContentTag::Artifact(ty));
        gc.tags.push(TagNode::Leaf(id));
    }
}

pub(crate) fn page_end(gc: &mut GlobalContext, surface: &mut Surface) {
    if gc.options.disable_tags {
        return;
    }

    if gc.tags.in_artifact.is_some() {
        surface.end_tagged();
    }
}

/// Add all annotations that were found in the page frame.
pub(crate) fn add_link_annotations(
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

pub(crate) fn update_bbox(
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

pub(crate) struct Tags {
    /// The intermediary stack of nested tag groups.
    pub(crate) stack: TagStack,
    /// A list of placeholders corresponding to a [`TagNode::Placeholder`].
    pub(crate) placeholders: Placeholders,
    /// Footnotes are inserted directly after the footenote reference in the
    /// reading order. Because of some layouting bugs, the entry might appear
    /// before the reference in the text, so we only resolve them once tags
    /// for the whole document are generated.
    pub(crate) footnotes: HashMap<Location, FootnoteCtx>,
    pub(crate) in_artifact: Option<(Location, ArtifactKind)>,
    /// Used to group multiple link annotations using quad points.
    link_id: LinkId,
    /// Used to generate IDs referenced in table `Headers` attributes.
    /// The IDs must be document wide unique.
    table_id: TableId,

    /// The output.
    pub(crate) tree: Vec<TagNode>,
}

impl Tags {
    pub(crate) fn new() -> Self {
        Self {
            stack: TagStack::new(),
            placeholders: Placeholders(Vec::new()),
            footnotes: HashMap::new(),
            in_artifact: None,

            link_id: LinkId(0),
            table_id: TableId(0),

            tree: Vec::new(),
        }
    }

    pub(crate) fn push(&mut self, node: TagNode) {
        if let Some(entry) = self.stack.last_mut() {
            entry.nodes.push(node);
        } else {
            self.tree.push(node);
        }
    }

    pub(crate) fn extend(&mut self, nodes: impl IntoIterator<Item = TagNode>) {
        if let Some(entry) = self.stack.last_mut() {
            entry.nodes.extend(nodes);
        } else {
            self.tree.extend(nodes);
        }
    }

    pub(crate) fn build_tree(&mut self) -> TagTree {
        let children = std::mem::take(&mut self.tree)
            .into_iter()
            .map(|node| self.resolve_node(node))
            .collect::<Vec<_>>();
        TagTree::from(children)
    }

    /// Resolves [`Placeholder`] nodes.
    fn resolve_node(&mut self, node: TagNode) -> Node {
        match node {
            TagNode::Group(tag, nodes) => {
                let children = nodes
                    .into_iter()
                    .map(|node| self.resolve_node(node))
                    .collect::<Vec<_>>();
                Node::Group(TagGroup::with_children(tag, children))
            }
            TagNode::Leaf(identifier) => Node::Leaf(identifier),
            TagNode::Placeholder(placeholder) => self.placeholders.take(placeholder),
            TagNode::FootnoteEntry(loc) => {
                let node = (self.footnotes.remove(&loc))
                    .and_then(|ctx| ctx.entry)
                    .expect("footnote");
                self.resolve_node(node)
            }
        }
    }

    fn context_supports(&self, _tag: &StackEntryKind) -> bool {
        // TODO: generate using: https://pdfa.org/resource/iso-ts-32005-hierarchical-inclusion-rules/
        true
    }

    pub(crate) fn next_link_id(&mut self) -> LinkId {
        self.link_id.0 += 1;
        self.link_id
    }

    fn next_table_id(&mut self) -> TableId {
        self.table_id.0 += 1;
        self.table_id
    }
}

#[derive(Debug)]
pub(crate) struct TagStack {
    items: Vec<StackEntry>,
    /// The index of the topmost stack entry that has a bbox.
    bbox_idx: Option<usize>,
}

impl<I: SliceIndex<[StackEntry]>> std::ops::Index<I> for TagStack {
    type Output = I::Output;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        std::ops::Index::index(&self.items, index)
    }
}

impl<I: SliceIndex<[StackEntry]>> std::ops::IndexMut<I> for TagStack {
    #[inline]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        std::ops::IndexMut::index_mut(&mut self.items, index)
    }
}

impl TagStack {
    pub(crate) fn new() -> Self {
        Self { items: Vec::new(), bbox_idx: None }
    }

    pub(crate) fn len(&self) -> usize {
        self.items.len()
    }

    pub(crate) fn last(&self) -> Option<&StackEntry> {
        self.items.last()
    }

    pub(crate) fn last_mut(&mut self) -> Option<&mut StackEntry> {
        self.items.last_mut()
    }

    pub(crate) fn iter(&self) -> std::slice::Iter<StackEntry> {
        self.items.iter()
    }

    pub(crate) fn push(&mut self, entry: StackEntry) {
        if entry.kind.bbox().is_some() {
            self.bbox_idx = Some(self.len());
        }
        self.items.push(entry);
    }

    pub(crate) fn extend(&mut self, iter: impl IntoIterator<Item = StackEntry>) {
        let start = self.len();
        self.items.extend(iter);
        let last_bbox_offset = self.items[start..]
            .iter()
            .rposition(|entry| entry.kind.bbox().is_some());
        if let Some(offset) = last_bbox_offset {
            self.bbox_idx = Some(start + offset);
        }
    }

    /// Remove the last stack entry if the predicate returns true.
    /// This takes care of updating the parent bboxes.
    pub(crate) fn pop_if(
        &mut self,
        mut predicate: impl FnMut(&mut StackEntry) -> bool,
    ) -> Option<StackEntry> {
        let last = self.items.last_mut()?;
        if predicate(last) { self.pop() } else { None }
    }

    /// Remove the last stack entry.
    /// This takes care of updating the parent bboxes.
    pub(crate) fn pop(&mut self) -> Option<StackEntry> {
        let removed = self.items.pop()?;

        let Some(inner_bbox) = removed.kind.bbox() else { return Some(removed) };

        self.bbox_idx = self.items.iter_mut().enumerate().rev().find_map(|(i, entry)| {
            let outer_bbox = entry.kind.bbox_mut()?;
            if let Some((page_idx, rect)) = inner_bbox.rect {
                outer_bbox.expand_page(page_idx, rect);
            }
            Some(i)
        });

        Some(removed)
    }

    pub(crate) fn parent(&mut self) -> Option<&mut StackEntryKind> {
        self.items.last_mut().map(|e| &mut e.kind)
    }

    pub(crate) fn parent_table(&mut self) -> Option<&mut TableCtx> {
        self.parent()?.as_table_mut()
    }

    pub(crate) fn parent_list(&mut self) -> Option<&mut ListCtx> {
        self.parent()?.as_list_mut()
    }

    pub(crate) fn parent_figure(&mut self) -> Option<&mut FigureCtx> {
        self.parent()?.as_figure_mut()
    }

    pub(crate) fn parent_outline(
        &mut self,
    ) -> Option<(&mut OutlineCtx, &mut Vec<TagNode>)> {
        self.items.last_mut().and_then(|e| {
            let ctx = e.kind.as_outline_mut()?;
            Some((ctx, &mut e.nodes))
        })
    }

    pub(crate) fn find_parent_link(
        &mut self,
    ) -> Option<(LinkId, &Packed<LinkMarker>, &mut Vec<TagNode>)> {
        self.items.iter_mut().rev().find_map(|e| {
            let (link_id, link) = e.kind.as_link()?;
            Some((link_id, link, &mut e.nodes))
        })
    }

    /// Finds the first parent that has a bounding box.
    pub(crate) fn find_parent_bbox(&mut self) -> Option<&mut BBoxCtx> {
        self.items[self.bbox_idx?].kind.bbox_mut()
    }
}

pub(crate) struct Placeholders(Vec<OnceCell<Node>>);

impl Placeholders {
    pub(crate) fn reserve(&mut self) -> Placeholder {
        let idx = self.0.len();
        self.0.push(OnceCell::new());
        Placeholder(idx)
    }

    pub(crate) fn init(&mut self, placeholder: Placeholder, node: Node) {
        self.0[placeholder.0]
            .set(node)
            .map_err(|_| ())
            .expect("placeholder to be uninitialized");
    }

    pub(crate) fn take(&mut self, placeholder: Placeholder) -> Node {
        self.0[placeholder.0].take().expect("initialized placeholder node")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct TableId(u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct LinkId(u32);

#[derive(Debug)]
pub(crate) struct StackEntry {
    pub(crate) loc: Location,
    pub(crate) span: Span,
    pub(crate) kind: StackEntryKind,
    pub(crate) nodes: Vec<TagNode>,
}

#[derive(Clone, Debug)]
pub(crate) enum StackEntryKind {
    Standard(TagKind),
    Outline(OutlineCtx),
    OutlineEntry(Packed<OutlineEntry>),
    Table(TableCtx),
    TableCell(Packed<TableCell>),
    List(ListCtx),
    ListItemLabel,
    ListItemBody,
    BibEntry,
    Figure(FigureCtx),
    Formula(FigureCtx),
    Link(LinkId, Packed<LinkMarker>),
    /// The footnote reference in the text, contains the declaration location.
    FootnoteRef(Location),
    /// The footnote entry at the end of the page. Contains the [`Location`] of
    /// the [`FootnoteElem`](typst_library::model::FootnoteElem).
    FootnoteEntry(Location),
}

impl StackEntryKind {
    pub(crate) fn as_outline_mut(&mut self) -> Option<&mut OutlineCtx> {
        if let Self::Outline(v) = self { Some(v) } else { None }
    }

    pub(crate) fn as_table_mut(&mut self) -> Option<&mut TableCtx> {
        if let Self::Table(v) = self { Some(v) } else { None }
    }

    pub(crate) fn as_list_mut(&mut self) -> Option<&mut ListCtx> {
        if let Self::List(v) = self { Some(v) } else { None }
    }

    pub(crate) fn as_figure_mut(&mut self) -> Option<&mut FigureCtx> {
        if let Self::Figure(v) = self { Some(v) } else { None }
    }

    pub(crate) fn as_link(&self) -> Option<(LinkId, &Packed<LinkMarker>)> {
        if let Self::Link(id, link) = self { Some((*id, link)) } else { None }
    }

    pub(crate) fn bbox(&self) -> Option<&BBoxCtx> {
        match self {
            Self::Table(ctx) => Some(&ctx.bbox),
            Self::Figure(ctx) => Some(&ctx.bbox),
            Self::Formula(ctx) => Some(&ctx.bbox),
            _ => None,
        }
    }

    pub(crate) fn bbox_mut(&mut self) -> Option<&mut BBoxCtx> {
        match self {
            Self::Table(ctx) => Some(&mut ctx.bbox),
            Self::Figure(ctx) => Some(&mut ctx.bbox),
            Self::Formula(ctx) => Some(&mut ctx.bbox),
            _ => None,
        }
    }

    fn is_breakable(&self, is_pdf_ua: bool) -> bool {
        match self {
            StackEntryKind::Standard(tag) => match tag {
                TagKind::Part(_) => !is_pdf_ua,
                TagKind::Article(_) => !is_pdf_ua,
                TagKind::Section(_) => !is_pdf_ua,
                TagKind::Div(_) => !is_pdf_ua,
                TagKind::BlockQuote(_) => !is_pdf_ua,
                TagKind::Caption(_) => !is_pdf_ua,
                TagKind::TOC(_) => false,
                TagKind::TOCI(_) => false,
                TagKind::Index(_) => false,
                TagKind::P(_) => true,
                TagKind::Hn(_) => !is_pdf_ua,
                TagKind::L(_) => false,
                TagKind::LI(_) => false,
                TagKind::Lbl(_) => !is_pdf_ua,
                TagKind::LBody(_) => !is_pdf_ua,
                TagKind::Table(_) => false,
                TagKind::TR(_) => false,
                // TODO: disallow table/grid cells outside of tables/grids
                TagKind::TH(_) => false,
                TagKind::TD(_) => false,
                TagKind::THead(_) => false,
                TagKind::TBody(_) => false,
                TagKind::TFoot(_) => false,
                TagKind::Span(_) => false,
                TagKind::InlineQuote(_) => !is_pdf_ua,
                TagKind::Note(_) => !is_pdf_ua,
                TagKind::Reference(_) => !is_pdf_ua,
                TagKind::BibEntry(_) => !is_pdf_ua,
                TagKind::Code(_) => !is_pdf_ua,
                TagKind::Link(_) => !is_pdf_ua,
                TagKind::Annot(_) => !is_pdf_ua,
                TagKind::Figure(_) => !is_pdf_ua,
                TagKind::Formula(_) => !is_pdf_ua,
                TagKind::NonStruct(_) => !is_pdf_ua,
                TagKind::Datetime(_) => !is_pdf_ua,
                TagKind::Terms(_) => !is_pdf_ua,
                TagKind::Title(_) => !is_pdf_ua,
            },
            StackEntryKind::Outline(_) => false,
            StackEntryKind::OutlineEntry(_) => false,
            StackEntryKind::Table(_) => false,
            StackEntryKind::TableCell(_) => false,
            StackEntryKind::List(_) => false,
            StackEntryKind::ListItemLabel => false,
            StackEntryKind::ListItemBody => false,
            StackEntryKind::BibEntry => false,
            StackEntryKind::Figure(_) => false,
            StackEntryKind::Formula(_) => false,
            StackEntryKind::Link(..) => !is_pdf_ua,
            StackEntryKind::FootnoteRef(_) => false,
            StackEntryKind::FootnoteEntry(_) => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FootnoteCtx {
    /// Whether this footenote has been referenced inside the document. The
    /// entry will be inserted inside the reading order after the first
    /// reference. All other references will still have links to the footnote.
    is_referenced: bool,
    /// The nodes that make up the footnote entry.
    entry: Option<TagNode>,
}

impl FootnoteCtx {
    pub const fn new() -> Self {
        Self { is_referenced: false, entry: None }
    }
}

/// Figure/Formula context
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FigureCtx {
    alt: Option<String>,
    bbox: BBoxCtx,
}

impl FigureCtx {
    fn new(alt: Option<String>) -> Self {
        Self { alt, bbox: BBoxCtx::new() }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BBoxCtx {
    rect: Option<(usize, Rect)>,
    multi_page: bool,
}

impl BBoxCtx {
    pub(crate) fn new() -> Self {
        Self { rect: None, multi_page: false }
    }

    pub(crate) fn reset(&mut self) {
        *self = Self::new();
    }

    /// Expand the bounding box with a `rect` relative to the current frame
    /// context transform.
    pub(crate) fn expand_frame(&mut self, fc: &FrameContext, rect: Rect) {
        let Some(page_idx) = fc.page_idx else { return };
        if self.multi_page {
            return;
        }
        let (idx, bbox) = self.rect.get_or_insert((
            page_idx,
            Rect::new(Point::splat(Abs::inf()), Point::splat(-Abs::inf())),
        ));
        if *idx != page_idx {
            self.multi_page = true;
            self.rect = None;
            return;
        }

        let size = rect.size();
        for point in [
            rect.min,
            rect.min + Point::with_x(size.x),
            rect.min + Point::with_y(size.y),
            rect.max,
        ] {
            let p = point.transform(fc.state().transform());
            bbox.min = bbox.min.min(p);
            bbox.max = bbox.max.max(p);
        }
    }

    /// Expand the bounding box with a rectangle that's already transformed into
    /// page coordinates.
    pub(crate) fn expand_page(&mut self, page_idx: usize, rect: Rect) {
        if self.multi_page {
            return;
        }
        let (idx, bbox) = self.rect.get_or_insert((
            page_idx,
            Rect::new(Point::splat(Abs::inf()), Point::splat(-Abs::inf())),
        ));
        if *idx != page_idx {
            self.multi_page = true;
            self.rect = None;
            return;
        }

        bbox.min = bbox.min.min(rect.min);
        bbox.max = bbox.max.max(rect.max);
    }

    pub(crate) fn get(&self) -> Option<BBox> {
        let (page_idx, rect) = self.rect?;
        let rect = kg::Rect::from_ltrb(
            rect.min.x.to_f32(),
            rect.min.y.to_f32(),
            rect.max.x.to_f32(),
            rect.max.y.to_f32(),
        )
        .unwrap();
        Some(BBox::new(page_idx, rect))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum TagNode {
    Group(TagKind, Vec<TagNode>),
    Leaf(Identifier),
    /// Allows inserting a placeholder into the tag tree.
    /// Currently used for [`krilla::page::Page::add_tagged_annotation`].
    Placeholder(Placeholder),
    FootnoteEntry(Location),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Placeholder(usize);

/// Automatically calls [`Surface::end_tagged`] when dropped.
pub(crate) struct TagHandle<'a, 'b> {
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
    pub(crate) fn surface<'c>(&'c mut self) -> &'c mut Surface<'a> {
        self.surface
    }
}

/// Returns a [`TagHandle`] that automatically calls [`Surface::end_tagged`]
/// when dropped.
pub(crate) fn start_span<'a, 'b>(
    gc: &mut GlobalContext,
    surface: &'b mut Surface<'a>,
    span: SpanTag,
) -> TagHandle<'a, 'b> {
    start_content(gc, surface, ContentTag::Span(span))
}

/// Returns a [`TagHandle`] that automatically calls [`Surface::end_tagged`]
/// when dropped.
pub(crate) fn start_artifact<'a, 'b>(
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
    } else if let Some(StackEntryKind::Table(_)) = gc.tags.stack.last().map(|e| &e.kind) {
        // Mark any direct child of a table as an aritfact. Any real content
        // will be wrapped inside a `TableCell`.
        ContentTag::Artifact(ArtifactType::Other)
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

trait PropertyGetAsRef<E, T, const I: u8> {
    fn get_as_ref(&self) -> Option<&T>;
}

impl<E, T, const I: u8> PropertyGetAsRef<E, T, I> for Settable<E, I>
where
    E: NativeElement,
    E: SettableProperty<I, Type = Option<T>>,
    E: RefableProperty<I>,
{
    fn get_as_ref(&self) -> Option<&T> {
        self.get_ref(StyleChain::default()).as_ref()
    }
}
