use std::cell::OnceCell;

use krilla::geom as kg;
use krilla::tagging::{BBox, Identifier, Node, TagKind};
use typst_library::layout::{Abs, Point, Rect};
use typst_library::text::Lang;

use crate::convert::FrameContext;
use crate::tags::text::{ResolvedTextAttrs, TextAttrs};
use crate::tags::tree::Tree;
use crate::tags::util::{Id, IdVec};
use crate::tags::{GroupId, GroupKind};
use crate::util::AbsExt;

pub use crate::tags::context::grid::{GridCtx, build_grid};
pub use crate::tags::context::list::ListCtx;
pub use crate::tags::context::outline::OutlineCtx;
pub use crate::tags::context::table::{TableCtx, build_table};

mod grid;
mod list;
mod outline;
mod table;

pub type TableId = Id<TableCtx>;
pub type GridId = Id<GridCtx>;
pub type ListId = Id<ListCtx>;
pub type OutlineId = Id<OutlineCtx>;
pub type BBoxId = Id<BBoxCtx>;
pub type TagId = Id<TagKind>;
pub type AnnotationId = Id<krilla::annotation::Annotation>;

pub struct Tags {
    pub in_tiling: bool,
    pub tree: Tree,
    /// The set of text attributes.
    pub text_attrs: TextAttrs,
    /// A list of placeholders for annotations in the tag tree.
    pub annotations: Annotations,
}

impl Tags {
    pub fn new(tree: Tree) -> Self {
        Self {
            in_tiling: false,
            tree,
            text_attrs: TextAttrs::new(),
            annotations: Annotations::new(),
        }
    }

    pub fn push_leaf(&mut self, id: Identifier) {
        let group = self.tree.groups.get_mut(self.tree.current());
        group.push_leaf(id);
    }

    pub fn push_text(&mut self, new_attrs: ResolvedTextAttrs, text_id: Identifier) {
        let group = self.tree.groups.get_mut(self.tree.current());
        group.push_text(new_attrs, text_id);
    }

    /// Try to set the language of the direct parent tag, or the entire document.
    /// If the language couldn't be set and is different from the existing one,
    /// this will return `Some`, and the language should be specified on the
    /// marked content directly.
    pub fn try_set_lang(&mut self, lang: Lang) -> Option<Lang> {
        self.tree.groups.try_set_lang(self.tree.current(), lang)
    }
}

pub struct Ctx {
    pub tables: IdVec<TableCtx>,
    pub grids: IdVec<GridCtx>,
    pub lists: IdVec<ListCtx>,
    pub outlines: IdVec<OutlineCtx>,
    pub bboxes: IdVec<BBoxCtx>,
}

impl Ctx {
    pub const fn new() -> Self {
        Self {
            tables: IdVec::new(),
            grids: IdVec::new(),
            lists: IdVec::new(),
            outlines: IdVec::new(),
            bboxes: IdVec::new(),
        }
    }

    pub fn new_bbox(&mut self) -> BBoxId {
        self.bboxes.push(BBoxCtx::new())
    }

    pub fn bbox(&self, kind: &GroupKind) -> Option<&BBoxCtx> {
        Some(self.bboxes.get(kind.bbox()?))
    }
}

pub struct Annotations(Vec<OnceCell<Identifier>>);

impl Annotations {
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    pub fn reserve(&mut self) -> AnnotationId {
        let id = AnnotationId::new(self.0.len() as u32);
        self.0.push(OnceCell::new());
        id
    }

    pub fn init(&mut self, id: AnnotationId, annot: Identifier) {
        self.0[id.idx()]
            .set(annot)
            .map_err(|_| ())
            .expect("annotation to be uninitialized");
    }

    pub fn take(&mut self, id: AnnotationId) -> Node {
        let annot = self.0[id.idx()].take().expect("initialized annotation node");
        Node::Leaf(annot)
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

    /// Expand the bounding box with a `rect` relative to the current frame
    /// context transform.
    pub fn expand_frame(
        &mut self,
        fc: &FrameContext,
        compute_rect: impl FnOnce() -> Rect,
    ) {
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

        let rect = compute_rect();
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
    pub fn expand_page(&mut self, inner: &BBoxCtx) {
        self.multi_page |= inner.multi_page;
        if self.multi_page {
            return;
        }

        let Some((page_idx, rect)) = inner.rect else { return };
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

#[derive(Debug, Clone, PartialEq)]
pub enum TagNode {
    Group(GroupId),
    Leaf(Identifier),
    /// Allows inserting a annotation into the tag tree.
    /// Currently used for [`krilla::page::Page::add_tagged_annotation`].
    Annotation(AnnotationId),
    /// If the attributes are non-empty this will resolve to a [`Tag::Span`],
    /// otherwise the items are inserted directly.
    Text(ResolvedTextAttrs, Vec<Identifier>),
}
