use std::cell::OnceCell;
use std::num::NonZeroU16;

use ecow::EcoString;
use krilla::page::Page;
use krilla::surface::Surface;
use krilla::tagging::{
    ArtifactType, ContentTag, Identifier, Node, SpanTag, Tag, TagGroup, TagKind, TagTree,
};
use typst_library::foundations::{Content, Packed, StyleChain};
use typst_library::introspection::Location;
use typst_library::layout::RepeatElem;
use typst_library::model::{
    FigureCaption, FigureElem, HeadingElem, LinkMarker, Outlinable, OutlineBody,
    OutlineEntry, TableCell, TableElem,
};
use typst_library::pdf::{ArtifactElem, ArtifactKind};
use typst_library::visualize::ImageElem;

use crate::convert::GlobalContext;
use crate::link::LinkAnnotation;
use crate::tags::outline::OutlineCtx;
use crate::tags::table::TableCtx;

mod outline;
mod table;

pub fn handle_start(gc: &mut GlobalContext, elem: &Content) {
    if gc.tags.in_artifact.is_some() {
        // Don't nest artifacts
        return;
    }

    let loc = elem.location().expect("elem to be locatable");

    if let Some(artifact) = elem.to_packed::<ArtifactElem>() {
        let kind = artifact.kind.get(StyleChain::default());
        start_artifact(gc, loc, kind);
        return;
    } else if let Some(_) = elem.to_packed::<RepeatElem>() {
        start_artifact(gc, loc, ArtifactKind::Other);
        return;
    }

    let tag = if let Some(heading) = elem.to_packed::<HeadingElem>() {
        let level = heading.level().try_into().unwrap_or(NonZeroU16::MAX);
        let name = heading.body.plain_text().to_string();
        Tag::Hn(level, Some(name)).into()
    } else if let Some(_) = elem.to_packed::<OutlineBody>() {
        push_stack(gc, loc, StackEntryKind::Outline(OutlineCtx::new()));
        return;
    } else if let Some(entry) = elem.to_packed::<OutlineEntry>() {
        push_stack(gc, loc, StackEntryKind::OutlineEntry(entry.clone()));
        return;
    } else if let Some(figure) = elem.to_packed::<FigureElem>() {
        let alt = figure.alt.get_cloned(StyleChain::default()).map(|s| s.to_string());
        Tag::Figure(alt).into()
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
    } else if let Some(_) = elem.to_packed::<FigureCaption>() {
        Tag::Caption.into()
    } else if let Some(table) = elem.to_packed::<TableElem>() {
        push_stack(gc, loc, StackEntryKind::Table(TableCtx::new(table.clone())));
        return;
    } else if let Some(cell) = elem.to_packed::<TableCell>() {
        push_stack(gc, loc, StackEntryKind::TableCell(cell.clone()));
        return;
    } else if let Some(link) = elem.to_packed::<LinkMarker>() {
        let link_id = gc.tags.next_link_id();
        push_stack(gc, loc, StackEntryKind::Link(link_id, link.clone()));
        return;
    } else {
        return;
    };

    push_stack(gc, loc, StackEntryKind::Standard(tag));
}

fn push_stack(gc: &mut GlobalContext, loc: Location, kind: StackEntryKind) {
    gc.tags.stack.push(StackEntry { loc, kind, nodes: Vec::new() });
}

pub fn handle_end(gc: &mut GlobalContext, loc: Location) {
    if let Some((l, _)) = gc.tags.in_artifact {
        if l == loc {
            gc.tags.in_artifact = None;
        }
        return;
    }

    let Some(entry) = gc.tags.stack.pop_if(|e| e.loc == loc) else {
        return;
    };

    let node = match entry.kind {
        StackEntryKind::Standard(tag) => TagNode::group(tag, entry.nodes),
        StackEntryKind::Outline(ctx) => {
            let nodes = ctx.build_outline(entry.nodes);
            TagNode::group(Tag::TOC, nodes)
        }
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
        StackEntryKind::Table(ctx) => {
            let summary = ctx
                .table
                .summary
                .get_ref(StyleChain::default())
                .as_ref()
                .map(EcoString::to_string);
            let nodes = ctx.build_table(entry.nodes);
            TagNode::group(Tag::Table.with_summary(summary), nodes)
        }
        StackEntryKind::TableCell(cell) => {
            let parent = gc.tags.stack.last_mut().expect("table");
            let StackEntryKind::Table(table_ctx) = &mut parent.kind else {
                unreachable!("expected table")
            };

            table_ctx.insert(cell, entry.nodes);

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

pub struct Tags {
    /// The intermediary stack of nested tag groups.
    pub stack: Vec<StackEntry>,
    /// A list of placeholders corresponding to a [`TagNode::Placeholder`].
    pub placeholders: Vec<OnceCell<Node>>,
    pub in_artifact: Option<(Location, ArtifactKind)>,
    /// Used to group multiple link annotations using quad points.
    pub link_id: LinkId,

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
}

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
    Link(LinkId, Packed<LinkMarker>),
}

impl StackEntryKind {
    pub fn as_outline_mut(&mut self) -> Option<&mut OutlineCtx> {
        if let Self::Outline(v) = self { Some(v) } else { None }
    }

    pub fn as_outline_entry_mut(&mut self) -> Option<&mut OutlineEntry> {
        if let Self::OutlineEntry(v) = self { Some(v) } else { None }
    }

    pub fn as_link(&self) -> Option<(LinkId, &Packed<LinkMarker>)> {
        if let Self::Link(id, link) = self { Some((*id, link)) } else { None }
    }
}

#[derive(Clone, Debug)]
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

#[derive(Clone, Copy, Debug)]
pub struct Placeholder(usize);

/// Automatically calls [`Surface::end_tagged`] when dropped.
pub struct TagHandle<'a, 'b> {
    surface: &'b mut Surface<'a>,
}

impl Drop for TagHandle<'_, '_> {
    fn drop(&mut self) {
        self.surface.end_tagged();
    }
}

impl<'a> TagHandle<'a, '_> {
    pub fn surface<'c>(&'c mut self) -> &'c mut Surface<'a> {
        self.surface
    }
}

/// Returns a [`TagHandle`] that automatically calls [`Surface::end_tagged`]
/// when dropped.
pub fn start_marked<'a, 'b>(
    gc: &mut GlobalContext,
    surface: &'b mut Surface<'a>,
) -> TagHandle<'a, 'b> {
    start_content(gc, surface, ContentTag::Other)
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

fn start_content<'a, 'b>(
    gc: &mut GlobalContext,
    surface: &'b mut Surface<'a>,
    content: ContentTag,
) -> TagHandle<'a, 'b> {
    let content = if let Some((_, kind)) = gc.tags.in_artifact {
        let ty = artifact_type(kind);
        ContentTag::Artifact(ty)
    } else if let Some(StackEntryKind::Table(_)) = gc.tags.stack.last().map(|e| &e.kind) {
        // Mark any direct child of a table as an aritfact. Any real content
        // will be wrapped inside a `TableCell`.
        ContentTag::Artifact(ArtifactType::Other)
    } else {
        content
    };
    let id = surface.start_tagged(content);
    gc.tags.push(TagNode::Leaf(id));
    TagHandle { surface }
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

fn start_artifact(gc: &mut GlobalContext, loc: Location, kind: ArtifactKind) {
    gc.tags.in_artifact = Some((loc, kind));
}

fn artifact_type(kind: ArtifactKind) -> ArtifactType {
    match kind {
        ArtifactKind::Header => ArtifactType::Header,
        ArtifactKind::Footer => ArtifactType::Footer,
        ArtifactKind::Page => ArtifactType::Page,
        ArtifactKind::Other => ArtifactType::Other,
    }
}
