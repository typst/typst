use std::num::NonZeroU16;

use krilla::surface::Surface;
use krilla::tagging::{
    ArtifactType, ContentTag, Identifier, Node, SpanTag, Tag, TagGroup, TagKind, TagTree,
};
use typst_library::foundations::{Content, StyleChain};
use typst_library::introspection::Location;
use typst_library::layout::RepeatElem;
use typst_library::model::{FigureCaption, FigureElem, HeadingElem, Outlinable};
use typst_library::pdf::{ArtifactElem, ArtifactKind};
use typst_library::visualize::ImageElem;

use crate::convert::GlobalContext;

pub struct Tags {
    /// The intermediary stack of nested tag groups.
    pub stack: Vec<StackEntry>,
    pub in_artifact: Option<(Location, ArtifactKind)>,

    /// The output.
    pub tree: Vec<TagNode>,
}

#[derive(Debug)]
pub struct StackEntry {
    pub loc: Location,
    pub kind: StackEntryKind,
    pub nodes: Vec<TagNode>,
}

#[derive(Debug)]
pub enum StackEntryKind {
    Standard(TagKind),
}

#[derive(Debug)]
pub enum TagNode {
    Group(TagKind, Vec<TagNode>),
    Leaf(Identifier),
}

impl TagNode {
    pub fn group(tag: impl Into<TagKind>, children: Vec<TagNode>) -> Self {
        TagNode::Group(tag.into(), children)
    }
}

impl Tags {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            in_artifact: None,

            tree: Vec::new(),
        }
    }

    /// Returns the current parent's list of children and the structure type ([Tag]).
    /// In case of the document root the structure type will be `None`.
    pub fn parent(&mut self) -> Option<&mut StackEntryKind> {
        self.stack.last_mut().map(|e| &mut e.kind)
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
        }
    }
}

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
    } else {
        content
    };
    let id = surface.start_tagged(content);
    gc.tags.push(TagNode::Leaf(id));
    TagHandle { surface }
}

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
    };

    gc.tags.push(node);
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
