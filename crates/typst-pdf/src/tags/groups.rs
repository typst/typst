use std::collections::hash_map::Entry;

use krilla::tagging::{ArtifactType, Identifier, ListNumbering, TagKind};
use rustc_hash::FxHashMap;
use typst_library::foundations::{Content, Packed};
use typst_library::introspection::Location;
use typst_library::layout::GridCell;
use typst_library::math::EquationElem;
use typst_library::model::{LinkMarker, OutlineEntry, TableCell};
use typst_library::text::Locale;
use typst_library::visualize::ImageElem;
use typst_syntax::Span;

use crate::tags::context::{
    AnnotationId, BBoxId, FigureId, GridId, ListId, OutlineId, TableId, TagId,
};
use crate::tags::resolve::TagNode;
use crate::tags::text::ResolvedTextAttrs;
use crate::tags::util::{self, Id, IdVec};

pub type GroupId = Id<Group>;

impl GroupId {
    pub const ROOT: Self = Self::new(0);
    pub const INVALID: Self = Self::new(u32::MAX);
}

#[derive(Debug)]
pub struct Groups {
    locations: FxHashMap<Location, LocatedGroup>,
    pub list: IdVec<Group>,
    pub tags: TagStorage,
}

impl Groups {
    pub fn new() -> Self {
        Self {
            locations: FxHashMap::default(),
            list: IdVec::new(),
            tags: TagStorage::new(),
        }
    }

    pub fn by_loc(&self, loc: &Location) -> Option<LocatedGroup> {
        self.locations.get(loc).copied()
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn get(&self, id: GroupId) -> &Group {
        self.list.get(id)
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn get_mut(&mut self, id: GroupId) -> &mut Group {
        self.list.get_mut(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Group> {
        self.list.iter()
    }

    /// See [`util::propagate_lang`].
    pub fn propagate_lang(&mut self, id: GroupId, lang: Locale) -> Option<Locale> {
        // TODO: walk up to the first parent that has a language.
        let group = &mut self.get_mut(id);

        let Some(parent) = group.kind.lang_mut() else { return Some(lang) };

        util::propagate_lang(parent, Some(lang))
    }

    /// Create a located group. If the location has already been taken,
    /// create a new virtual group.
    pub fn new_located(
        &mut self,
        loc: Location,
        parent: GroupId,
        span: Span,
        kind: GroupKind,
    ) -> GroupId {
        let id = self.new_virtual(parent, span, kind);
        match self.locations.entry(loc) {
            Entry::Occupied(occupied) => {
                // Multiple introspection tags have the same location,
                // for example because an element was queried and then
                // placed again. Create a new group that doesn't have
                // a location mapping.
                let located = occupied.into_mut();
                located.multiple_parents = true;
            }
            Entry::Vacant(vacant) => {
                vacant.insert(LocatedGroup { id, multiple_parents: false });
            }
        }
        id
    }

    /// Create a new virtual group, not associated with any location.
    pub fn new_virtual(
        &mut self,
        parent: GroupId,
        span: Span,
        kind: GroupKind,
    ) -> GroupId {
        self.list.push(Group::new(parent, span, kind))
    }

    /// NOTE: this needs to be kept in sync with [`Groups::break_group`].
    pub fn is_breakable(&self, kind: &GroupKind, is_pdf_ua: bool) -> bool {
        match kind {
            GroupKind::Root(..) => false,
            GroupKind::Artifact(..) => true,
            GroupKind::LogicalParent(..) => false,
            GroupKind::LogicalChild => false,
            GroupKind::Outline(..) => false,
            GroupKind::OutlineEntry(..) => false,
            GroupKind::Table(..) => false,
            GroupKind::TableCell(..) => false,
            GroupKind::Grid(..) => false,
            GroupKind::GridCell(..) => false,
            GroupKind::List(..) => false,
            GroupKind::ListItemLabel(..) => false,
            GroupKind::ListItemBody(..) => false,
            GroupKind::TermsItemLabel(..) => false,
            GroupKind::TermsItemBody(..) => false,
            GroupKind::BibEntry(..) => false,
            GroupKind::Figure(..) => false,
            GroupKind::FigureCaption(..) => false,
            GroupKind::Image(..) => false,
            GroupKind::Formula(..) => false,
            GroupKind::Link(..) => !is_pdf_ua,
            GroupKind::CodeBlock(..) => false,
            GroupKind::CodeBlockLine(..) => false,
            GroupKind::Standard(tag, ..) => match self.tags.get(*tag) {
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
        }
    }

    /// NOTE: this needs to be kept in sync with [`GroupKind::is_breakable`].
    pub fn break_group(&mut self, id: GroupId, new_parent: GroupId) -> GroupId {
        let group = self.get(id);
        let span = group.span;

        let new_kind = match &group.kind {
            GroupKind::Artifact(ty) => GroupKind::Artifact(*ty),
            GroupKind::Link(elem, _) => GroupKind::Link(elem.clone(), None),
            GroupKind::Standard(old, _) => {
                let tag = self.tags.get(*old).clone();
                let new = self.tags.push(tag);
                GroupKind::Standard(new, None)
            }
            GroupKind::Root(..)
            | GroupKind::LogicalParent(..)
            | GroupKind::LogicalChild
            | GroupKind::Outline(..)
            | GroupKind::OutlineEntry(..)
            | GroupKind::Table(..)
            | GroupKind::TableCell(..)
            | GroupKind::Grid(..)
            | GroupKind::GridCell(..)
            | GroupKind::List(..)
            | GroupKind::ListItemLabel(..)
            | GroupKind::ListItemBody(..)
            | GroupKind::TermsItemLabel(..)
            | GroupKind::TermsItemBody(..)
            | GroupKind::BibEntry(..)
            | GroupKind::Figure(..)
            | GroupKind::FigureCaption(..)
            | GroupKind::Image(..)
            | GroupKind::Formula(..)
            | GroupKind::CodeBlock(..)
            | GroupKind::CodeBlockLine(..) => unreachable!(),
        };
        self.list.push(Group::new(new_parent, span, new_kind))
    }
}

/// These methods are the only way to insert nested groups in the
/// [`Group::nodes`] list.
impl Groups {
    /// Create a new group with a standard tag and push it into the parent.
    pub fn push_tag(&mut self, parent: GroupId, tag: impl Into<TagKind>) -> GroupId {
        let tag_id = self.tags.push(tag);
        let kind = GroupKind::Standard(tag_id, None);
        let id = self.list.push(Group::new(parent, Span::detached(), kind));
        self.get_mut(parent).nodes.push(TagNode::Group(id));
        id
    }

    /// Prepend an existing group to the start of the parent.
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn prepend_group(&mut self, parent: GroupId, child: GroupId) {
        debug_assert!(self.check_ancestor(parent, child));
        self.get_mut(parent).nodes.insert(0, TagNode::Group(child));
    }

    /// Append an existing group to the end of the parent.
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn push_group(&mut self, parent: GroupId, child: GroupId) {
        debug_assert!(self.check_ancestor(parent, child));
        self.get_mut(parent).nodes.push(TagNode::Group(child));
    }

    /// Append multiple existing groups to the end of the parent.
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn extend_groups(
        &mut self,
        parent: GroupId,
        children: impl ExactSizeIterator<Item = GroupId>,
    ) {
        self.get_mut(parent).nodes.reserve(children.len());
        for child in children {
            self.push_group(parent, child);
        }
    }

    /// Check whether the child's [`Group::parent`] is either the `parent` or an
    /// ancestor of the `parent`.
    fn check_ancestor(&self, parent: GroupId, child: GroupId) -> bool {
        let ancestor = self.get(child).parent;
        let mut current = parent;
        while current != GroupId::INVALID {
            if current == ancestor {
                return true;
            }
            current = self.get(current).parent;
        }
        false
    }
}

#[derive(Debug, Default)]
pub struct TagStorage(Vec<Option<TagKind>>);

impl TagStorage {
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    pub fn push(&mut self, tag: impl Into<TagKind>) -> TagId {
        let id = TagId::new(self.0.len() as u32);
        self.0.push(Some(tag.into()));
        id
    }

    pub fn set(&mut self, id: TagId, tag: impl Into<TagKind>) {
        self.0[id.idx()] = Some(tag.into());
    }

    pub fn get(&self, id: TagId) -> &TagKind {
        self.0[id.idx()].as_ref().expect("tag")
    }

    pub fn take(&mut self, id: TagId) -> TagKind {
        self.0[id.idx()].take().expect("tag")
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LocatedGroup {
    pub id: GroupId,
    pub multiple_parents: bool,
}

#[derive(Debug)]
pub struct Group {
    /// The parent of this group. Must not be the direct parent in the concrete
    /// tag tree that will be built. But it must be an ancestor in the resulting
    /// tree. For example for a [`GroupKind::TableCell`] this will point to the
    /// parent [`GroupKind::Table`] even though the concrete tag tree will have
    /// intermediate [`TagKind::TR`] or [`TagKind::TBody`] groups in the
    /// generated `nodes`.
    pub parent: GroupId,
    pub span: Span,
    pub kind: GroupKind,
    /// Only allow mutating this list through the API, to ensure the parent
    /// will be set for child groups.
    nodes: Vec<TagNode>,
}

impl Group {
    fn new(parent: GroupId, span: Span, kind: GroupKind) -> Self {
        Group { parent, span, kind, nodes: Vec::new() }
    }

    pub fn nodes(&self) -> &[TagNode] {
        &self.nodes
    }

    pub fn push_leaf(&mut self, id: Identifier) {
        self.nodes.push(TagNode::Leaf(id));
    }

    pub fn push_annotation(&mut self, annot_id: AnnotationId) {
        self.nodes.push(TagNode::Annotation(annot_id));
    }

    pub fn push_text(&mut self, new_attrs: ResolvedTextAttrs, text_id: Identifier) {
        if new_attrs.is_empty() {
            self.push_leaf(text_id);
            return;
        }

        let last_node = self.nodes.last_mut();
        if let Some(TagNode::Text(prev_attrs, nodes)) = last_node
            && *prev_attrs == new_attrs
        {
            nodes.push(text_id);
        } else {
            self.nodes.push(TagNode::Text(new_attrs, vec![text_id]));
        }
    }

    pub fn pop_node(&mut self) -> Option<TagNode> {
        self.nodes.pop()
    }
}

pub enum GroupKind {
    Root(Option<Locale>),
    Artifact(ArtifactType),
    LogicalParent(Content),
    LogicalChild,
    Outline(OutlineId, Option<Locale>),
    OutlineEntry(Packed<OutlineEntry>, Option<Locale>),
    Table(TableId, BBoxId, Option<Locale>),
    TableCell(Packed<TableCell>, TagId, Option<Locale>),
    Grid(GridId, Option<Locale>),
    GridCell(Packed<GridCell>, Option<Locale>),
    List(ListId, ListNumbering, Option<Locale>),
    ListItemLabel(Option<Locale>),
    ListItemBody(Option<Locale>),
    TermsItemLabel(Option<Locale>),
    TermsItemBody(Option<GroupId>, Option<Locale>),
    BibEntry(Option<Locale>),
    Figure(FigureId, BBoxId, Option<Locale>),
    /// The figure caption has a bbox so marked content sequences won't expand
    /// the bbox of the parent figure group kind. The caption might be moved
    /// into table, or next to to the figure tag.
    FigureCaption(BBoxId, Option<Locale>),
    Image(Packed<ImageElem>, BBoxId, Option<Locale>),
    Formula(Packed<EquationElem>, BBoxId, Option<Locale>),
    Link(Packed<LinkMarker>, Option<Locale>),
    CodeBlock(Option<Locale>),
    CodeBlockLine(Option<Locale>),
    Standard(TagId, Option<Locale>),
}

impl std::fmt::Debug for GroupKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad(match self {
            Self::Root(_) => "Root",
            Self::Artifact(_) => "Artifact",
            Self::LogicalParent(_) => "LogicalParent",
            Self::LogicalChild => "LogicalChild",
            Self::Outline(..) => "Outline",
            Self::OutlineEntry(..) => "OutlineEntry",
            Self::Table(..) => "Table",
            Self::TableCell(..) => "TableCell",
            Self::Grid(..) => "Grid",
            Self::GridCell(..) => "GridCell",
            Self::List(..) => "List",
            Self::ListItemLabel(..) => "ListItemLabel",
            Self::ListItemBody(..) => "ListItemBody",
            Self::TermsItemLabel(..) => "TermsItemLabel",
            Self::TermsItemBody(..) => "TermsItemBody",
            Self::BibEntry(..) => "BibEntry",
            Self::Figure(..) => "Figure",
            Self::FigureCaption(..) => "FigureCaption",
            Self::Image(..) => "Image",
            Self::Formula(..) => "Formula",
            Self::Link(..) => "Link",
            Self::CodeBlock(..) => "CodeBlock",
            Self::CodeBlockLine(..) => "CodeBlockLine",
            Self::Standard(..) => "Standard",
        })
    }
}

impl GroupKind {
    pub fn is_artifact(&self) -> bool {
        matches!(self, Self::Artifact(_))
    }

    pub fn is_link(&self) -> bool {
        matches!(self, Self::Link(..))
    }

    pub fn as_artifact(&self) -> Option<ArtifactType> {
        if let Self::Artifact(v) = self { Some(*v) } else { None }
    }

    pub fn as_list(&self) -> Option<ListId> {
        if let Self::List(v, ..) = self { Some(*v) } else { None }
    }

    pub fn as_link(&self) -> Option<&Packed<LinkMarker>> {
        if let Self::Link(v, ..) = self { Some(v) } else { None }
    }

    pub fn bbox(&self) -> Option<BBoxId> {
        match self {
            GroupKind::Table(_, id, _) => Some(*id),
            GroupKind::Figure(_, id, _) => Some(*id),
            GroupKind::FigureCaption(id, _) => Some(*id),
            GroupKind::Image(_, id, _) => Some(*id),
            GroupKind::Formula(_, id, _) => Some(*id),
            _ => None,
        }
    }

    pub fn lang(&self) -> Option<Option<Locale>> {
        Some(match *self {
            GroupKind::Root(lang) => lang,
            GroupKind::Artifact(_) => return None,
            GroupKind::LogicalParent(_) => return None,
            GroupKind::LogicalChild => return None,
            GroupKind::Outline(_, lang) => lang,
            GroupKind::OutlineEntry(_, lang) => lang,
            GroupKind::Table(_, _, lang) => lang,
            GroupKind::TableCell(_, _, lang) => lang,
            GroupKind::Grid(_, lang) => lang,
            GroupKind::GridCell(_, lang) => lang,
            GroupKind::List(_, _, lang) => lang,
            GroupKind::ListItemLabel(lang) => lang,
            GroupKind::ListItemBody(lang) => lang,
            GroupKind::TermsItemLabel(lang) => lang,
            GroupKind::TermsItemBody(_, lang) => lang,
            GroupKind::BibEntry(lang) => lang,
            GroupKind::Figure(_, _, lang) => lang,
            GroupKind::FigureCaption(_, lang) => lang,
            GroupKind::Image(_, _, lang) => lang,
            GroupKind::Formula(_, _, lang) => lang,
            GroupKind::Link(_, lang) => lang,
            GroupKind::CodeBlock(lang) => lang,
            GroupKind::CodeBlockLine(lang) => lang,
            GroupKind::Standard(_, lang) => lang,
        })
    }

    pub fn lang_mut(&mut self) -> Option<&mut Option<Locale>> {
        Some(match self {
            GroupKind::Root(lang) => lang,
            GroupKind::Artifact(_) => return None,
            GroupKind::LogicalParent(_) => return None,
            GroupKind::LogicalChild => return None,
            GroupKind::Outline(_, lang) => lang,
            GroupKind::OutlineEntry(_, lang) => lang,
            GroupKind::Table(_, _, lang) => lang,
            GroupKind::TableCell(_, _, lang) => lang,
            GroupKind::Grid(_, lang) => lang,
            GroupKind::GridCell(_, lang) => lang,
            GroupKind::List(_, _, lang) => lang,
            GroupKind::ListItemLabel(lang) => lang,
            GroupKind::ListItemBody(lang) => lang,
            GroupKind::TermsItemLabel(lang) => lang,
            GroupKind::TermsItemBody(_, lang) => lang,
            GroupKind::BibEntry(lang) => lang,
            GroupKind::Figure(_, _, lang) => lang,
            GroupKind::FigureCaption(_, lang) => lang,
            GroupKind::Image(_, _, lang) => lang,
            GroupKind::Formula(_, _, lang) => lang,
            GroupKind::Link(_, lang) => lang,
            GroupKind::CodeBlock(lang) => lang,
            GroupKind::CodeBlockLine(lang) => lang,
            GroupKind::Standard(_, lang) => lang,
        })
    }
}
