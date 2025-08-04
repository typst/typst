use std::cell::OnceCell;
use std::collections::HashMap;
use std::slice::SliceIndex;

use krilla::geom as kg;
use krilla::tagging::{
    BBox, Identifier, LineHeight, NaiveRgbColor, Node, Tag, TagKind, TagTree,
    TextDecorationType,
};
use typst_library::diag::{SourceResult, bail};
use typst_library::foundations::{Content, LinkMarker, Packed};
use typst_library::introspection::Location;
use typst_library::layout::{Abs, Length, Point, Rect};
use typst_library::model::{OutlineEntry, TableCell};
use typst_library::pdf::ArtifactKind;
use typst_library::text::Lang;
use typst_syntax::Span;

use crate::PdfOptions;
use crate::convert::FrameContext;
use crate::tags::list::ListCtx;
use crate::tags::outline::OutlineCtx;
use crate::tags::table::TableCtx;
use crate::tags::{Placeholder, TagNode};
use crate::util::AbsExt;

pub struct Tags {
    /// The language of the first text item that has been encountered.
    pub doc_lang: Option<Lang>,
    /// The current set of text attributes.
    pub text_attrs: TextAttrs,
    /// The intermediary stack of nested tag groups.
    pub stack: TagStack,
    /// A list of placeholders corresponding to a [`TagNode::Placeholder`].
    pub placeholders: Placeholders,
    /// Footnotes are inserted directly after the footenote reference in the
    /// reading order. Because of some layouting bugs, the entry might appear
    /// before the reference in the text, so we only resolve them once tags
    /// for the whole document are generated.
    pub footnotes: HashMap<Location, FootnoteCtx>,
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
            footnotes: HashMap::new(),
            disable: None,

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

    pub fn push_text(&mut self, new_attrs: ResolvedTextAttrs, id: Identifier) {
        // FIXME: Artifacts will force a split in the spans, and decoartions
        // generate artifacts
        let last_node = if let Some(entry) = self.stack.last_mut() {
            entry.nodes.last_mut()
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

    pub fn extend(&mut self, nodes: impl IntoIterator<Item = TagNode>) {
        if let Some(entry) = self.stack.last_mut() {
            entry.nodes.extend(nodes);
        } else {
            self.tree.extend(nodes);
        }
    }

    pub fn build_tree(&mut self) -> TagTree {
        assert!(self.stack.items.is_empty(), "tags weren't properly closed");

        let mut nodes = Vec::new();
        for child in std::mem::take(&mut self.tree) {
            self.resolve_node(&mut nodes, child);
        }
        TagTree::from(nodes)
    }

    /// Try to set the language of a parent tag, or the entire document.
    /// If the language couldn't be set and is different from the existing one,
    /// this will return `Some`, and the language should be specified on the
    /// marked content directly.
    pub fn try_set_lang(&mut self, lang: Lang) -> Option<Lang> {
        if self.doc_lang.is_none_or(|l| l == lang) {
            self.doc_lang = Some(lang);
            return None;
        }
        if let Some(last) = self.stack.last_mut()
            && last.lang.is_none_or(|l| l == lang)
        {
            last.lang = Some(lang);
            return None;
        }
        Some(lang)
    }

    /// Resolves nodes into an accumulator.
    fn resolve_node(&mut self, accum: &mut Vec<Node>, node: TagNode) {
        match node {
            TagNode::Group(group) => {
                let mut nodes = Vec::new();
                for child in group.nodes {
                    self.resolve_node(&mut nodes, child);
                }
                let group = krilla::tagging::TagGroup::with_children(group.tag, nodes);
                accum.push(Node::Group(group));
            }
            TagNode::Leaf(identifier) => {
                accum.push(Node::Leaf(identifier));
            }
            TagNode::Placeholder(placeholder) => {
                accum.push(self.placeholders.take(placeholder));
            }
            TagNode::FootnoteEntry(loc) => {
                let node = (self.footnotes.remove(&loc))
                    .and_then(|ctx| ctx.entry)
                    .expect("footnote");
                self.resolve_node(accum, node)
            }
            TagNode::Text(attrs, ids) => {
                let children = ids.into_iter().map(|id| Node::Leaf(id));
                if attrs.is_empty() {
                    accum.extend(children);
                } else {
                    let tag = Tag::Span
                        .with_line_height(attrs.lineheight)
                        .with_baseline_shift(attrs.baseline_shift)
                        .with_text_decoration_type(attrs.deco.map(|d| d.kind.to_krilla()))
                        .with_text_decoration_color(attrs.deco.and_then(|d| d.color))
                        .with_text_decoration_thickness(
                            attrs.deco.and_then(|d| d.thickness),
                        );
                    let group =
                        krilla::tagging::TagGroup::with_children(tag, children.collect());
                    accum.push(Node::Group(group));
                }
            }
        }
    }

    pub fn context_supports(&self, _tag: &StackEntryKind) -> bool {
        // TODO: generate using: https://pdfa.org/resource/iso-ts-32005-hierarchical-inclusion-rules/
        true
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

#[derive(Clone, Copy, Debug)]
pub enum Disable {
    /// Either an artifact or a hide element.
    Elem(Location, ArtifactKind),
    Tiling,
}

#[derive(Clone, Debug)]
pub struct TextAttrs {
    lineheight: Option<LineHeight>,
    baseline_shift: Option<f32>,
    /// PDF can only represent one of the following attributes at a time.
    /// Keep track of all of them, and depending if PDF/UA-1 is enforced, either
    /// throw an error, or just use one of them.
    decos: Vec<(Location, TextDeco)>,
}

impl TextAttrs {
    pub fn new() -> Self {
        Self {
            lineheight: None,
            baseline_shift: None,
            decos: Vec::new(),
        }
    }

    pub fn push_deco(
        &mut self,
        options: &PdfOptions,
        elem: &Content,
        kind: TextDecoKind,
        stroke: TextDecoStroke,
    ) -> SourceResult<()> {
        let deco = TextDeco { kind, stroke };

        // TODO: can overlapping tags break this?
        if self.decos.iter().any(|(_, d)| d.kind != deco.kind) {
            let validator = options.standards.config.validator();
            let validator = validator.as_str();
            bail!(
                elem.span(),
                "{validator} error: cannot combine underline, overline, and or strike"
            );
        }

        let loc = elem.location().unwrap();
        self.decos.push((loc, deco));
        Ok(())
    }

    /// Returns true if a decoration was removed.
    pub fn pop_deco(&mut self, loc: Location) -> bool {
        // TODO: Ideally we would just check the top of the stack, can
        // overlapping tags even happen for decorations?
        if let Some(i) = self.decos.iter().rposition(|(l, _)| *l == loc) {
            self.decos.remove(i);
            return true;
        }
        false
    }

    pub fn resolve(&self, em: Abs) -> ResolvedTextAttrs {
        let deco = self.decos.last().map(|&(_, TextDeco { kind, stroke })| {
            let thickness = stroke.thickness.map(|t| t.at(em).to_f32());
            ResolvedTextDeco { kind, color: stroke.color, thickness }
        });

        ResolvedTextAttrs {
            lineheight: self.lineheight,
            baseline_shift: self.baseline_shift,
            deco,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TextDeco {
    kind: TextDecoKind,
    stroke: TextDecoStroke,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextDecoKind {
    Underline,
    Overline,
    Strike,
}

impl TextDecoKind {
    fn to_krilla(self) -> TextDecorationType {
        match self {
            TextDecoKind::Underline => TextDecorationType::Underline,
            TextDecoKind::Overline => TextDecorationType::Overline,
            TextDecoKind::Strike => TextDecorationType::LineThrough,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TextDecoStroke {
    pub color: Option<NaiveRgbColor>,
    pub thickness: Option<Length>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResolvedTextAttrs {
    lineheight: Option<LineHeight>,
    baseline_shift: Option<f32>,
    deco: Option<ResolvedTextDeco>,
}

impl ResolvedTextAttrs {
    pub fn is_empty(&self) -> bool {
        self.lineheight.is_none() && self.baseline_shift.is_none() && self.deco.is_none()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResolvedTextDeco {
    kind: TextDecoKind,
    color: Option<NaiveRgbColor>,
    thickness: Option<f32>,
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

    pub fn last(&self) -> Option<&StackEntry> {
        self.items.last()
    }

    pub fn last_mut(&mut self) -> Option<&mut StackEntry> {
        self.items.last_mut()
    }

    pub fn iter(&self) -> std::slice::Iter<StackEntry> {
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

    pub fn parent_list(&mut self) -> Option<&mut ListCtx> {
        self.parent()?.as_list_mut()
    }

    pub fn parent_figure(&mut self) -> Option<&mut FigureCtx> {
        self.parent()?.as_figure_mut()
    }

    pub fn parent_outline(&mut self) -> Option<(&mut OutlineCtx, &mut Vec<TagNode>)> {
        self.items.last_mut().and_then(|e| {
            let ctx = e.kind.as_outline_mut()?;
            Some((ctx, &mut e.nodes))
        })
    }

    pub fn find_parent_link(
        &mut self,
    ) -> Option<(LinkId, &Packed<LinkMarker>, &mut Vec<TagNode>)> {
        self.items.iter_mut().rev().find_map(|e| {
            let (link_id, link) = e.kind.as_link()?;
            Some((link_id, link, &mut e.nodes))
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
    pub loc: Location,
    pub span: Span,
    pub lang: Option<Lang>,
    pub kind: StackEntryKind,
    pub nodes: Vec<TagNode>,
}

#[derive(Clone, Debug)]
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
    Figure(FigureCtx),
    Formula(FigureCtx),
    Link(LinkId, Packed<LinkMarker>),
    /// The footnote reference in the text, contains the declaration location.
    FootnoteRef(Location),
    /// The footnote entry at the end of the page. Contains the [`Location`] of
    /// the [`FootnoteElem`](typst_library::model::FootnoteElem).
    FootnoteEntry(Location),
    Code(Option<String>),
}

impl StackEntryKind {
    pub fn as_outline_mut(&mut self) -> Option<&mut OutlineCtx> {
        if let Self::Outline(v) = self { Some(v) } else { None }
    }

    pub fn as_table_mut(&mut self) -> Option<&mut TableCtx> {
        if let Self::Table(v) = self { Some(v) } else { None }
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
            },
            StackEntryKind::Outline(_) => false,
            StackEntryKind::OutlineEntry(_) => false,
            StackEntryKind::Table(_) => false,
            StackEntryKind::TableCell(_) => false,
            StackEntryKind::List(_) => false,
            StackEntryKind::ListItemLabel => false,
            StackEntryKind::ListItemBody => false,
            StackEntryKind::BibEntry => false,
            StackEntryKind::Figure(_) => false,
            StackEntryKind::Formula(_) => false,
            StackEntryKind::Link(..) => !is_pdf_ua,
            StackEntryKind::FootnoteRef(_) => false,
            StackEntryKind::FootnoteEntry(_) => false,
            StackEntryKind::Code(_) => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FootnoteCtx {
    /// Whether this footenote has been referenced inside the document. The
    /// entry will be inserted inside the reading order after the first
    /// reference. All other references will still have links to the footnote.
    pub is_referenced: bool,
    /// The nodes that make up the footnote entry.
    pub entry: Option<TagNode>,
}

impl FootnoteCtx {
    pub const fn new() -> Self {
        Self { is_referenced: false, entry: None }
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

    pub fn get(&self) -> Option<BBox> {
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
