use std::cell::OnceCell;
use std::collections::hash_map::Entry;
use std::slice::SliceIndex;

use krilla::geom as kg;
use krilla::tagging as kt;
use krilla::tagging::{BBox, Identifier, Node, TagKind, TagTree};
use rustc_hash::FxHashMap;
use typst_library::diag::{SourceResult, bail};
use typst_library::foundations::{Content, LinkMarker, Packed};
use typst_library::introspection::Location;
use typst_library::layout::{Abs, GridCell, Point, Rect};
use typst_library::model::{OutlineEntry, TableCell};
use typst_library::pdf::ArtifactKind;
use typst_library::text::Lang;
use typst_syntax::Span;

use crate::PdfOptions;
use crate::convert::FrameContext;
use crate::tags::grid::{GridCtx, TableCtx};
use crate::tags::list::ListCtx;
use crate::tags::outline::OutlineCtx;
use crate::tags::text::{ResolvedTextAttrs, TextAttrs};
use crate::util::AbsExt;

pub struct Tags {
    /// The set of text attributes.
    pub text_attrs: TextAttrs,
    /// The intermediary stack of nested tag groups.
    pub stack: TagStack,
    pub groups: Groups,
    /// A list of placeholders corresponding to a [`TagNode::Placeholder`].
    pub placeholders: Placeholders,
    pub disable: Option<Disable>,
    /// The document group which makes up the whole tag tree.
    doc: GroupId,
    /// Used to group multiple link annotations using quad points.
    link_id: LinkId,
    /// Used to generate IDs referenced in table `Headers` attributes.
    /// The IDs must be document wide unique.
    table_id: TableId,
}

impl Tags {
    pub fn new(doc_lang: Option<Lang>) -> Self {
        let mut groups = Groups::new();
        let doc = groups.root_document(doc_lang);
        Self {
            text_attrs: TextAttrs::new(),
            stack: TagStack::new(),
            groups,
            placeholders: Placeholders(Vec::new()),
            disable: None,
            doc,
            link_id: LinkId(0),
            table_id: TableId(0),
        }
    }

    /// Either the group of the last stack entry or the root document group.
    pub fn parent_group(&self) -> GroupId {
        self.stack.last().map(|e| e.id).unwrap_or(self.doc)
    }

    pub fn push(&mut self, node: TagNode) {
        self.groups.push_node(self.parent_group(), node);
    }

    pub fn push_text(&mut self, new_attrs: ResolvedTextAttrs, text_id: Identifier) {
        if new_attrs.is_empty() {
            self.push(TagNode::Leaf(text_id));
            return;
        }

        let last_node = self.groups.nodes_mut(self.parent_group()).last_mut();
        if let Some(TagNode::Text(prev_attrs, nodes)) = last_node
            && *prev_attrs == new_attrs
        {
            nodes.push(text_id);
        } else {
            self.push(TagNode::Text(new_attrs, vec![text_id]));
        }
    }

    pub fn finish(&mut self) -> (Option<Lang>, TagTree) {
        assert!(self.stack.items.is_empty(), "tags weren't properly closed");

        let doc = self.groups.take_group(self.doc);
        let GroupState::Root(mut doc_lang) = doc.state else { unreachable!() };

        let mut children = Vec::with_capacity(doc.nodes.len());

        for child in doc.nodes {
            resolve_node(
                &mut self.groups,
                &mut self.placeholders,
                &mut doc_lang,
                &mut children,
                child,
            );
        }

        (doc_lang, TagTree::from(children))
    }

    /// Try to set the language of the direct parent tag, or the entire document.
    /// If the language couldn't be set and is different from the existing one,
    /// this will return `Some`, and the language should be specified on the
    /// marked content directly.
    pub fn try_set_lang(&mut self, lang: Lang) -> Option<Lang> {
        self.groups.try_set_lang(self.parent_group(), lang)
    }

    pub fn next_link_id(&mut self) -> LinkId {
        self.link_id.0 += 1;
        self.link_id
    }

    pub fn next_table_id(&mut self) -> TableId {
        self.table_id.0 += 1;
        self.table_id
    }
}

/// Resolves nodes into an accumulator.
fn resolve_node(
    groups: &mut Groups,
    placeholders: &mut Placeholders,
    mut parent_lang: &mut Option<Lang>,
    accum: &mut Vec<Node>,
    node: TagNode,
) {
    match node {
        TagNode::Group(id) => {
            let mut group = groups.take_group(id);

            assert!(group.unfinished_stack.is_empty());

            let mut nodes = Vec::with_capacity(group.nodes.len());
            let lang = group.state.lang_mut().unwrap_or(&mut parent_lang);
            for child in group.nodes.into_iter() {
                resolve_node(groups, placeholders, lang, &mut nodes, child);
            }

            match group.state {
                GroupState::Root(_) => unreachable!(),
                GroupState::Tagged(tag, mut group_lang) => {
                    // Try to propagate the groups language to the parent tag.
                    if let Some(lang) = group_lang
                        && parent_lang.is_none_or(|l| l == lang)
                    {
                        *parent_lang = Some(lang);
                        group_lang = None;
                    }

                    let tag = tag
                        .expect("tag to be initialized")
                        .with_lang(group_lang.map(|l| l.as_str().to_string()))
                        .with_location(group.span.map(Span::into_raw));

                    let group = kt::TagGroup::with_children(tag, nodes);
                    accum.push(Node::Group(group));
                }
                GroupState::Transparent => {
                    accum.extend(nodes);
                }
            }
        }
        TagNode::Leaf(identifier) => {
            accum.push(Node::Leaf(identifier));
        }
        TagNode::Placeholder(placeholder) => {
            accum.push(placeholders.take(placeholder));
        }
        TagNode::Text(attrs, ids) => {
            attrs.resolve_nodes(accum, ids);
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Disable {
    /// Either an artifact or a hide element.
    Elem(Location, ArtifactKind),
    Tiling,
}

#[derive(Debug)]
pub struct TagStack {
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
    pub fn new() -> Self {
        Self { items: Vec::new(), bbox_idx: None }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn get(&self, idx: usize) -> Option<&StackEntry> {
        self.items.get(idx)
    }

    pub fn last(&self) -> Option<&StackEntry> {
        self.items.last()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, StackEntry> {
        self.items.iter()
    }

    pub fn push(&mut self, entry: StackEntry) {
        if entry.kind.bbox().is_some() {
            self.bbox_idx = Some(self.len());
        }
        self.items.push(entry);
    }

    pub fn extend(&mut self, iter: impl IntoIterator<Item = StackEntry>) {
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
    pub fn pop_if(
        &mut self,
        mut predicate: impl FnMut(&mut StackEntry) -> bool,
    ) -> Option<StackEntry> {
        let last = self.items.last_mut()?;
        if predicate(last) { self.pop() } else { None }
    }

    /// Remove the last stack entry.
    /// This takes care of updating the parent bboxes.
    pub fn pop(&mut self) -> Option<StackEntry> {
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

    /// Remove all stack entries after the idx.
    /// This takes care of updating the parent bboxes.
    pub fn take_unfinished_stack(
        &mut self,
        idx: usize,
    ) -> std::vec::Drain<'_, StackEntry> {
        if self.bbox_idx.is_some() {
            // The inner tags are broken across regions (pages), which invalidates all bounding boxes.
            for entry in self.items.iter_mut() {
                if let Some(bbox) = entry.kind.bbox_mut() {
                    bbox.multi_page = true;
                }
            }
            self.bbox_idx = self.items[..idx]
                .iter()
                .enumerate()
                .rev()
                .find(|(_, entry)| entry.kind.bbox().is_some())
                .map(|(idx, _)| idx);
        }
        self.items.drain(idx + 1..)
    }

    pub fn parent(&mut self) -> Option<&mut StackEntryKind> {
        self.items.last_mut().map(|e| &mut e.kind)
    }

    pub fn parent_table(&mut self) -> Option<&mut TableCtx> {
        self.parent()?.as_table_mut()
    }

    pub fn parent_grid(&mut self) -> Option<&mut GridCtx> {
        self.parent()?.as_grid_mut()
    }

    pub fn parent_list(&mut self) -> Option<&mut ListCtx> {
        self.parent()?.as_list_mut()
    }

    pub fn parent_figure(&mut self) -> Option<&mut FigureCtx> {
        self.parent()?.as_figure_mut()
    }

    pub fn parent_outline(&mut self) -> Option<(&mut OutlineCtx, GroupId)> {
        self.items.last_mut().and_then(|e| {
            let ctx = e.kind.as_outline_mut()?;
            Some((ctx, e.id))
        })
    }

    pub fn parent_outline_entry(&mut self) -> Option<&mut OutlineEntry> {
        self.parent()?.as_outline_entry_mut()
    }

    pub fn find_parent_link(&mut self) -> Option<(LinkId, &Packed<LinkMarker>, GroupId)> {
        self.items.iter_mut().rev().find_map(|e| {
            let (link_id, link) = e.kind.as_link()?;
            Some((link_id, link, e.id))
        })
    }

    /// Finds the first parent that has a bounding box.
    pub fn find_parent_bbox(&mut self) -> Option<&mut BBoxCtx> {
        self.items[self.bbox_idx?].kind.bbox_mut()
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Placeholder(usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TableId(u32);

impl TableId {
    pub fn get(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LinkId(u32);

#[derive(Debug)]
pub struct StackEntry {
    /// The location of the stack entry. If this is `None` the stack entry has
    /// to be manually popped.
    pub loc: Option<Location>,
    pub span: Span,
    pub id: GroupId,
    pub kind: StackEntryKind,
}

#[derive(Clone, Debug)]
pub enum StackEntryKind {
    Standard(TagKind),
    LogicalParent(Content),
    LogicalChild,
    Outline(OutlineCtx),
    OutlineEntry(Packed<OutlineEntry>),
    Table(TableCtx),
    TableCell(Packed<TableCell>),
    Grid(GridCtx),
    GridCell(Packed<GridCell>),
    List(ListCtx),
    ListItemLabel,
    ListItemBody,
    BibEntry,
    Figure(FigureCtx),
    Formula(FigureCtx),
    Link(LinkId, Packed<LinkMarker>, bool),
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

    pub fn as_grid_mut(&mut self) -> Option<&mut GridCtx> {
        if let Self::Grid(v) = self { Some(v) } else { None }
    }

    pub fn as_list_mut(&mut self) -> Option<&mut ListCtx> {
        if let Self::List(v) = self { Some(v) } else { None }
    }

    pub fn as_figure_mut(&mut self) -> Option<&mut FigureCtx> {
        if let Self::Figure(v) = self { Some(v) } else { None }
    }

    pub fn as_link(&self) -> Option<(LinkId, &Packed<LinkMarker>)> {
        if let Self::Link(id, link, _) = self { Some((*id, link)) } else { None }
    }

    pub fn is_code_block(&self) -> bool {
        matches!(self, Self::CodeBlock)
    }

    pub fn bbox(&self) -> Option<&BBoxCtx> {
        match self {
            Self::Table(ctx) => Some(&ctx.bbox),
            Self::Figure(ctx) => Some(&ctx.bbox),
            Self::Formula(ctx) => Some(&ctx.bbox),
            _ => None,
        }
    }

    pub fn bbox_mut(&mut self) -> Option<&mut BBoxCtx> {
        match self {
            Self::Table(ctx) => Some(&mut ctx.bbox),
            Self::Figure(ctx) => Some(&mut ctx.bbox),
            Self::Formula(ctx) => Some(&mut ctx.bbox),
            _ => None,
        }
    }

    pub fn is_breakable(&self, is_pdf_ua: bool) -> bool {
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
                TagKind::Span(_) => true,
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
                TagKind::Strong(_) => true,
                TagKind::Em(_) => true,
            },
            StackEntryKind::LogicalParent(_) => false,
            StackEntryKind::LogicalChild => false,
            StackEntryKind::Outline(_) => false,
            StackEntryKind::OutlineEntry(_) => false,
            StackEntryKind::Table(_) => false,
            StackEntryKind::TableCell(_) => false,
            StackEntryKind::Grid(_) => false,
            StackEntryKind::GridCell(_) => false,
            StackEntryKind::List(_) => false,
            StackEntryKind::ListItemLabel => false,
            StackEntryKind::ListItemBody => false,
            StackEntryKind::BibEntry => false,
            StackEntryKind::Figure(_) => false,
            StackEntryKind::Formula(_) => false,
            StackEntryKind::Link(..) => !is_pdf_ua,
            StackEntryKind::CodeBlock => false,
            StackEntryKind::CodeBlockLine => false,
        }
    }
}

/// Figure/Formula context
#[derive(Debug, Clone, PartialEq)]
pub struct FigureCtx {
    pub alt: Option<String>,
    pub bbox: BBoxCtx,
}

impl FigureCtx {
    pub fn new(alt: Option<String>) -> Self {
        Self { alt, bbox: BBoxCtx::new() }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BBoxCtx {
    pub rect: Option<(usize, Rect)>,
    pub multi_page: bool,
}

impl BBoxCtx {
    pub fn new() -> Self {
        Self { rect: None, multi_page: false }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Expand the bounding box with a `rect` relative to the current frame
    /// context transform.
    pub fn expand_frame(&mut self, fc: &FrameContext, rect: Rect) {
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
    pub fn expand_page(&mut self, page_idx: usize, rect: Rect) {
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

    pub fn to_krilla(&self) -> Option<BBox> {
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

#[derive(Debug)]
pub struct Groups {
    locations: FxHashMap<Location, GroupId>,
    list: Vec<Group>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GroupId(u32);

impl Groups {
    pub fn new() -> Self {
        Self { locations: FxHashMap::default(), list: Vec::new() }
    }

    fn _get(&self, id: GroupId) -> &Group {
        &self.list[id.0 as usize]
    }

    fn _get_mut(&mut self, id: GroupId) -> &mut Group {
        &mut self.list[id.0 as usize]
    }

    pub fn tag(&self, id: GroupId) -> Option<&TagKind> {
        self._get(id).state.tag()
    }

    pub fn nodes(&self, id: GroupId) -> &[TagNode] {
        &self._get(id).nodes
    }

    pub fn nodes_mut(&mut self, id: GroupId) -> &mut Vec<TagNode> {
        &mut self._get_mut(id).nodes
    }

    pub fn push_node(&mut self, id: GroupId, node: TagNode) {
        self._get_mut(id).nodes.push(node);
    }

    pub fn pop_node(&mut self, id: GroupId) -> Option<TagNode> {
        self._get_mut(id).nodes.pop()
    }

    pub fn store_unfinished_stack(
        &mut self,
        id: GroupId,
        entries: impl IntoIterator<Item = StackEntry>,
    ) {
        self._get_mut(id).unfinished_stack.extend(entries);
    }

    pub fn take_unfinished_stack(
        &mut self,
        id: GroupId,
    ) -> impl IntoIterator<Item = StackEntry> {
        self._get_mut(id).unfinished_stack.drain(..)
    }

    fn try_set_lang(&mut self, id: GroupId, lang: Lang) -> Option<Lang> {
        let group = &mut self._get_mut(id);
        if let GroupState::Tagged(_, parent_lang) = &mut group.state
            && parent_lang.is_none_or(|l| l == lang)
        {
            *parent_lang = Some(lang);
            return None;
        }
        Some(lang)
    }

    fn take_group(&mut self, id: GroupId) -> Group {
        std::mem::take(&mut self.list[id.0 as usize])
    }

    /// Reserves a located group, if the location hasn't already been reserved,
    /// otherwise returns the already reserved id.
    pub fn reserve_located(
        &mut self,
        options: &PdfOptions,
        loc: Location,
        kind: GroupKind,
    ) -> SourceResult<GroupId> {
        match self.locations.entry(loc) {
            Entry::Occupied(occupied) => {
                let id = *occupied.get();
                let group = &mut self.list[id.0 as usize];

                let is_child = kind == GroupKind::LogicalChild;
                if is_child {
                    group.has_logical_children = true;
                } else {
                    group.num_parents += 1
                }

                if group.span.is_none() {
                    group.span = kind.span();
                }

                if options.is_pdf_ua()
                    && group.num_parents > 1
                    && group.has_logical_children
                {
                    let validator = options.standards.config.validator();
                    let validator = validator.as_str();
                    let span = kind.span().or(group.span).unwrap_or(Span::detached());
                    bail!(
                        span,
                        "{validator} error: ambigous logical parent";
                        hint: "please report this as a bug"
                    );
                }

                if !is_child {
                    if group.num_parents == 1 {
                        group.state = kind.into();
                    } else {
                        // Multiple introspection tags have the same location,
                        // for example because an element was queried and then
                        // placed again. Create a new group that doesn't have
                        // a location mapping.
                        return Ok(self.reserve_virtual(kind));
                    }
                }

                Ok(id)
            }
            Entry::Vacant(vacant) => {
                let id = GroupId(self.list.len() as u32);
                vacant.insert(id);
                self.list.push(Group::new(kind));
                Ok(id)
            }
        }
    }

    /// Reserves a virtual group not associated with any [`Location`].
    pub fn reserve_virtual(&mut self, kind: GroupKind) -> GroupId {
        let id = GroupId(self.list.len() as u32);
        self.list.push(Group::new(kind));
        id
    }

    /// Create the root document group.
    pub fn root_document(&mut self, doc_lang: Option<Lang>) -> GroupId {
        let id = GroupId(self.list.len() as u32);
        self.list.push(Group {
            state: GroupState::Root(doc_lang),
            nodes: Vec::new(),
            span: None,
            num_parents: 1,
            has_logical_children: false,
            unfinished_stack: Vec::new(),
        });
        id
    }

    /// Directly create a virtual group, which didn't originate directly from a
    /// typst element. It has [`Location`] associated with it, and thus cannot
    /// be found by logical children.
    pub fn new_virtual(
        &mut self,
        tag: impl Into<TagKind>,
        nodes: Vec<TagNode>,
    ) -> TagNode {
        let id = GroupId(self.list.len() as u32);
        self.list.push(Group {
            state: GroupState::Tagged(Some(tag.into()), None),
            nodes,
            span: None,
            num_parents: 1,
            has_logical_children: false,
            unfinished_stack: Vec::new(),
        });
        TagNode::Group(id)
    }

    /// Creaate an empty virtual group. See [`Self::new_virtual`].
    pub fn new_empty(&mut self, tag: impl Into<TagKind>) -> TagNode {
        self.new_virtual(tag, Vec::new())
    }

    /// Initialize a group that has been reserved using either
    /// [`Self::reserve_located`] or [`Self::reserve_virtual`].
    pub fn init_tag(
        &mut self,
        tag: impl Into<TagKind>,
        contents: GroupContents,
    ) -> TagNode {
        let tag = tag.into();
        let group = self._get_mut(contents.id);

        match &mut group.state {
            GroupState::Tagged(t, _) => {
                assert!(t.is_none());
                *t = Some(tag);
            }
            GroupState::Root(_) | GroupState::Transparent => unreachable!(),
        }
        TagNode::Group(contents.id)
    }
}

#[derive(Debug, Default)]
pub struct Group {
    state: GroupState,
    pub nodes: Vec<TagNode>,
    pub span: Option<Span>,

    num_parents: u32,
    has_logical_children: bool,

    /// Currently only used for table/grid cells that are broken across multiple
    /// regions, and thus can have opening/closing introspection tags that are
    /// in completely different frames, due to the logical parenting mechanism.
    unfinished_stack: Vec<StackEntry>,
}

impl Group {
    fn new(kind: GroupKind) -> Self {
        let is_child = kind == GroupKind::LogicalChild;
        Group {
            state: kind.into(),
            span: kind.span(),
            num_parents: if is_child { 0 } else { 1 },
            has_logical_children: is_child,
            nodes: Vec::new(),
            unfinished_stack: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupKind {
    /// A tagged group that will produce a PDF tag.
    Tagged(Span),
    /// A logical parent group, that is transparently inserted after the element
    /// content. For example to mark where a place element should be inserted.
    /// This won't produce a PDF tag.
    LogcialParent(Span),
    /// A logical child that is added to a located group.
    LogicalChild,
}

impl GroupKind {
    fn span(self) -> Option<Span> {
        match self {
            GroupKind::Tagged(span) => Some(span),
            GroupKind::LogcialParent(span) => Some(span),
            GroupKind::LogicalChild => None,
        }
    }
}

impl From<GroupKind> for GroupState {
    fn from(val: GroupKind) -> Self {
        match val {
            GroupKind::Tagged(_) => GroupState::Tagged(None, None),
            GroupKind::LogcialParent(_) | GroupKind::LogicalChild => {
                GroupState::Transparent
            }
        }
    }
}

#[derive(Debug, Default)]
enum GroupState {
    Root(Option<Lang>),
    Tagged(Option<TagKind>, Option<Lang>),
    #[default]
    Transparent,
}

impl GroupState {
    pub fn tag(&self) -> Option<&TagKind> {
        match self {
            Self::Root(_) => None,
            Self::Tagged(tag, _) => tag.as_ref(),
            Self::Transparent => None,
        }
    }

    pub fn lang_mut(&mut self) -> Option<&mut Option<Lang>> {
        match self {
            Self::Root(lang) => Some(lang),
            Self::Tagged(_, lang) => Some(lang),
            Self::Transparent => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GroupContents {
    pub id: GroupId,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TagNode {
    Group(GroupId),
    Leaf(Identifier),
    /// Allows inserting a placeholder into the tag tree.
    /// Currently used for [`krilla::page::Page::add_tagged_annotation`].
    Placeholder(Placeholder),
    /// If the attributes are non-empty this will resolve to a [`Tag::Span`],
    /// otherwise the items are inserted directly.
    Text(ResolvedTextAttrs, Vec<Identifier>),
}
