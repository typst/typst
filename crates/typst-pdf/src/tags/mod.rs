use std::cell::OnceCell;
use std::collections::HashMap;
use std::num::NonZeroU32;

use ecow::EcoString;
use krilla::configure::Validator;
use krilla::geom as kg;
use krilla::page::Page;
use krilla::surface::Surface;
use krilla::tagging::{
    ArtifactType, BBox, ContentTag, Identifier, ListNumbering, Node, SpanTag, Tag,
    TagGroup, TagKind, TagTree,
};
use typst_library::diag::SourceResult;
use typst_library::foundations::{
    Content, LinkMarker, NativeElement, Packed, RefableProperty, Settable,
    SettableProperty, StyleChain,
};
use typst_library::introspection::Location;
use typst_library::layout::{Abs, Point, Rect, RepeatElem};
use typst_library::math::EquationElem;
use typst_library::model::{
    Destination, EnumElem, FigureCaption, FigureElem, FootnoteElem, FootnoteEntry,
    HeadingElem, ListElem, Outlinable, OutlineEntry, QuoteElem, TableCell, TableElem,
    TermsElem,
};
use typst_library::pdf::{ArtifactElem, ArtifactKind, PdfMarkerTag, PdfMarkerTagKind};
use typst_library::visualize::ImageElem;

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

    let loc = elem.location().expect("elem to be locatable");

    if let Some(artifact) = elem.to_packed::<ArtifactElem>() {
        let kind = artifact.kind.get(StyleChain::default());
        push_artifact(gc, surface, loc, kind);
        return Ok(());
    } else if let Some(_) = elem.to_packed::<RepeatElem>() {
        push_artifact(gc, surface, loc, ArtifactKind::Other);
        return Ok(());
    }

    let mut tag: TagKind = if let Some(tag) = elem.to_packed::<PdfMarkerTag>() {
        match &tag.kind {
            PdfMarkerTagKind::OutlineBody => {
                push_stack(gc, loc, StackEntryKind::Outline(OutlineCtx::new()))?;
                return Ok(());
            }
            PdfMarkerTagKind::FigureBody(alt) => {
                let alt = alt.as_ref().map(|s| s.to_string());
                push_stack(gc, loc, StackEntryKind::Figure(FigureCtx::new(alt)))?;
                return Ok(());
            }
            PdfMarkerTagKind::Bibliography(numbered) => {
                let numbering =
                    if *numbered { ListNumbering::Decimal } else { ListNumbering::None };
                push_stack(gc, loc, StackEntryKind::List(ListCtx::new(numbering)))?;
                return Ok(());
            }
            PdfMarkerTagKind::BibEntry => {
                push_stack(gc, loc, StackEntryKind::BibEntry)?;
                return Ok(());
            }
            PdfMarkerTagKind::ListItemLabel => {
                push_stack(gc, loc, StackEntryKind::ListItemLabel)?;
                return Ok(());
            }
            PdfMarkerTagKind::ListItemBody => {
                push_stack(gc, loc, StackEntryKind::ListItemBody)?;
                return Ok(());
            }
            PdfMarkerTagKind::Label => Tag::Lbl.into(),
        }
    } else if let Some(entry) = elem.to_packed::<OutlineEntry>() {
        push_stack(gc, loc, StackEntryKind::OutlineEntry(entry.clone()))?;
        return Ok(());
    } else if let Some(_list) = elem.to_packed::<ListElem>() {
        let numbering = ListNumbering::Circle; // TODO: infer numbering from `list.marker`
        push_stack(gc, loc, StackEntryKind::List(ListCtx::new(numbering)))?;
        return Ok(());
    } else if let Some(_enumeration) = elem.to_packed::<EnumElem>() {
        let numbering = ListNumbering::Decimal; // TODO: infer numbering from `enum.numbering`
        push_stack(gc, loc, StackEntryKind::List(ListCtx::new(numbering)))?;
        return Ok(());
    } else if let Some(_enumeration) = elem.to_packed::<TermsElem>() {
        let numbering = ListNumbering::None;
        push_stack(gc, loc, StackEntryKind::List(ListCtx::new(numbering)))?;
        return Ok(());
    } else if let Some(_) = elem.to_packed::<FigureElem>() {
        // Wrap the figure tag and the sibling caption in a container, if the
        // caption is contained within the figure like recommended for tables
        // screen readers might ignore it.
        // TODO: maybe this could be a `NonStruct` tag?
        Tag::P.into()
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
            push_stack(gc, loc, StackEntryKind::Figure(FigureCtx::new(alt)))?;
            return Ok(());
        }
    } else if let Some(equation) = elem.to_packed::<EquationElem>() {
        let alt = equation.alt.get_as_ref().map(|s| s.to_string());
        push_stack(gc, loc, StackEntryKind::Formula(FigureCtx::new(alt)))?;
        return Ok(());
    } else if let Some(table) = elem.to_packed::<TableElem>() {
        let table_id = gc.tags.next_table_id();
        let summary = table.summary.get_as_ref().map(|s| s.to_string());
        let ctx = TableCtx::new(table_id, summary);
        push_stack(gc, loc, StackEntryKind::Table(ctx))?;
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
            push_artifact(gc, surface, loc, ArtifactKind::Other);
        } else {
            push_stack(gc, loc, StackEntryKind::TableCell(cell.clone()))?;
        }
        return Ok(());
    } else if let Some(heading) = elem.to_packed::<HeadingElem>() {
        let level = heading.level().try_into().unwrap_or(NonZeroU32::MAX);
        let name = heading.body.plain_text().to_string();
        Tag::Hn(level, Some(name)).into()
    } else if let Some(link) = elem.to_packed::<LinkMarker>() {
        let link_id = gc.tags.next_link_id();
        push_stack(gc, loc, StackEntryKind::Link(link_id, link.clone()))?;
        return Ok(());
    } else if let Some(_) = elem.to_packed::<FootnoteElem>() {
        push_stack(gc, loc, StackEntryKind::FootNoteRef)?;
        return Ok(());
    } else if let Some(entry) = elem.to_packed::<FootnoteEntry>() {
        let footnote_loc = entry.note.location().unwrap();
        push_stack(gc, loc, StackEntryKind::FootNoteEntry(footnote_loc))?;
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

    tag.set_location(Some(elem.span().into_raw().get()));
    push_stack(gc, loc, StackEntryKind::Standard(tag))?;

    Ok(())
}

pub(crate) fn handle_end(gc: &mut GlobalContext, surface: &mut Surface, loc: Location) {
    if gc.options.disable_tags {
        return;
    }

    if let Some((l, _)) = gc.tags.in_artifact {
        if l == loc {
            pop_artifact(gc, surface);
        }
        return;
    }

    let Some(entry) = gc.tags.stack.pop_if(|e| e.loc == loc) else {
        return;
    };

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
                let tag = Tag::TD.with_location(Some(cell.span().into_raw().get()));
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
        StackEntryKind::FootNoteRef => {
            // transparently inset all children.
            gc.tags.extend(entry.nodes);
            gc.tags.push(TagNode::FootnoteEntry(loc));
            return;
        }
        StackEntryKind::FootNoteEntry(footnote_loc) => {
            // Store footnotes separately so they can be inserted directly after
            // the footnote reference in the reading order.
            let tag = TagNode::Group(Tag::Note.into(), entry.nodes);
            gc.tags.footnotes.insert(footnote_loc, tag);
            return;
        }
    };

    gc.tags.push(node);
}

fn push_stack(
    gc: &mut GlobalContext,
    loc: Location,
    kind: StackEntryKind,
) -> SourceResult<()> {
    if !gc.tags.context_supports(&kind) {
        if gc.options.standards.config.validator() == Validator::UA1 {
            // TODO: error
        } else {
            // TODO: warning
        }
    }

    gc.tags.stack.push(StackEntry { loc, kind, nodes: Vec::new() });

    Ok(())
}

fn push_artifact(
    gc: &mut GlobalContext,
    surface: &mut Surface,
    loc: Location,
    kind: ArtifactKind,
) {
    let ty = artifact_type(kind);
    let id = surface.start_tagged(ContentTag::Artifact(ty));
    gc.tags.push(TagNode::Leaf(id));
    gc.tags.in_artifact = Some((loc, kind));
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
    for annotation in annotations.into_iter() {
        let LinkAnnotation { id: _, placeholder, alt, quad_points, target, span } =
            annotation;
        let annot = krilla::annotation::Annotation::new_link(
            krilla::annotation::LinkAnnotation::new_with_quad_points(quad_points, target),
            alt,
        )
        .with_location(Some(span.into_raw().get()));

        if gc.options.disable_tags {
            page.add_annotation(annot);
        } else {
            let annot_id = page.add_tagged_annotation(annot);
            gc.tags.placeholders.init(placeholder, Node::Leaf(annot_id));
        }
    }
}

pub(crate) fn update_bbox(
    gc: &mut GlobalContext,
    fc: &FrameContext,
    compute_bbox: impl FnOnce() -> Rect,
) {
    if gc.options.standards.config.validator() == Validator::UA1
        && let Some(bbox) = gc.tags.stack.find_parent_bbox()
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
    pub(crate) footnotes: HashMap<Location, TagNode>,
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
            stack: TagStack(Vec::new()),
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
                let node = self.footnotes.remove(&loc).expect("footnote");
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

pub(crate) struct TagStack(Vec<StackEntry>);

impl TagStack {
    pub(crate) fn last(&self) -> Option<&StackEntry> {
        self.0.last()
    }

    pub(crate) fn last_mut(&mut self) -> Option<&mut StackEntry> {
        self.0.last_mut()
    }

    pub(crate) fn push(&mut self, entry: StackEntry) {
        self.0.push(entry);
    }

    pub(crate) fn pop_if(
        &mut self,
        predicate: impl FnMut(&mut StackEntry) -> bool,
    ) -> Option<StackEntry> {
        let entry = self.0.pop_if(predicate)?;

        // TODO: If tags of the items were overlapping, only updating the
        // direct parent bounding box might produce too large bounding boxes.
        if let Some((page_idx, rect)) = entry.kind.bbox().and_then(|b| b.rect)
            && let Some(parent) = self.find_parent_bbox()
        {
            parent.expand_page(page_idx, rect);
        }

        Some(entry)
    }

    pub(crate) fn parent(&mut self) -> Option<&mut StackEntryKind> {
        self.0.last_mut().map(|e| &mut e.kind)
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
        self.0.last_mut().and_then(|e| {
            let ctx = e.kind.as_outline_mut()?;
            Some((ctx, &mut e.nodes))
        })
    }

    pub(crate) fn find_parent_link(
        &mut self,
    ) -> Option<(LinkId, &Packed<LinkMarker>, &mut Vec<TagNode>)> {
        self.0.iter_mut().rev().find_map(|e| {
            let (link_id, link) = e.kind.as_link()?;
            Some((link_id, link, &mut e.nodes))
        })
    }

    /// Finds the first parent that has a bounding box.
    pub(crate) fn find_parent_bbox(&mut self) -> Option<&mut BBoxCtx> {
        self.0.iter_mut().rev().find_map(|e| e.kind.bbox_mut())
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
    pub(crate) kind: StackEntryKind,
    pub(crate) nodes: Vec<TagNode>,
}

#[derive(Debug)]
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
    /// The footnote reference in the text.
    FootNoteRef,
    /// The footnote entry at the end of the page. Contains the [`Location`] of
    /// the [`FootnoteElem`](typst_library::model::FootnoteElem).
    FootNoteEntry(Location),
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
        Some(BBox::new(page_idx as usize, rect))
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
