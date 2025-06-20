use std::cell::OnceCell;
use std::collections::HashMap;

use krilla::page::Page;
use krilla::surface::Surface;
use krilla::tagging::{
    ArtifactType, ContentTag, Identifier, Node, Tag, TagGroup, TagTree,
};
use typst_library::foundations::{Content, LinkMarker, StyleChain};
use typst_library::introspection::Location;
use typst_library::model::{
    Destination, FigureCaption, FigureElem, HeadingElem, Outlinable, OutlineElem,
    OutlineEntry,
};
use typst_library::pdf::{ArtifactElem, ArtifactKind, PdfTagElem, PdfTagKind};
use typst_library::visualize::ImageElem;

use crate::convert::GlobalContext;
use crate::link::LinkAnnotation;

pub(crate) struct Tags {
    /// The intermediary stack of nested tag groups.
    pub(crate) stack: Vec<StackEntry>,
    /// A list of placeholders corresponding to a [`TagNode::Placeholder`].
    pub(crate) placeholders: Vec<OnceCell<Node>>,
    pub(crate) in_artifact: Option<(Location, ArtifactKind)>,
    pub(crate) link_id: LinkId,

    /// The output.
    pub(crate) tree: Vec<TagNode>,
}

pub(crate) struct StackEntry {
    pub(crate) loc: Location,
    pub(crate) link_id: Option<LinkId>,
    /// A list of tags that are wrapped around this tag when it is inserted into
    /// the tag tree.
    pub(crate) wrappers: Vec<Tag>,
    pub(crate) tag: Tag,
    pub(crate) nodes: Vec<TagNode>,
}

pub(crate) enum TagNode {
    Group(Tag, Vec<TagNode>),
    Leaf(Identifier),
    /// Allows inserting a placeholder into the tag tree.
    /// Currently used for [`krilla::page::Page::add_tagged_annotation`].
    Placeholder(Placeholder),
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct LinkId(u32);

#[derive(Clone, Copy)]
pub(crate) struct Placeholder(usize);

impl Tags {
    pub(crate) fn new() -> Self {
        Self {
            stack: Vec::new(),
            placeholders: Vec::new(),
            in_artifact: None,

            tree: Vec::new(),
            link_id: LinkId(0),
        }
    }

    pub(crate) fn reserve_placeholder(&mut self) -> Placeholder {
        let idx = self.placeholders.len();
        self.placeholders.push(OnceCell::new());
        Placeholder(idx)
    }

    pub(crate) fn init_placeholder(&mut self, placeholder: Placeholder, node: Node) {
        self.placeholders[placeholder.0]
            .set(node)
            .map_err(|_| ())
            .expect("placeholder to be uninitialized");
    }

    pub(crate) fn take_placeholder(&mut self, placeholder: Placeholder) -> Node {
        self.placeholders[placeholder.0]
            .take()
            .expect("initialized placeholder node")
    }

    /// Returns the current parent's list of children and the structure type ([Tag]).
    /// In case of the document root the structure type will be `None`.
    pub(crate) fn parent(&mut self) -> (Option<&mut Tag>, &mut Vec<TagNode>) {
        if let Some(entry) = self.stack.last_mut() {
            (Some(&mut entry.tag), &mut entry.nodes)
        } else {
            (None, &mut self.tree)
        }
    }

    pub(crate) fn push(&mut self, node: TagNode) {
        self.parent().1.push(node);
    }

    pub(crate) fn build_tree(&mut self) -> TagTree {
        let mut tree = TagTree::new();
        let nodes = std::mem::take(&mut self.tree);
        // PERF: collect into vec and construct TagTree directly from tag nodes.
        for node in nodes.into_iter().map(|node| self.resolve_node(node)) {
            tree.push(node);
        }
        tree
    }

    /// Resolves [`Placeholder`] nodes.
    fn resolve_node(&mut self, node: TagNode) -> Node {
        match node {
            TagNode::Group(tag, nodes) => {
                let mut group = TagGroup::new(tag);
                // PERF: collect into vec and construct TagTree directly from tag nodes.
                for node in nodes.into_iter().map(|node| self.resolve_node(node)) {
                    group.push(node);
                }
                Node::Group(group)
            }
            TagNode::Leaf(identifier) => Node::Leaf(identifier),
            TagNode::Placeholder(placeholder) => self.take_placeholder(placeholder),
        }
    }

    fn context_supports(&self, _tag: &Tag) -> bool {
        // TODO: generate using: https://pdfa.org/resource/iso-ts-32005-hierarchical-inclusion-rules/
        true
    }

    fn next_link_id(&mut self) -> LinkId {
        self.link_id.0 += 1;
        self.link_id
    }
}

/// Marked-content may not cross page boundaries: restart tag that was still open
/// at the end of the last page.
pub(crate) fn restart_open(gc: &mut GlobalContext, surface: &mut Surface) {
    // TODO: somehow avoid empty marked-content sequences
    if let Some((_, kind)) = gc.tags.in_artifact {
        start_artifact(gc, surface, kind);
    } else if let Some(entry) = gc.tags.stack.last_mut() {
        let id = surface.start_tagged(ContentTag::Other);
        entry.nodes.push(TagNode::Leaf(id));
    }
}

/// Marked-content may not cross page boundaries: end any open tag.
pub(crate) fn end_open(gc: &mut GlobalContext, surface: &mut Surface) {
    if !gc.tags.stack.is_empty() || gc.tags.in_artifact.is_some() {
        surface.end_tagged();
    }
}

/// Add all annotations that were found in the page frame.
pub(crate) fn add_annotations(
    gc: &mut GlobalContext,
    page: &mut Page,
    annotations: HashMap<LinkId, LinkAnnotation>,
) {
    for annotation in annotations.into_values() {
        let LinkAnnotation { placeholder, alt, rect, quad_points, target } = annotation;
        let annot = krilla::annotation::Annotation::new_link(
            krilla::annotation::LinkAnnotation::new(rect, Some(quad_points), target),
            alt,
        );
        let annot_id = page.add_tagged_annotation(annot);
        gc.tags.init_placeholder(placeholder, Node::Leaf(annot_id));
    }
}

pub(crate) fn handle_start(
    gc: &mut GlobalContext,
    surface: &mut Surface,
    elem: &Content,
) {
    if gc.tags.in_artifact.is_some() {
        // Don't nest artifacts
        return;
    }

    let loc = elem.location().unwrap();

    if let Some(artifact) = elem.to_packed::<ArtifactElem>() {
        if !gc.tags.stack.is_empty() {
            surface.end_tagged();
        }
        let kind = artifact.kind(StyleChain::default());
        start_artifact(gc, surface, kind);
        gc.tags.in_artifact = Some((loc, kind));
        return;
    }

    let mut link_id = None;
    let mut wrappers = Vec::new();
    let tag = if let Some(pdf_tag) = elem.to_packed::<PdfTagElem>() {
        let kind = pdf_tag.kind(StyleChain::default());
        match kind {
            PdfTagKind::Part => Tag::Part,
            _ => todo!(),
        }
    } else if let Some(heading) = elem.to_packed::<HeadingElem>() {
        let level = heading.level();
        let name = heading.body.plain_text().to_string();
        match level.get() {
            1 => Tag::H1(Some(name)),
            2 => Tag::H2(Some(name)),
            3 => Tag::H3(Some(name)),
            4 => Tag::H4(Some(name)),
            5 => Tag::H5(Some(name)),
            // TODO: when targeting PDF 2.0 headings `> 6` are supported
            _ => Tag::H6(Some(name)),
        }
    } else if let Some(_) = elem.to_packed::<OutlineElem>() {
        Tag::TOC
    } else if let Some(_) = elem.to_packed::<OutlineEntry>() {
        Tag::TOCI
    } else if let Some(_) = elem.to_packed::<FigureElem>() {
        let alt = None; // TODO
        Tag::Figure(alt)
    } else if let Some(image) = elem.to_packed::<ImageElem>() {
        let alt = image.alt(StyleChain::default()).map(|s| s.to_string());

        end_open(gc, surface);
        let id = surface.start_tagged(ContentTag::Other);
        let mut node = TagNode::Leaf(id);

        if let Some(Tag::Figure(alt_text)) = gc.tags.parent().0 {
            // HACK: set alt text of outer figure tag, if the contained image
            // has alt text specified
            if alt_text.is_none() {
                *alt_text = alt;
            }
        } else {
            node = TagNode::Group(Tag::Figure(alt), vec![node]);
        }
        gc.tags.push(node);

        return;
    } else if let Some(_) = elem.to_packed::<FigureCaption>() {
        Tag::Caption
    } else if let Some(link) = elem.to_packed::<LinkMarker>() {
        link_id = Some(gc.tags.next_link_id());
        if let Destination::Position(_) | Destination::Location(_) = link.dest {
            wrappers.push(Tag::Reference);
        }
        Tag::Link
    } else {
        return;
    };

    if !gc.tags.context_supports(&tag) {
        // TODO: error or warning?
    }

    // close previous marked-content and open a nested tag.
    end_open(gc, surface);
    let id = surface.start_tagged(krilla::tagging::ContentTag::Other);
    gc.tags.stack.push(StackEntry {
        loc,
        link_id,
        wrappers,
        tag,
        nodes: vec![TagNode::Leaf(id)],
    });
}

pub(crate) fn handle_end(gc: &mut GlobalContext, surface: &mut Surface, loc: Location) {
    if let Some((l, _)) = gc.tags.in_artifact {
        if l == loc {
            gc.tags.in_artifact = None;
            surface.end_tagged();
            if let Some(entry) = gc.tags.stack.last_mut() {
                let id = surface.start_tagged(ContentTag::Other);
                entry.nodes.push(TagNode::Leaf(id));
            }
        }
        return;
    }

    let Some(entry) = gc.tags.stack.pop_if(|e| e.loc == loc) else {
        return;
    };

    surface.end_tagged();

    let (parent_tag, parent_nodes) = gc.tags.parent();
    let mut node = TagNode::Group(entry.tag, entry.nodes);
    for tag in entry.wrappers {
        node = TagNode::Group(tag, vec![node]);
    }
    parent_nodes.push(node);
    if parent_tag.is_some() {
        // TODO: somehow avoid empty marked-content sequences
        let id = surface.start_tagged(ContentTag::Other);
        parent_nodes.push(TagNode::Leaf(id));
    }
}

fn start_artifact(gc: &mut GlobalContext, surface: &mut Surface, kind: ArtifactKind) {
    let ty = artifact_type(kind);
    let id = surface.start_tagged(ContentTag::Artifact(ty));

    gc.tags.push(TagNode::Leaf(id));
}

fn artifact_type(kind: ArtifactKind) -> ArtifactType {
    match kind {
        ArtifactKind::Header => ArtifactType::Header,
        ArtifactKind::Footer => ArtifactType::Footer,
        ArtifactKind::Page => ArtifactType::Page,
        ArtifactKind::Other => ArtifactType::Other,
    }
}
