use std::cell::OnceCell;
use std::num::NonZeroU16;
use std::ops::{Deref, DerefMut};

use krilla::page::Page;
use krilla::surface::Surface;
use krilla::tagging as kt;
use krilla::tagging::{
    ArtifactType, ContentTag, Identifier, ListNumbering, Node, SpanTag, Tag, TagGroup,
    TagKind, TagTree,
};
use rustc_hash::FxHashMap;
use typst_library::foundations::{Content, Packed};
use typst_library::introspection::Location;
use typst_library::layout::RepeatElem;
use typst_library::math::EquationElem;
use typst_library::model::{
    EnumElem, FigureCaption, FigureElem, FootnoteEntry, HeadingElem, LinkMarker,
    ListElem, Outlinable, OutlineEntry, QuoteElem, TableCell, TableElem, TermsElem,
};
use typst_library::pdf::{ArtifactElem, ArtifactKind, PdfMarkerTag, PdfMarkerTagKind};
use typst_library::text::{RawElem, RawLine};
use typst_library::visualize::ImageElem;

use crate::convert::GlobalContext;
use crate::link::LinkAnnotation;
use crate::tags::list::ListCtx;
use crate::tags::outline::OutlineCtx;
use crate::tags::table::TableCtx;
use crate::tags::util::{PropertyOptRef, PropertyValCopied};

mod list;
mod outline;
mod table;
mod util;

pub fn handle_start(gc: &mut GlobalContext, surface: &mut Surface, elem: &Content) {
    if gc.tags.in_artifact.is_some() {
        // Don't nest artifacts
        return;
    }

    let loc = elem.location().expect("elem to be locatable");

    if let Some(artifact) = elem.to_packed::<ArtifactElem>() {
        let kind = artifact.kind.val();
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
            PdfMarkerTagKind::FootnoteRef(decl_loc) => {
                push_stack(gc, loc, StackEntryKind::FootnoteRef(*decl_loc));
                return;
            }
            PdfMarkerTagKind::Bibliography(numbered) => {
                let numbering =
                    if *numbered { ListNumbering::Decimal } else { ListNumbering::None };
                push_stack(gc, loc, StackEntryKind::List(ListCtx::new(numbering)));
                return;
            }
            PdfMarkerTagKind::BibEntry => {
                push_stack(gc, loc, StackEntryKind::BibEntry);
                return;
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
        let alt = image.alt.opt_ref().map(|s| s.to_string());

        if let Some(StackEntryKind::Standard(TagKind::Figure(tag))) =
            gc.tags.stack.parent()
        {
            // Set alt text of outer figure tag, if not present.
            if tag.alt_text().is_none() {
                tag.set_alt_text(alt);
            }
            return;
        } else {
            Tag::Figure(alt).into()
        }
    } else if let Some(equation) = elem.to_packed::<EquationElem>() {
        let alt = equation.alt.opt_ref().map(|s| s.to_string());
        if let Some(StackEntryKind::Standard(TagKind::Figure(tag))) =
            gc.tags.stack.parent()
        {
            // Set alt text of outer figure tag, if not present.
            if tag.alt_text().is_none() {
                tag.set_alt_text(alt.clone());
            }
        }
        Tag::Formula(alt).into()
    } else if let Some(table) = elem.to_packed::<TableElem>() {
        let table_id = gc.tags.next_table_id();
        let summary = table.summary.opt_ref().map(|s| s.to_string());
        let ctx = TableCtx::new(table_id, summary);
        push_stack(gc, loc, StackEntryKind::Table(ctx));
        return;
    } else if let Some(cell) = elem.to_packed::<TableCell>() {
        let table_ctx = gc.tags.stack.parent_table();

        // Only repeated table headers and footer cells are laid out multiple
        // times. Mark duplicate headers as artifacts, since they have no
        // semantic meaning in the tag tree, which doesn't use page breaks for
        // it's semantic structure.
        if cell.is_repeated.val() || table_ctx.is_some_and(|ctx| ctx.contains(cell)) {
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
    } else if let Some(entry) = elem.to_packed::<FootnoteEntry>() {
        let footnote_loc = entry.note.location().unwrap();
        push_stack(gc, loc, StackEntryKind::FootnoteEntry(footnote_loc));
        return;
    } else if let Some(quote) = elem.to_packed::<QuoteElem>() {
        // TODO: should the attribution be handled somehow?
        if quote.block.val() { Tag::BlockQuote.into() } else { Tag::InlineQuote.into() }
    } else if let Some(raw) = elem.to_packed::<RawElem>() {
        if raw.block.val() {
            push_stack(gc, loc, StackEntryKind::CodeBlock);
            return;
        } else {
            Tag::Code.into()
        }
    } else if let Some(_) = elem.to_packed::<RawLine>() {
        // If the raw element is inline, the content can be inserted directly.
        if gc.tags.stack.parent().is_some_and(|p| p.is_code_block()) {
            push_stack(gc, loc, StackEntryKind::CodeBlockLine);
        }
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
            let Some((outline_ctx, outline_nodes)) = gc.tags.stack.parent_outline()
            else {
                // PDF/UA compliance of the structure hierarchy is checked
                // elsewhere. While this doesn't make a lot of sense, just
                // avoid crashing here.
                gc.tags.push(TagNode::group(Tag::TOCI, entry.nodes));
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
                gc.tags.push(TagNode::group(Tag::TD, entry.nodes));
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
        StackEntryKind::Link(_, _) => {
            let mut node = TagNode::group(Tag::Link, entry.nodes);
            // Wrap link in reference tag if inside an outline entry.
            if gc.tags.stack.parent_outline_entry().is_some() {
                node = TagNode::group(Tag::Reference, vec![node]);
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
            let tag = TagNode::group(Tag::Note, entry.nodes);
            let ctx = gc.tags.footnotes.entry(footnote_loc).or_insert(FootnoteCtx::new());
            ctx.entry = Some(tag);
            return;
        }
        StackEntryKind::CodeBlock => TagNode::group(
            Tag::Code.with_placement(Some(kt::Placement::Block)),
            entry.nodes,
        ),
        StackEntryKind::CodeBlockLine => {
            // If the raw element is a block, wrap each line in a BLSE, so the
            // individual lines are properly wrapped and indented when reflowed.
            TagNode::group(Tag::P, entry.nodes)
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
        gc.tags.placeholders.init(a.placeholder, Node::Leaf(annot_id));
    }
}

pub struct Tags {
    /// The intermediary stack of nested tag groups.
    pub stack: TagStack,
    /// A list of placeholders corresponding to a [`TagNode::Placeholder`].
    pub placeholders: Placeholders,
    /// Footnotes are inserted directly after the footenote reference in the
    /// reading order. Because of some layouting bugs, the entry might appear
    /// before the reference in the text, so we only resolve them once tags
    /// for the whole document are generated.
    pub footnotes: FxHashMap<Location, FootnoteCtx>,
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
            stack: TagStack(Vec::new()),
            placeholders: Placeholders(Vec::new()),
            footnotes: FxHashMap::default(),
            in_artifact: None,

            link_id: LinkId(0),
            table_id: TableId(0),

            tree: Vec::new(),
        }
    }

    pub fn push(&mut self, node: TagNode) {
        if let Some(entry) = self.stack.last_mut() {
            entry.nodes.push(node);
        } else {
            self.tree.push(node);
        }
    }

    pub fn extend(&mut self, nodes: impl IntoIterator<Item = TagNode>) {
        if let Some(entry) = self.stack.last_mut() {
            entry.nodes.extend(nodes);
        } else {
            self.tree.extend(nodes);
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
            TagNode::Placeholder(placeholder) => self.placeholders.take(placeholder),
            TagNode::FootnoteEntry(loc) => {
                let node = (self.footnotes.remove(&loc))
                    .and_then(|ctx| ctx.entry)
                    .expect("footnote");
                self.resolve_node(node)
            }
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

pub struct TagStack(Vec<StackEntry>);

impl Deref for TagStack {
    type Target = Vec<StackEntry>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TagStack {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl TagStack {
    pub fn parent(&mut self) -> Option<&mut StackEntryKind> {
        self.0.last_mut().map(|e| &mut e.kind)
    }

    pub fn parent_table(&mut self) -> Option<&mut TableCtx> {
        self.parent()?.as_table_mut()
    }

    pub fn parent_list(&mut self) -> Option<&mut ListCtx> {
        self.parent()?.as_list_mut()
    }

    pub fn parent_outline(&mut self) -> Option<(&mut OutlineCtx, &mut Vec<TagNode>)> {
        self.0.last_mut().and_then(|e| {
            let ctx = e.kind.as_outline_mut()?;
            Some((ctx, &mut e.nodes))
        })
    }

    pub fn parent_outline_entry(&mut self) -> Option<&mut OutlineEntry> {
        self.parent()?.as_outline_entry_mut()
    }

    pub fn find_parent_link(
        &mut self,
    ) -> Option<(LinkId, &LinkMarker, &mut Vec<TagNode>)> {
        self.0.iter_mut().rev().find_map(|e| {
            let (link_id, link) = e.kind.as_link()?;
            Some((link_id, link.as_ref(), &mut e.nodes))
        })
    }
}

pub struct Placeholders(Vec<OnceCell<Node>>);

impl Placeholders {
    pub fn reserve(&mut self) -> Placeholder {
        let idx = self.0.len();
        self.0.push(OnceCell::new());
        Placeholder(idx)
    }

    pub fn init(&mut self, placeholder: Placeholder, node: Node) {
        self.0[placeholder.0]
            .set(node)
            .map_err(|_| ())
            .expect("placeholder to be uninitialized");
    }

    pub fn take(&mut self, placeholder: Placeholder) -> Node {
        self.0[placeholder.0].take().expect("initialized placeholder node")
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
    BibEntry,
    Link(LinkId, Packed<LinkMarker>),
    /// The footnote reference in the text, contains the declaration location.
    FootnoteRef(Location),
    /// The footnote entry at the end of the page. Contains the [`Location`] of
    /// the [`FootnoteElem`](typst_library::model::FootnoteElem).
    FootnoteEntry(Location),
    CodeBlock,
    CodeBlockLine,
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

    pub fn is_code_block(&self) -> bool {
        matches!(self, Self::CodeBlock)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FootnoteCtx {
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

#[derive(Debug, Clone, PartialEq)]
pub enum TagNode {
    Group(TagKind, Vec<TagNode>),
    Leaf(Identifier),
    /// Allows inserting a placeholder into the tag tree.
    /// Currently used for [`krilla::page::Page::add_tagged_annotation`].
    Placeholder(Placeholder),
    FootnoteEntry(Location),
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
