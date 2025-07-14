use std::cell::OnceCell;
use std::num::NonZeroU16;

use ecow::EcoString;
use krilla::page::Page;
use krilla::surface::Surface;
use krilla::tagging::{
    ArtifactType, ContentTag, Identifier, ListNumbering, Node, SpanTag, Tag, TagGroup,
    TagKind, TagTree,
};
use typst_library::foundations::{Content, Packed, StyleChain};
use typst_library::introspection::Location;
use typst_library::layout::RepeatElem;
use typst_library::model::{
    EnumElem, FigureCaption, FigureElem, HeadingElem, LinkMarker, ListElem, Outlinable,
    OutlineEntry, TableCell, TableElem, TermsElem,
};
use typst_library::pdf::{ArtifactElem, ArtifactKind, PdfMarkerTag, PdfMarkerTagKind};
use typst_library::visualize::ImageElem;

use crate::convert::GlobalContext;
use crate::link::LinkAnnotation;
use crate::tags::list::ListCtx;
use crate::tags::outline::OutlineCtx;
use crate::tags::table::TableCtx;

mod list;
mod outline;
mod table;

pub fn handle_start(gc: &mut GlobalContext, surface: &mut Surface, elem: &Content) {
    if gc.tags.in_artifact.is_some() {
        // Don't nest artifacts
        return;
    }

    let loc = elem.location().expect("elem to be locatable");

    if let Some(artifact) = elem.to_packed::<ArtifactElem>() {
        let kind = artifact.kind.get(StyleChain::default());
        push_artifact(gc, surface, loc, kind);
        return;
    } else if let Some(_) = elem.to_packed::<RepeatElem>() {
        push_artifact(gc, surface, loc, ArtifactKind::Other);
        return;
    }

    let tag = if let Some(tag) = elem.to_packed::<PdfMarkerTag>() {
        match &tag.kind {
            PdfMarkerTagKind::OutlineBody => {
                push_stack(gc, loc, StackEntryKind::Outline(OutlineCtx::new()));
                return;
            }
            PdfMarkerTagKind::FigureBody(alt) => {
                let alt = alt.as_ref().map(|s| s.to_string());
                Tag::Figure(alt).into()
            }
            PdfMarkerTagKind::ListItemLabel => {
                push_stack(gc, loc, StackEntryKind::ListItemLabel);
                return;
            }
            PdfMarkerTagKind::ListItemBody => {
                push_stack(gc, loc, StackEntryKind::ListItemBody);
                return;
            }
            PdfMarkerTagKind::Label => Tag::Lbl.into(),
        }
    } else if let Some(entry) = elem.to_packed::<OutlineEntry>() {
        push_stack(gc, loc, StackEntryKind::OutlineEntry(entry.clone()));
        return;
    } else if let Some(_list) = elem.to_packed::<ListElem>() {
        let numbering = ListNumbering::Circle; // TODO: infer numbering from `list.marker`
        push_stack(gc, loc, StackEntryKind::List(ListCtx::new(numbering)));
        return;
    } else if let Some(_enumeration) = elem.to_packed::<EnumElem>() {
        let numbering = ListNumbering::Decimal; // TODO: infer numbering from `enum.numbering`
        push_stack(gc, loc, StackEntryKind::List(ListCtx::new(numbering)));
        return;
    } else if let Some(_terms) = elem.to_packed::<TermsElem>() {
        let numbering = ListNumbering::None;
        push_stack(gc, loc, StackEntryKind::List(ListCtx::new(numbering)));
        return;
    } else if let Some(_) = elem.to_packed::<FigureElem>() {
        // Wrap the figure tag and the sibling caption in a container, if the
        // caption is contained within the figure like recommended for tables
        // screen readers might ignore it.
        Tag::NonStruct.into()
    } else if let Some(_) = elem.to_packed::<FigureCaption>() {
        Tag::Caption.into()
    } else if let Some(image) = elem.to_packed::<ImageElem>() {
        let alt = image.alt.get_cloned(StyleChain::default()).map(|s| s.to_string());

        if let Some(StackEntryKind::Standard(TagKind::Figure(tag))) = gc.tags.parent() {
            // Set alt text of outer figure tag, if not present.
            if tag.alt_text().is_none() {
                tag.set_alt_text(alt);
            }
            return;
        } else {
            Tag::Figure(alt).into()
        }
    } else if let Some(table) = elem.to_packed::<TableElem>() {
        let table_id = gc.tags.next_table_id();
        let summary = table
            .summary
            .get_ref(StyleChain::default())
            .as_ref()
            .map(EcoString::to_string);
        let ctx = TableCtx::new(table_id, summary);
        push_stack(gc, loc, StackEntryKind::Table(ctx));
        return;
    } else if let Some(cell) = elem.to_packed::<TableCell>() {
        let table_ctx = gc.tags.parent_table();

        // Only repeated table headers and footer cells are laid out multiple
        // times. Mark duplicate headers as artifacts, since they have no
        // semantic meaning in the tag tree, which doesn't use page breaks for
        // it's semantic structure.
        if cell.is_repeated.get(StyleChain::default())
            || table_ctx.is_some_and(|ctx| ctx.contains(cell))
        {
            push_artifact(gc, surface, loc, ArtifactKind::Other);
        } else {
            push_stack(gc, loc, StackEntryKind::TableCell(cell.clone()));
        }
        return;
    } else if let Some(heading) = elem.to_packed::<HeadingElem>() {
        let level = heading.level().try_into().unwrap_or(NonZeroU16::MAX);
        let name = heading.body.plain_text().to_string();
        Tag::Hn(level, Some(name)).into()
    } else if let Some(link) = elem.to_packed::<LinkMarker>() {
        let link_id = gc.tags.next_link_id();
        push_stack(gc, loc, StackEntryKind::Link(link_id, link.clone()));
        return;
    } else {
        return;
    };

    push_stack(gc, loc, StackEntryKind::Standard(tag));
}

pub fn handle_end(gc: &mut GlobalContext, surface: &mut Surface, loc: Location) {
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
        StackEntryKind::Standard(tag) => TagNode::group(tag, entry.nodes),
        StackEntryKind::Outline(ctx) => ctx.build_outline(entry.nodes),
        StackEntryKind::OutlineEntry(outline_entry) => {
            let parent = gc.tags.stack.last_mut().and_then(|parent| {
                let ctx = parent.kind.as_outline_mut()?;
                Some((&mut parent.nodes, ctx))
            });
            let Some((parent_nodes, outline_ctx)) = parent else {
                // PDF/UA compliance of the structure hierarchy is checked
                // elsewhere. While this doesn't make a lot of sense, just
                // avoid crashing here.
                gc.tags.push(TagNode::group(Tag::TOCI, entry.nodes));
                return;
            };

            outline_ctx.insert(parent_nodes, outline_entry, entry.nodes);
            return;
        }
        StackEntryKind::Table(ctx) => ctx.build_table(entry.nodes),
        StackEntryKind::TableCell(cell) => {
            let Some(table_ctx) = gc.tags.parent_table() else {
                // PDF/UA compliance of the structure hierarchy is checked
                // elsewhere. While this doesn't make a lot of sense, just
                // avoid crashing here.
                gc.tags.push(TagNode::group(Tag::TD, entry.nodes));
                return;
            };

            table_ctx.insert(&cell, entry.nodes);
            return;
        }
        StackEntryKind::List(list) => list.build_list(entry.nodes),
        StackEntryKind::ListItemLabel => {
            let list_ctx = gc.tags.parent_list().expect("parent list");
            list_ctx.push_label(entry.nodes);
            return;
        }
        StackEntryKind::ListItemBody => {
            let list_ctx = gc.tags.parent_list().expect("parent list");
            list_ctx.push_body(entry.nodes);
            return;
        }
        StackEntryKind::Link(_, _) => {
            let mut node = TagNode::group(Tag::Link, entry.nodes);
            // Wrap link in reference tag if inside an outline entry.
            if gc.tags.parent_outline_entry().is_some() {
                node = TagNode::group(Tag::Reference, vec![node]);
            }
            node
        }
    };

    gc.tags.push(node);
}

fn push_stack(gc: &mut GlobalContext, loc: Location, kind: StackEntryKind) {
    gc.tags.stack.push(StackEntry { loc, kind, nodes: Vec::new() });
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

pub fn page_start(gc: &mut GlobalContext, surface: &mut Surface) {
    if let Some((_, kind)) = gc.tags.in_artifact {
        let ty = artifact_type(kind);
        let id = surface.start_tagged(ContentTag::Artifact(ty));
        gc.tags.push(TagNode::Leaf(id));
    }
}

pub fn page_end(gc: &mut GlobalContext, surface: &mut Surface) {
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
        );
        let annot_id = page.add_tagged_annotation(annotation);
        gc.tags.init_placeholder(a.placeholder, Node::Leaf(annot_id));
    }
}

pub struct Tags {
    /// The intermediary stack of nested tag groups.
    pub stack: Vec<StackEntry>,
    /// A list of placeholders corresponding to a [`TagNode::Placeholder`].
    pub placeholders: Vec<OnceCell<Node>>,
    pub in_artifact: Option<(Location, ArtifactKind)>,
    /// Used to group multiple link annotations using quad points.
    pub link_id: LinkId,
    /// Used to generate IDs referenced in table `Headers` attributes.
    /// The IDs must be document wide unique.
    pub table_id: TableId,

    /// The output.
    pub tree: Vec<TagNode>,
}

impl Tags {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            placeholders: Vec::new(),
            in_artifact: None,

            tree: Vec::new(),
            link_id: LinkId(0),
            table_id: TableId(0),
        }
    }

    pub fn reserve_placeholder(&mut self) -> Placeholder {
        let idx = self.placeholders.len();
        self.placeholders.push(OnceCell::new());
        Placeholder(idx)
    }

    pub fn init_placeholder(&mut self, placeholder: Placeholder, node: Node) {
        self.placeholders[placeholder.0]
            .set(node)
            .map_err(|_| ())
            .expect("placeholder to be uninitialized");
    }

    pub fn take_placeholder(&mut self, placeholder: Placeholder) -> Node {
        self.placeholders[placeholder.0]
            .take()
            .expect("initialized placeholder node")
    }

    pub fn parent(&mut self) -> Option<&mut StackEntryKind> {
        self.stack.last_mut().map(|e| &mut e.kind)
    }

    pub fn parent_table(&mut self) -> Option<&mut TableCtx> {
        self.parent()?.as_table_mut()
    }

    pub fn parent_list(&mut self) -> Option<&mut ListCtx> {
        self.parent()?.as_list_mut()
    }

    pub fn parent_outline_entry(&mut self) -> Option<&mut OutlineEntry> {
        self.parent()?.as_outline_entry_mut()
    }

    pub fn find_parent_link(&self) -> Option<(LinkId, &Packed<LinkMarker>)> {
        self.stack.iter().rev().find_map(|entry| entry.kind.as_link())
    }

    pub fn push(&mut self, node: TagNode) {
        if let Some(entry) = self.stack.last_mut() {
            entry.nodes.push(node);
        } else {
            self.tree.push(node);
        }
    }

    pub fn build_tree(&mut self) -> TagTree {
        assert!(self.stack.is_empty(), "tags weren't properly closed");

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
            TagNode::Placeholder(placeholder) => self.take_placeholder(placeholder),
        }
    }

    fn next_link_id(&mut self) -> LinkId {
        self.link_id.0 += 1;
        self.link_id
    }

    fn next_table_id(&mut self) -> TableId {
        self.table_id.0 += 1;
        self.table_id
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TableId(u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LinkId(u32);

#[derive(Debug)]
pub struct StackEntry {
    pub loc: Location,
    pub kind: StackEntryKind,
    pub nodes: Vec<TagNode>,
}

#[derive(Debug)]
pub enum StackEntryKind {
    Standard(TagKind),
    Outline(OutlineCtx),
    OutlineEntry(Packed<OutlineEntry>),
    Table(TableCtx),
    TableCell(Packed<TableCell>),
    List(ListCtx),
    ListItemLabel,
    ListItemBody,
    Link(LinkId, Packed<LinkMarker>),
}

impl StackEntryKind {
    pub fn as_outline_mut(&mut self) -> Option<&mut OutlineCtx> {
        if let Self::Outline(v) = self { Some(v) } else { None }
    }

    pub fn as_outline_entry_mut(&mut self) -> Option<&mut OutlineEntry> {
        if let Self::OutlineEntry(v) = self { Some(v) } else { None }
    }

    pub fn as_table_mut(&mut self) -> Option<&mut TableCtx> {
        if let Self::Table(v) = self { Some(v) } else { None }
    }

    pub fn as_list_mut(&mut self) -> Option<&mut ListCtx> {
        if let Self::List(v) = self { Some(v) } else { None }
    }

    pub fn as_link(&self) -> Option<(LinkId, &Packed<LinkMarker>)> {
        if let Self::Link(id, link) = self { Some((*id, link)) } else { None }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TagNode {
    Group(TagKind, Vec<TagNode>),
    Leaf(Identifier),
    /// Allows inserting a placeholder into the tag tree.
    /// Currently used for [`krilla::page::Page::add_tagged_annotation`].
    Placeholder(Placeholder),
}

impl TagNode {
    pub fn group(tag: impl Into<TagKind>, children: Vec<TagNode>) -> Self {
        TagNode::Group(tag.into(), children)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Placeholder(usize);

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
