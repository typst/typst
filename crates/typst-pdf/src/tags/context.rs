use std::cell::OnceCell;
use std::slice::SliceIndex;

use krilla::geom as kg;
use krilla::tagging as kt;
use krilla::tagging::{BBox, Identifier, Node, TagKind, TagTree};
use rustc_hash::FxHashMap;
use typst_library::foundations::{LinkMarker, Packed};
use typst_library::introspection::Location;
use typst_library::layout::{Abs, GridCell, Point, Rect};
use typst_library::model::{OutlineEntry, TableCell};
use typst_library::pdf::ArtifactKind;
use typst_library::text::Lang;
use typst_syntax::Span;

use crate::convert::FrameContext;
use crate::tags::grid::{GridCtx, TableCtx};
use crate::tags::list::ListCtx;
use crate::tags::outline::OutlineCtx;
use crate::tags::text::{ResolvedTextAttrs, TextAttrs};
use crate::util::AbsExt;

pub struct Tags {
    /// The language of the first text item that has been encountered.
    pub doc_lang: Option<Lang>,
    /// The set of text attributes.
    pub text_attrs: TextAttrs,
    /// The intermediary stack of nested tag groups.
    pub stack: TagStack,
    /// A list of placeholders corresponding to a [`TagNode::Placeholder`].
    pub placeholders: Placeholders,
    pub groups: Groups,
    pub disable: Option<Disable>,
    /// Used to group multiple link annotations using quad points.
    link_id: LinkId,
    /// Used to generate IDs referenced in table `Headers` attributes.
    /// The IDs must be document wide unique.
    table_id: TableId,

    /// The output.
    tree: Vec<TagNode>,
}

impl Tags {
    pub fn new() -> Self {
        Self {
            doc_lang: None,
            text_attrs: TextAttrs::new(),
            stack: TagStack::new(),
            placeholders: Placeholders(Vec::new()),
            groups: Groups::default(),
            disable: None,

            link_id: LinkId(0),
            table_id: TableId(0),

            tree: Vec::new(),
        }
    }

    pub fn push(&mut self, node: TagNode) {
        if let Some(entry) = self.stack.last_mut() {
            self.groups.get_mut(entry.id).nodes.push(node);
        } else {
            self.tree.push(node);
        }
    }

    pub fn push_text(&mut self, new_attrs: ResolvedTextAttrs, id: Identifier) {
        if new_attrs.is_empty() {
            self.push(TagNode::Leaf(id));
            return;
        }

        let last_node = if let Some(entry) = self.stack.last_mut() {
            self.groups.get_mut(entry.id).nodes.last_mut()
        } else {
            self.tree.last_mut()
        };
        if let Some(TagNode::Text(prev_attrs, nodes)) = last_node
            && *prev_attrs == new_attrs
        {
            nodes.push(id);
        } else {
            self.push(TagNode::Text(new_attrs, vec![id]));
        }
    }

    pub fn build_tree(&mut self) -> TagTree {
        assert!(self.stack.items.is_empty(), "tags weren't properly closed");

        let mut children = Vec::with_capacity(self.tree.len());

        for child in std::mem::take(&mut self.tree) {
            resolve_node(
                &mut self.groups,
                &mut self.placeholders,
                &mut self.doc_lang,
                &mut children,
                child,
            );
        }

        TagTree::from(children)
    }

    /// Try to set the language of the direct parent tag, or the entire document.
    /// If the language couldn't be set and is different from the existing one,
    /// this will return `Some`, and the language should be specified on the
    /// marked content directly.
    pub fn try_set_lang(&mut self, lang: Lang) -> Option<Lang> {
        if self.doc_lang.is_none_or(|l| l == lang) {
            self.doc_lang = Some(lang);
            return None;
        }
        if let Some(last) = self.stack.last_mut()
            && let last = &mut self.groups.get_mut(last.id)
            && last.lang.is_none_or(|l| l == lang)
        {
            last.lang = Some(lang);
            return None;
        }
        Some(lang)
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
            let mut group = groups.take(id);

            let mut nodes = Vec::with_capacity(group.nodes.len());
            let lang =
                if group.tag.is_some() { &mut group.lang } else { &mut parent_lang };
            for child in group.nodes.into_iter() {
                resolve_node(groups, placeholders, lang, &mut nodes, child);
            }

            // Try to propagate the groups language to the parent tag.
            group.lang = group.lang.and_then(|lang| {
                if parent_lang.is_none_or(|l| l == lang) {
                    *parent_lang = Some(lang);
                    return None;
                }
                Some(lang)
            });

            if let Some(mut tag) = group.tag {
                tag.set_lang(group.lang.map(|l| l.as_str().to_string()));
                let group = kt::TagGroup::with_children(tag, nodes);
                accum.push(Node::Group(group));
            } else if let Some(lang) = group.lang {
                // HACK: This should never happen. This group has a language
                // attribute associated with it, that couldn't be propagated to
                // the parent group, but also it doesn't have a tag because it
                // is a transparent group. This will only happen if a transparent
                // group has direct children that are marked content sequences,
                // which currently cannot happen.
                let tag = kt::Tag::NonStruct.with_lang(Some(lang.as_str().to_string()));
                let group = kt::TagGroup::with_children(tag, nodes);
                accum.push(Node::Group(group));
            } else {
                accum.extend(nodes);
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

    pub fn last_mut(&mut self) -> Option<&mut StackEntry> {
        self.items.last_mut()
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
    LogicalParent,
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
    Link(LinkId, Packed<LinkMarker>),
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
        if let Self::Link(id, link) = self { Some((*id, link)) } else { None }
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
            StackEntryKind::LogicalParent => false,
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

#[derive(Debug, Default)]
pub struct Groups {
    locations: FxHashMap<Location, GroupId>,
    list: Vec<Group>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GroupId(u32);

impl Groups {
    pub fn get(&self, id: GroupId) -> &Group {
        &self.list[id.0 as usize]
    }

    pub fn get_mut(&mut self, id: GroupId) -> &mut Group {
        &mut self.list[id.0 as usize]
    }

    pub fn take(&mut self, id: GroupId) -> Group {
        std::mem::take(&mut self.list[id.0 as usize])
    }

    /// Reserves a located group, if the location hasn't already been reserved,
    /// otherwise returns the already reserved id.
    pub fn reserve_located(&mut self, loc: Location) -> GroupId {
        *self.locations.entry(loc).or_insert_with(|| {
            let id = GroupId(self.list.len() as u32);
            self.list.push(Group { tag: None, lang: None, nodes: Vec::new() });
            id
        })
    }

    /// Reserves a virtual group not associated with any [`Location`].
    pub fn reserve_virtual(&mut self) -> GroupId {
        let id = GroupId(self.list.len() as u32);
        self.list.push(Group { tag: None, lang: None, nodes: Vec::new() });
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
        self.list.push(Group { tag: Some(tag.into()), lang: None, nodes });
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
        let tag = tag.into().with_location(Some(contents.span.into_raw()));
        let group = self.get_mut(contents.id);
        assert!(group.tag.is_none());
        group.tag = Some(tag);
        TagNode::Group(contents.id)
    }
}

#[derive(Debug, Default)]
pub struct Group {
    /// The optional parent tag of this group. If not present, either there is
    /// no parent tag for these children and they will be added directly to the
    /// tag tree, or it hasn't been found yet.
    pub tag: Option<TagKind>,
    pub lang: Option<Lang>,
    pub nodes: Vec<TagNode>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GroupContents {
    pub id: GroupId,
    pub span: Span,
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
