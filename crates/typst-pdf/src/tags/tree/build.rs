//! Building the logical tree.
//!
//! The tree of [`Frame`]s which is split up into pages doesn't necessarily
//! represent the logical structure of the Typst document. The logical structure
//! is instead defined by the start and end [`introspection::Tag`]s and
//! additional insertions of frames by the means of [`Frame::set_parent`].
//! These inserted frames resolve to groups of kind [`GroupKind::LogicalChild`].
//!
//! This module resolves the logical structure in a pre-pass, so that the
//! complete logical tree is available when the document's content is converted.
//!
//! [`introspection::Tag`]: typst_library::introspection::Tag
//! [`FrameItem::parent`]: typst_library::layout::FrameItem

use std::num::NonZeroU16;

use krilla::tagging::{ArtifactType, ListNumbering, Tag, TagKind};
use rustc_hash::FxHashMap;
use smallvec::SmallVec;
use typst_library::diag::{SourceResult, bail};
use typst_library::foundations::Content;
use typst_library::introspection::Location;
use typst_library::layout::{
    Frame, FrameItem, GridCell, GridElem, GroupItem, HideElem, PagedDocument, PlaceElem,
    RepeatElem,
};
use typst_library::math::EquationElem;
use typst_library::model::{
    EnumElem, FigureCaption, FigureElem, FootnoteElem, FootnoteEntry, HeadingElem,
    LinkMarker, ListElem, Outlinable, OutlineEntry, ParElem, QuoteElem, TableCell,
    TableElem, TermsElem, TitleElem,
};
use typst_library::pdf::{ArtifactElem, PdfMarkerTag, PdfMarkerTagKind};
use typst_library::text::{RawElem, RawLine};
use typst_library::visualize::ImageElem;
use typst_syntax::Span;

use crate::PdfOptions;
use crate::tags::GroupId;
use crate::tags::context::{Ctx, FigureCtx, GridCtx, ListCtx, OutlineCtx, TableCtx};
use crate::tags::groups::{GroupKind, Groups};
use crate::tags::tree::{Break, BreakKind, TraversalStates, Tree};
use crate::tags::util::{ArtifactKindExt, PropertyValCopied};

pub struct TreeBuilder<'a> {
    options: &'a PdfOptions<'a>,

    /// Each [`FrameItem::Tag`] and each [`FrameItem::Group`] with a parent
    /// will append a progression to this tree. This list of progressions is
    /// used to determine the location in the tree when doing the actual PDF
    /// generation and inserting the marked content sequences.
    progressions: Vec<GroupId>,
    breaks: Vec<Break>,
    groups: Groups,
    ctx: Ctx,
    logical_children: FxHashMap<Location, SmallVec<[GroupId; 4]>>,

    stack: TagStack,
    /// Currently only used for table/grid cells that are broken across multiple
    /// regions, and thus can have opening/closing introspection tags that are
    /// in completely different frames, due to the logical parenting mechanism.
    unfinished_stacks: FxHashMap<Location, Vec<StackEntry>>,
}

impl<'a> TreeBuilder<'a> {
    pub fn new(document: &PagedDocument, options: &'a PdfOptions) -> Self {
        let mut groups = Groups::new();
        let doc = groups.new_virtual(
            GroupId::INVALID,
            Span::detached(),
            GroupKind::Root(document.info.lang.custom()),
        );
        Self {
            options,
            progressions: vec![doc],
            breaks: Vec::new(),
            groups,
            ctx: Ctx::new(),
            logical_children: FxHashMap::default(),

            stack: TagStack::new(),
            unfinished_stacks: FxHashMap::default(),
        }
    }

    pub fn finish(self) -> Tree {
        Tree {
            prog_cursor: 0,
            progressions: self.progressions,
            break_cursor: 0,
            breaks: self.breaks,
            state: TraversalStates::new(),
            groups: self.groups,
            ctx: self.ctx,
            logical_children: self.logical_children,
        }
    }

    pub fn root_document(&self) -> GroupId {
        self.progressions[0]
    }

    /// The last group in the progression.
    pub fn current(&self) -> GroupId {
        *self.progressions.last().unwrap()
    }

    /// The last group on the stack or the root document.
    pub fn parent(&self) -> GroupId {
        self.stack.last().map(|e| e.id).unwrap_or(self.root_document())
    }

    pub fn parent_kind(&self) -> &GroupKind {
        &self.groups.get(self.parent()).kind
    }

    pub fn insert_break(&mut self, kind: BreakKind) {
        let progression_idx = self.progressions.len() as u32;
        self.breaks.push(Break { progression_idx, kind });
    }
}

#[derive(Debug)]
struct TagStack {
    items: Vec<StackEntry>,
}

impl std::ops::Index<usize> for TagStack {
    type Output = StackEntry;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        std::ops::Index::index(&self.items, index)
    }
}

impl std::ops::IndexMut<usize> for TagStack {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        std::ops::IndexMut::index_mut(&mut self.items, index)
    }
}

impl TagStack {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn last(&self) -> Option<&StackEntry> {
        self.items.last()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, StackEntry> {
        self.items.iter()
    }

    pub fn push(&mut self, entry: StackEntry) {
        self.items.push(entry);
    }

    pub fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = StackEntry>,
        I::IntoIter: ExactSizeIterator,
    {
        self.items.extend(iter)
    }

    pub fn pop_if(
        &mut self,
        mut predicate: impl FnMut(&mut StackEntry) -> bool,
    ) -> Option<StackEntry> {
        let last = self.items.last_mut()?;
        if predicate(last) { self.pop() } else { None }
    }

    pub fn pop(&mut self) -> Option<StackEntry> {
        self.items.pop()
    }

    pub fn truncate(&mut self, len: usize) {
        self.items.truncate(len);
    }

    /// Remove all stack entries after the idx.
    pub fn take_unfinished_stack(&mut self, idx: usize) -> Option<Vec<StackEntry>> {
        if idx + 1 < self.items.len() {
            Some(self.items.drain(idx + 1..).collect())
        } else {
            None
        }
    }
}

#[derive(Debug)]
struct StackEntry {
    /// The location of the stack entry. If this is `None` the stack entry has
    /// to be manually popped.
    loc: Option<Location>,
    id: GroupId,
}

pub fn build(document: &PagedDocument, options: &PdfOptions) -> SourceResult<Tree> {
    let mut tree = TreeBuilder::new(document, options);
    for page in document.pages.iter() {
        visit_frame(&mut tree, &page.frame)?;
    }

    assert!(tree.stack.is_empty(), "tags weren't properly closed");
    assert!(tree.unfinished_stacks.is_empty(), "tags weren't properly closed");
    assert_eq!(
        tree.progressions.first(),
        tree.progressions.last(),
        "tags weren't properly closed"
    );

    // Insert logical children into the tree.
    #[allow(clippy::iter_over_hash_type)]
    for (loc, children) in tree.logical_children.iter() {
        let located = tree.groups.by_loc(loc).expect("parent group");

        if options.is_pdf_ua() && located.multiple_parents {
            let validator = options.standards.config.validator().as_str();
            let group = tree.groups.get(located.id);
            bail!(
                group.span,
                "{validator} error: ambiguous logical parent";
                hint: "please report this as a bug"
            );
        }

        for child in children.iter() {
            tree.groups.get_mut(*child).parent = located.id;
        }
    }

    #[cfg(debug_assertions)]
    for group in tree.groups.iter().skip(1) {
        assert_ne!(group.parent, GroupId::INVALID);
    }

    Ok(tree.finish())
}

fn visit_frame(tree: &mut TreeBuilder, frame: &Frame) -> SourceResult<()> {
    for (_, item) in frame.items() {
        match item {
            FrameItem::Group(group) => visit_group_frame(tree, group)?,
            FrameItem::Tag(typst_library::introspection::Tag::Start(elem)) => {
                visit_start_tag(tree, elem);
            }
            FrameItem::Tag(typst_library::introspection::Tag::End(loc, _)) => {
                visit_end_tag(tree, *loc)?;
            }
            FrameItem::Text(_) => (),
            FrameItem::Shape(..) => (),
            FrameItem::Image(..) => (),
            FrameItem::Link(..) => (),
        }
    }
    Ok(())
}

/// Handle children frames logically belonging to another element, because
/// [typst_library::layout::GroupItem::parent] has been set. All elements that
/// can have children set by this mechanism must be handled in [`handle_start`]
/// and must produce a located [`Group`], so the children can be inserted there.
///
/// Currently the the frame parent is only set for:
/// - place elements [`PlaceElem`]
/// - footnote entries [`FootnoteEntry`]
/// - broken table/grid cells [`TableCell`]/[`GridCell`]
fn visit_group_frame(tree: &mut TreeBuilder, group: &GroupItem) -> SourceResult<()> {
    let Some(parent_loc) = group.parent else {
        return visit_frame(tree, &group.frame);
    };

    let prev = tree.current();

    let id = tree.groups.new_virtual(
        GroupId::INVALID,
        Span::detached(),
        GroupKind::LogicalChild,
    );
    tree.logical_children.entry(parent_loc).or_default().push(id);

    let stack_idx = tree.stack.len();
    push_stack_entry(tree, None, id);
    if let Some(stack) = tree.unfinished_stacks.remove(&parent_loc) {
        tree.stack.extend(stack);
    }
    // Move to the top of the stack, including the pushed on unfinished stack.
    tree.progressions.push(tree.stack.last().unwrap().id);

    visit_frame(tree, &group.frame)?;

    if let Some(stack) = tree.stack.take_unfinished_stack(stack_idx) {
        tree.unfinished_stacks.insert(parent_loc, stack);
        tree.insert_break(BreakKind::Unfinished { group_to_close: id });
    }
    tree.stack.pop().expect("stack entry");
    tree.progressions.push(prev);
    Ok(())
}

fn visit_start_tag(tree: &mut TreeBuilder, elem: &Content) {
    let group_id = progress_tree_start(tree, elem);
    tree.progressions.push(group_id);
}

fn visit_end_tag(tree: &mut TreeBuilder, loc: Location) -> SourceResult<()> {
    let group = progress_tree_end(tree, loc)?;
    tree.progressions.push(group);
    Ok(())
}

fn progress_tree_start(tree: &mut TreeBuilder, elem: &Content) -> GroupId {
    #[allow(clippy::redundant_pattern_matching)]
    if let Some(_) = elem.to_packed::<HideElem>() {
        push_artifact(tree, elem, ArtifactType::Other)
    } else if let Some(artifact) = elem.to_packed::<ArtifactElem>() {
        let kind = artifact.kind.val();
        push_artifact(tree, elem, kind.to_krilla())
    } else if let Some(_) = elem.to_packed::<RepeatElem>() {
        push_artifact(tree, elem, ArtifactType::Other)
    } else if let Some(tag) = elem.to_packed::<PdfMarkerTag>() {
        match &tag.kind {
            PdfMarkerTagKind::OutlineBody => {
                let id = tree.ctx.outlines.push(OutlineCtx::new());
                push_stack(tree, elem, GroupKind::Outline(id, None))
            }
            PdfMarkerTagKind::Bibliography(numbered) => {
                let numbering =
                    if *numbered { ListNumbering::Decimal } else { ListNumbering::None };
                let id = tree.ctx.lists.push(ListCtx::new());
                push_stack(tree, elem, GroupKind::List(id, numbering, None))
            }
            PdfMarkerTagKind::BibEntry => {
                push_stack(tree, elem, GroupKind::BibEntry(None))
            }
            PdfMarkerTagKind::ListItemLabel => {
                push_stack(tree, elem, GroupKind::ListItemLabel(None))
            }
            PdfMarkerTagKind::ListItemBody => {
                push_stack(tree, elem, GroupKind::ListItemBody(None))
            }
            PdfMarkerTagKind::Label => push_tag(tree, elem, Tag::Lbl),
        }
    } else if let Some(link) = elem.to_packed::<LinkMarker>() {
        push_stack(tree, elem, GroupKind::Link(link.clone(), None))
    } else if let Some(_) = elem.to_packed::<TitleElem>() {
        push_tag(tree, elem, Tag::Title)
    } else if let Some(entry) = elem.to_packed::<OutlineEntry>() {
        push_stack(tree, elem, GroupKind::OutlineEntry(entry.clone(), None))
    } else if let Some(_) = elem.to_packed::<ListElem>() {
        // TODO: infer numbering from `list.marker`
        let numbering = ListNumbering::Circle;
        let id = tree.ctx.lists.push(ListCtx::new());
        push_stack(tree, elem, GroupKind::List(id, numbering, None))
    } else if let Some(_) = elem.to_packed::<EnumElem>() {
        // TODO: infer numbering from `enum.numbering`
        let numbering = ListNumbering::Decimal;
        let id = tree.ctx.lists.push(ListCtx::new());
        push_stack(tree, elem, GroupKind::List(id, numbering, None))
    } else if let Some(_) = elem.to_packed::<TermsElem>() {
        let numbering = ListNumbering::None;
        let id = tree.ctx.lists.push(ListCtx::new());
        push_stack(tree, elem, GroupKind::List(id, numbering, None))
    } else if let Some(figure) = elem.to_packed::<FigureElem>() {
        let bbox = tree.ctx.new_bbox();
        let figure = tree.ctx.figures.push(FigureCtx::new(figure.clone()));
        push_stack(tree, elem, GroupKind::Figure(figure, bbox, None))
    } else if let Some(_) = elem.to_packed::<FigureCaption>() {
        let bbox = tree.ctx.new_bbox();
        push_stack(tree, elem, GroupKind::FigureCaption(bbox, None))
    } else if let Some(image) = elem.to_packed::<ImageElem>() {
        let bbox = tree.ctx.new_bbox();
        push_stack(tree, elem, GroupKind::Image(image.clone(), bbox, None))
    } else if let Some(equation) = elem.to_packed::<EquationElem>() {
        let bbox = tree.ctx.new_bbox();
        push_stack(tree, elem, GroupKind::Formula(equation.clone(), bbox, None))
    } else if let Some(table) = elem.to_packed::<TableElem>() {
        let id = tree.ctx.tables.push_with(|id| TableCtx::new(id, table.clone()));
        let bbox = tree.ctx.new_bbox();
        push_stack(tree, elem, GroupKind::Table(id, bbox, None))
    } else if let Some(cell) = elem.to_packed::<TableCell>() {
        // Only repeated table headers and footer cells are laid out multiple
        // times. Mark duplicate headers as artifacts, since they have no
        // semantic meaning in the tag tree, which doesn't use page breaks for
        // it's semantic structure.
        if cell.is_repeated.val() {
            push_artifact(tree, elem, ArtifactType::Other)
        } else {
            let tag = tree.groups.tags.push(Tag::TD);
            push_stack(tree, elem, GroupKind::TableCell(cell.clone(), tag, None))
        }
    } else if let Some(grid) = elem.to_packed::<GridElem>() {
        let id = tree.ctx.grids.push(GridCtx::new(grid));
        push_stack(tree, elem, GroupKind::Grid(id, None))
    } else if let Some(cell) = elem.to_packed::<GridCell>() {
        // If there is no grid parent, this means a grid layouter is used
        // internally. Don't generate a stack entry.
        if !matches!(tree.parent_kind(), GroupKind::Grid(..)) {
            return no_progress(tree);
        }

        // The grid cells are collected into a grid to ensure proper reading
        // order, even when using rowspans, which may be laid out later than
        // other cells in the same row.

        // Only repeated grid headers and footer cells are laid out multiple
        // times. Mark duplicate headers as artifacts, since they have no
        // semantic meaning in the tag tree, which doesn't use page breaks for
        // it's semantic structure.
        if cell.is_repeated.val() {
            push_artifact(tree, elem, ArtifactType::Other)
        } else {
            push_stack(tree, elem, GroupKind::GridCell(cell.clone(), None))
        }
    } else if let Some(heading) = elem.to_packed::<HeadingElem>() {
        let level = heading.level().try_into().unwrap_or(NonZeroU16::MAX);
        let name = heading.body.plain_text().to_string();
        push_tag(tree, elem, Tag::Hn(level, Some(name)))
    } else if let Some(_) = elem.to_packed::<ParElem>() {
        push_tag(tree, elem, Tag::P)
    } else if let Some(_) = elem.to_packed::<FootnoteElem>() {
        push_stack(tree, elem, GroupKind::LogicalParent(elem.clone()))
    } else if let Some(_) = elem.to_packed::<FootnoteEntry>() {
        push_tag(tree, elem, Tag::Note)
    } else if let Some(quote) = elem.to_packed::<QuoteElem>() {
        // TODO: should the attribution be handled somehow?
        if quote.block.val() {
            push_tag(tree, elem, Tag::BlockQuote)
        } else {
            push_tag(tree, elem, Tag::InlineQuote)
        }
    } else if let Some(raw) = elem.to_packed::<RawElem>() {
        if raw.block.val() {
            push_stack(tree, elem, GroupKind::CodeBlock(None))
        } else {
            push_tag(tree, elem, Tag::Code)
        }
    } else if let Some(_) = elem.to_packed::<RawLine>() {
        // If the raw element is inline, the content can be inserted directly.
        if matches!(tree.parent_kind(), GroupKind::CodeBlock(..)) {
            push_stack(tree, elem, GroupKind::CodeBlockLine(None))
        } else {
            no_progress(tree)
        }
    } else if let Some(place) = elem.to_packed::<PlaceElem>() {
        if place.float.val() {
            push_stack(tree, elem, GroupKind::LogicalParent(elem.clone()))
        } else {
            no_progress(tree)
        }
    } else {
        no_progress(tree)
    }
}

fn no_progress(tree: &TreeBuilder) -> GroupId {
    tree.current()
}

fn push_tag(tree: &mut TreeBuilder, elem: &Content, tag: impl Into<TagKind>) -> GroupId {
    let id = tree.groups.tags.push(tag.into());
    push_stack(tree, elem, GroupKind::Standard(id, None))
}

fn push_stack(tree: &mut TreeBuilder, elem: &Content, kind: GroupKind) -> GroupId {
    let loc = elem.location().expect("elem to be locatable");
    let span = elem.span();
    let parent = tree.current();
    let id = tree.groups.new_located(loc, parent, span, kind);
    push_stack_entry(tree, Some(loc), id)
}

fn push_artifact(tree: &mut TreeBuilder, elem: &Content, ty: ArtifactType) -> GroupId {
    let loc = elem.location().expect("elem to be locatable");
    let span = elem.span();
    let parent = tree.current();
    let kind = GroupKind::Artifact(ty);
    let id = tree.groups.new_virtual(parent, span, kind);
    push_stack_entry(tree, Some(loc), id)
}

fn push_stack_entry(
    tree: &mut TreeBuilder,
    loc: Option<Location>,
    id: GroupId,
) -> GroupId {
    let entry = StackEntry { loc, id };
    tree.stack.push(entry);
    id
}

fn progress_tree_end(tree: &mut TreeBuilder, loc: Location) -> SourceResult<GroupId> {
    if tree.stack.pop_if(|e| e.loc == Some(loc)).is_some() {
        // The tag nesting was properly closed.
        return Ok(tree.parent());
    }

    // Search for an improperly nested starting tag, that is being closed.
    let Some(stack_idx) = (tree.stack.iter().enumerate())
        .rev()
        .find_map(|(i, e)| (e.loc == Some(loc)).then_some(i))
    else {
        // The start tag isn't in the tag stack, just ignore the end tag.
        return Ok(no_progress(tree));
    };

    // Table/grid cells can only have overlapping tags if they are broken across
    // multiple regions. In that case store the unfinished stack entries, and
    // push them back on when processing the logical children.
    let entry_id = tree.stack[stack_idx].id;
    let group = tree.groups.get(entry_id);
    if matches!(group.kind, GroupKind::TableCell(..) | GroupKind::GridCell(..)) {
        if let Some(stack) = tree.stack.take_unfinished_stack(stack_idx) {
            tree.unfinished_stacks.insert(loc, stack);
            tree.insert_break(BreakKind::Unfinished { group_to_close: entry_id });
        }
        tree.stack.pop().unwrap();
        return Ok(tree.parent());
    }

    // There are overlapping tags in the tag tree. Figure out whether breaking
    // up the current tag stack is semantically ok.
    let mut is_breakable = true;
    let mut non_breakable_span = Span::detached();
    for e in tree.stack.iter().skip(stack_idx + 1) {
        let group = tree.groups.get(e.id);
        if tree.groups.is_breakable(&group.kind, tree.options.is_pdf_ua()) {
            continue;
        }

        is_breakable = false;
        if !group.span.is_detached() {
            non_breakable_span = group.span;
            break;
        }
    }
    if !is_breakable {
        if tree.options.is_pdf_ua() {
            let validator = tree.options.standards.config.validator().as_str();
            bail!(
                non_breakable_span,
                "{validator} error: invalid semantic structure, \
                    this element's tag would be split up";
                hint: "maybe this is caused by a `parbreak`, `colbreak`, or `pagebreak`"
            );
        } else {
            bail!(
                non_breakable_span,
                "invalid semantic structure, \
                this element's tag would be split up";
                hint: "maybe this is caused by a `parbreak`, `colbreak`, or `pagebreak`";
                hint: "disable tagged pdf by passing `--no-pdf-tags`"
            );
        }
    }

    // Duplicate all broken entries.
    let mut parent = group.parent;
    let new_entries = (tree.stack.iter())
        .skip(stack_idx + 1)
        .map(|broken_entry| {
            let new_id = tree.groups.break_group(broken_entry.id, parent);
            parent = new_id;
            StackEntry { loc: broken_entry.loc, id: new_id }
        })
        .collect::<Vec<_>>();

    // Since the broken groups won't be visited again in any future progression,
    // they'll need to be closed when this progression is visited.
    let num_closed_groups = (tree.stack.len() - stack_idx) as u16;
    tree.insert_break(BreakKind::Broken { num_closed_groups });

    // Remove the closed entries.
    tree.stack.truncate(stack_idx);

    // Push all broken and afterwards duplicated entries back on.
    tree.stack.extend(new_entries);

    // We're now in a new duplicated group
    Ok(tree.parent())
}
