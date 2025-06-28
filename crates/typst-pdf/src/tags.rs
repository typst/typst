use std::cell::OnceCell;
use std::num::{NonZeroU32, NonZeroUsize};

use ecow::EcoString;
use krilla::page::Page;
use krilla::surface::Surface;
use krilla::tagging::{
    ArtifactType, ContentTag, Identifier, Node, SpanTag, TableCellSpan, TableDataCell,
    TableHeaderCell, Tag, TagBuilder, TagGroup, TagKind, TagTree,
};
use typst_library::foundations::{Content, LinkMarker, Packed, Smart, StyleChain};
use typst_library::introspection::Location;
use typst_library::layout::RepeatElem;
use typst_library::model::{
    Destination, FigureCaption, FigureElem, HeadingElem, Outlinable, OutlineBody,
    OutlineEntry, TableCell, TableCellKind, TableElem, TableHeaderScope,
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
    pub(crate) kind: StackEntryKind,
    pub(crate) nodes: Vec<TagNode>,
}

pub(crate) enum StackEntryKind {
    Standard(Tag),
    Outline(OutlineCtx),
    OutlineEntry(Packed<OutlineEntry>),
    Table(TableCtx),
    TableCell(Packed<TableCell>),
    Link(LinkId, Packed<LinkMarker>),
}

impl StackEntryKind {
    pub(crate) fn as_standard_mut(&mut self) -> Option<&mut Tag> {
        if let Self::Standard(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

pub(crate) struct OutlineCtx {
    stack: Vec<OutlineSection>,
}

pub(crate) struct OutlineSection {
    entries: Vec<TagNode>,
}

impl OutlineSection {
    const fn new() -> Self {
        OutlineSection { entries: Vec::new() }
    }

    fn push(&mut self, entry: TagNode) {
        self.entries.push(entry);
    }

    fn into_tag(self) -> TagNode {
        TagNode::Group(TagKind::TOC.into(), self.entries)
    }
}

impl OutlineCtx {
    fn new() -> Self {
        Self { stack: Vec::new() }
    }

    fn insert(
        &mut self,
        outline_nodes: &mut Vec<TagNode>,
        entry: Packed<OutlineEntry>,
        nodes: Vec<TagNode>,
    ) {
        let expected_len = entry.level.get() - 1;
        if self.stack.len() < expected_len {
            self.stack.resize_with(expected_len, || OutlineSection::new());
        } else {
            while self.stack.len() > expected_len {
                self.finish_section(outline_nodes);
            }
        }

        let section_entry = TagNode::Group(TagKind::TOCI.into(), nodes);
        self.push(outline_nodes, section_entry);
    }

    fn finish_section(&mut self, outline_nodes: &mut Vec<TagNode>) {
        let sub_section = self.stack.pop().unwrap().into_tag();
        self.push(outline_nodes, sub_section);
    }

    fn push(&mut self, outline_nodes: &mut Vec<TagNode>, entry: TagNode) {
        match self.stack.last_mut() {
            Some(section) => section.push(entry),
            None => outline_nodes.push(entry),
        }
    }

    fn build_outline(mut self, mut outline_nodes: Vec<TagNode>) -> Vec<TagNode> {
        while self.stack.len() > 0 {
            self.finish_section(&mut outline_nodes);
        }
        outline_nodes
    }
}

pub(crate) struct TableCtx {
    table: Packed<TableElem>,
    rows: Vec<Vec<GridCell>>,
}

#[derive(Clone, Default)]
enum GridCell {
    Cell(TableCtxCell),
    Spanned(usize, usize),
    #[default]
    Missing,
}

impl GridCell {
    fn as_cell(&self) -> Option<&TableCtxCell> {
        if let Self::Cell(v) = self {
            Some(v)
        } else {
            None
        }
    }

    fn into_cell(self) -> Option<TableCtxCell> {
        if let Self::Cell(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

#[derive(Clone)]
struct TableCtxCell {
    rowspan: NonZeroUsize,
    colspan: NonZeroUsize,
    kind: TableCellKind,
    header_scope: Smart<TableHeaderScope>,
    nodes: Vec<TagNode>,
}

impl TableCtx {
    fn new(table: Packed<TableElem>) -> Self {
        Self { table: table.clone(), rows: Vec::new() }
    }

    fn insert(&mut self, cell: Packed<TableCell>, nodes: Vec<TagNode>) {
        let x = cell.x(StyleChain::default()).unwrap_or_else(|| unreachable!());
        let y = cell.y(StyleChain::default()).unwrap_or_else(|| unreachable!());
        let rowspan = cell.rowspan(StyleChain::default());
        let colspan = cell.colspan(StyleChain::default());
        let kind = cell.kind(StyleChain::default());
        let header_scope = cell.header_scope(StyleChain::default());

        // The explicit cell kind takes precedence, but if it is `auto` and a
        // scope was specified, make this a header cell.
        let kind = match (kind, header_scope) {
            (Smart::Custom(kind), _) => kind,
            (Smart::Auto, Smart::Custom(_)) => TableCellKind::Header,
            (Smart::Auto, Smart::Auto) => TableCellKind::Data,
        };

        // Extend the table grid to fit this cell.
        let required_height = y + rowspan.get();
        let required_width = x + colspan.get();
        if self.rows.len() < required_height {
            self.rows
                .resize(required_height, vec![GridCell::Missing; required_width]);
        }
        let row = &mut self.rows[y];
        if row.len() < required_width {
            row.resize_with(required_width, || GridCell::Missing);
        }

        // Store references to the cell for all spanned cells.
        for i in y..y + rowspan.get() {
            for j in x..x + colspan.get() {
                self.rows[i][j] = GridCell::Spanned(x, y);
            }
        }

        self.rows[y][x] =
            GridCell::Cell(TableCtxCell { rowspan, colspan, kind, header_scope, nodes });
    }

    fn build_table(self, mut nodes: Vec<TagNode>) -> Vec<TagNode> {
        // Table layouting ensures that there are no overlapping cells, and that
        // any gaps left by the user are filled with empty cells.

        // Only generate row groups such as `THead`, `TFoot`, and `TBody` if
        // there are no rows with mixed cell kinds.
        let mut mixed_row_kinds = false;
        let row_kinds = (self.rows.iter())
            .map(|row| {
                row.iter()
                    .filter_map(|cell| match cell {
                        GridCell::Cell(cell) => Some(cell),
                        &GridCell::Spanned(x, y) => self.rows[y][x].as_cell(),
                        GridCell::Missing => None,
                    })
                    .map(|cell| cell.kind)
                    .reduce(|a, b| {
                        if a != b {
                            mixed_row_kinds = true;
                        }
                        a
                    })
                    .expect("tables must have at least one column")
            })
            .collect::<Vec<_>>();

        let Some(mut chunk_kind) = row_kinds.first().copied() else {
            return nodes;
        };
        let mut row_chunk = Vec::new();
        for (row, row_kind) in self.rows.into_iter().zip(row_kinds) {
            let row_nodes = row
                .into_iter()
                .filter_map(|cell| {
                    let cell = cell.into_cell()?;
                    let span = TableCellSpan {
                        rows: cell.rowspan.get() as i32,
                        cols: cell.colspan.get() as i32,
                    };
                    let tag = match cell.kind {
                        TableCellKind::Header => {
                            let scope = match cell.header_scope {
                                Smart::Custom(scope) => table_header_scope(scope),
                                Smart::Auto => krilla::tagging::TableHeaderScope::Column,
                            };
                            TagKind::TH(TableHeaderCell::new(scope).with_span(span))
                        }
                        TableCellKind::Footer | TableCellKind::Data => {
                            TagKind::TD(TableDataCell::new().with_span(span))
                        }
                    };

                    Some(TagNode::Group(tag.into(), cell.nodes))
                })
                .collect();

            let row = TagNode::Group(TagKind::TR.into(), row_nodes);

            // Push the `TR` tags directly.
            if mixed_row_kinds {
                nodes.push(row);
                continue;
            }

            // Generate row groups.
            if row_kind != chunk_kind {
                let tag = match chunk_kind {
                    TableCellKind::Header => TagKind::THead,
                    TableCellKind::Footer => TagKind::TFoot,
                    TableCellKind::Data => TagKind::TBody,
                };
                nodes.push(TagNode::Group(tag.into(), std::mem::take(&mut row_chunk)));

                chunk_kind = row_kind;
            }
            row_chunk.push(row);
        }

        if !row_chunk.is_empty() {
            let tag = match chunk_kind {
                TableCellKind::Header => TagKind::THead,
                TableCellKind::Footer => TagKind::TFoot,
                TableCellKind::Data => TagKind::TBody,
            };
            nodes.push(TagNode::Group(tag.into(), row_chunk));
        }

        nodes
    }
}

#[derive(Clone)]
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
    pub(crate) fn parent(&mut self) -> Option<&mut StackEntryKind> {
        self.stack.last_mut().map(|e| &mut e.kind)
    }

    pub(crate) fn push(&mut self, node: TagNode) {
        if let Some(entry) = self.stack.last_mut() {
            entry.nodes.push(node);
        } else {
            self.tree.push(node);
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
            TagNode::Placeholder(placeholder) => self.take_placeholder(placeholder),
        }
    }

    fn context_supports(&self, _tag: &StackEntryKind) -> bool {
        // TODO: generate using: https://pdfa.org/resource/iso-ts-32005-hierarchical-inclusion-rules/
        true
    }

    fn next_link_id(&mut self) -> LinkId {
        self.link_id.0 += 1;
        self.link_id
    }
}

/// Automatically calls [`Surface::end_tagged`] when dropped.
pub(crate) struct TagHandle<'a, 'b> {
    surface: &'b mut Surface<'a>,
}

impl Drop for TagHandle<'_, '_> {
    fn drop(&mut self) {
        self.surface.end_tagged();
    }
}

impl<'a> TagHandle<'a, '_> {
    pub(crate) fn surface<'c>(&'c mut self) -> &'c mut Surface<'a> {
        &mut self.surface
    }
}

/// Returns a [`TagHandle`] that automatically calls [`Surface::end_tagged`]
/// when dropped.
pub(crate) fn start_marked<'a, 'b>(
    gc: &mut GlobalContext,
    surface: &'b mut Surface<'a>,
) -> TagHandle<'a, 'b> {
    start_content(gc, surface, ContentTag::Other)
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
pub(crate) fn add_annotations(
    gc: &mut GlobalContext,
    page: &mut Page,
    annotations: Vec<LinkAnnotation>,
) {
    for annotation in annotations.into_iter() {
        let LinkAnnotation { id: _, placeholder, alt, rect, quad_points, target } =
            annotation;
        let annot = krilla::annotation::Annotation::new_link(
            krilla::annotation::LinkAnnotation::new(rect, Some(quad_points), target),
            alt,
        );
        let annot_id = page.add_tagged_annotation(annot);
        gc.tags.init_placeholder(placeholder, Node::Leaf(annot_id));
    }
}

pub(crate) fn handle_start(gc: &mut GlobalContext, elem: &Content) {
    if gc.tags.in_artifact.is_some() {
        // Don't nest artifacts
        return;
    }

    let loc = elem.location().unwrap();

    if let Some(artifact) = elem.to_packed::<ArtifactElem>() {
        let kind = artifact.kind(StyleChain::default());
        start_artifact(gc, loc, kind);
        return;
    } else if let Some(_) = elem.to_packed::<RepeatElem>() {
        start_artifact(gc, loc, ArtifactKind::Other);
        return;
    }

    let tag: Tag = if let Some(pdf_tag) = elem.to_packed::<PdfTagElem>() {
        let kind = pdf_tag.kind(StyleChain::default());
        match kind {
            PdfTagKind::Part => TagKind::Part.into(),
            _ => todo!(),
        }
    } else if let Some(heading) = elem.to_packed::<HeadingElem>() {
        let level = heading.level().try_into().unwrap_or(NonZeroU32::MAX);
        let name = heading.body.plain_text().to_string();
        TagKind::Hn(level, Some(name)).into()
    } else if let Some(_) = elem.to_packed::<OutlineBody>() {
        push_stack(gc, loc, StackEntryKind::Outline(OutlineCtx::new()));
        return;
    } else if let Some(entry) = elem.to_packed::<OutlineEntry>() {
        push_stack(gc, loc, StackEntryKind::OutlineEntry(entry.clone()));
        return;
    } else if let Some(_) = elem.to_packed::<FigureElem>() {
        let alt = None; // TODO
        TagKind::Figure.with_alt_text(alt)
    } else if let Some(image) = elem.to_packed::<ImageElem>() {
        let alt = image.alt(StyleChain::default()).map(|s| s.to_string());

        let figure_tag = (gc.tags.parent())
            .and_then(StackEntryKind::as_standard_mut)
            .filter(|tag| tag.kind == TagKind::Figure);
        if let Some(figure_tag) = figure_tag {
            // Set alt text of outer figure tag, if not present.
            if figure_tag.alt_text.is_none() {
                figure_tag.alt_text = alt;
            }
            return;
        } else {
            TagKind::Figure.with_alt_text(alt)
        }
    } else if let Some(_) = elem.to_packed::<FigureCaption>() {
        TagKind::Caption.into()
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
    if !gc.tags.context_supports(&kind) {
        // TODO: error or warning?
    }

    gc.tags.stack.push(StackEntry { loc, kind, nodes: Vec::new() });
}

pub(crate) fn handle_end(gc: &mut GlobalContext, loc: Location) {
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
        StackEntryKind::Standard(tag) => TagNode::Group(tag, entry.nodes),
        StackEntryKind::Outline(ctx) => {
            let nodes = ctx.build_outline(entry.nodes);
            TagNode::Group(TagKind::TOC.into(), nodes)
        }
        StackEntryKind::OutlineEntry(outline_entry) => {
            let parent = gc.tags.stack.last_mut().expect("outline");
            let StackEntryKind::Outline(outline_ctx) = &mut parent.kind else {
                unreachable!("expected outline")
            };

            outline_ctx.insert(&mut parent.nodes, outline_entry, entry.nodes);

            return;
        }
        StackEntryKind::Table(ctx) => {
            let summary = ctx.table.summary(StyleChain::default()).map(EcoString::into);
            let nodes = ctx.build_table(entry.nodes);
            TagNode::Group(TagKind::Table(summary).into(), nodes)
        }
        StackEntryKind::TableCell(cell) => {
            let parent = gc.tags.stack.last_mut().expect("table");
            let StackEntryKind::Table(table_ctx) = &mut parent.kind else {
                unreachable!("expected table")
            };

            table_ctx.insert(cell, entry.nodes);

            return;
        }
        StackEntryKind::Link(_, link) => {
            let alt = link.alt.as_ref().map(EcoString::to_string);
            let tag = TagKind::Link.with_alt_text(alt);
            let mut node = TagNode::Group(tag, entry.nodes);
            // Wrap link in reference tag, if it's not a url.
            if let Destination::Position(_) | Destination::Location(_) = link.dest {
                node = TagNode::Group(TagKind::Reference.into(), vec![node]);
            }
            node
        }
    };

    gc.tags.push(node);
}

fn start_artifact(gc: &mut GlobalContext, loc: Location, kind: ArtifactKind) {
    gc.tags.in_artifact = Some((loc, kind));
}

fn table_header_scope(scope: TableHeaderScope) -> krilla::tagging::TableHeaderScope {
    match scope {
        TableHeaderScope::Both => krilla::tagging::TableHeaderScope::Both,
        TableHeaderScope::Column => krilla::tagging::TableHeaderScope::Column,
        TableHeaderScope::Row => krilla::tagging::TableHeaderScope::Row,
    }
}

fn artifact_type(kind: ArtifactKind) -> ArtifactType {
    match kind {
        ArtifactKind::Header => ArtifactType::Header,
        ArtifactKind::Footer => ArtifactType::Footer,
        ArtifactKind::Page => ArtifactType::Page,
        ArtifactKind::Other => ArtifactType::Other,
    }
}
