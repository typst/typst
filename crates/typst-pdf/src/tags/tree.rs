use std::num::NonZeroU16;

use krilla::surface::Surface;
use krilla::tagging::{ArtifactType, ContentTag, ListNumbering, Tag, TagKind};
use rustc_hash::FxHashMap;
use smallvec::SmallVec;
use typst_library::diag::{SourceResult, bail};
use typst_library::foundations::{Content, LinkMarker, Packed};
use typst_library::introspection::Location;
use typst_library::layout::{
    Frame, FrameItem, GridCell, GridElem, GroupItem, HideElem, PagedDocument, PlaceElem,
    RepeatElem,
};
use typst_library::math::EquationElem;
use typst_library::model::{
    EnumElem, FigureCaption, FigureElem, FootnoteElem, FootnoteEntry, HeadingElem,
    ListElem, Outlinable, OutlineEntry, ParElem, QuoteElem, TableCell, TableElem,
    TermsElem,
};
use typst_library::pdf::{ArtifactElem, PdfMarkerTag, PdfMarkerTagKind};
use typst_library::text::{RawElem, RawLine};
use typst_library::visualize::ImageElem;
use typst_syntax::Span;

use crate::PdfOptions;
use crate::tags::context::{self, Ctx, GridCtx, ListCtx, OutlineCtx, TableCtx};
use crate::tags::util::{ArtifactKindExt, PropertyValCopied};
use crate::tags::{BBoxCtx, FigureCtx, GroupId, GroupKind, Groups};

pub struct Tree {
    /// Points at the current group in the `progressions` list.
    prog_cursor: usize,
    progressions: Vec<GroupId>,
    /// Points at the next break in the `breaks` list.
    break_cursor: usize,
    breaks: Vec<Break>,
    state: TreeStates,
    pub groups: Groups,
    pub ctx: Ctx,
}

impl Tree {
    pub fn empty() -> Self {
        Self {
            prog_cursor: 0,
            progressions: Vec::new(),
            break_cursor: 0,
            breaks: Vec::new(),
            state: TreeStates::new(),
            groups: Groups::new(),
            ctx: Ctx::new(),
        }
    }

    pub fn root(&self) -> GroupId {
        self.progressions[0]
    }

    pub fn current(&self) -> GroupId {
        self.progressions[self.prog_cursor]
    }

    fn pop_artifact(&mut self, id: GroupId) -> bool {
        self.state.current_artifact.take_if(|(i, _)| *i == id).is_some()
    }

    fn pop_bbox(&mut self, id: GroupId) -> bool {
        self.state.bbox_stack.pop_if(|i| *i == id).is_some()
    }

    /// Find the lowest link ancestor in the tree.
    pub fn parent_link(&self) -> Option<(GroupId, &Packed<LinkMarker>)> {
        let mut current = self.current();

        while current != GroupId::INVALID {
            let group = self.groups.get(current);
            if let Some(link) = group.kind.as_link() {
                return Some((current, link));
            }
            current = group.parent;
        }

        None
    }

    /// Find the highest artifact ancestor in the tree.
    pub fn parent_artifact(&self) -> Option<ArtifactType> {
        let (_, ty) = self.state.current_artifact?;
        Some(ty)
    }

    /// Find the lowest ancestor with a bounding box in the tree.
    pub fn parent_bbox(&mut self) -> Option<&mut BBoxCtx> {
        let id = *self.state.bbox_stack.last()?;
        self.ctx.bbox_mut(&self.groups.get(id).kind)
    }

    pub fn finished_traversal(&self) -> bool {
        self.prog_cursor + 1 == self.progressions.len()
            && self.break_cursor == self.breaks.len()
    }
}

struct TreeStates {
    /// Always non-empty.
    stack: Vec<TreeState>,
}

impl TreeStates {
    fn new() -> Self {
        Self { stack: vec![TreeState::new()] }
    }

    fn push(&mut self, state: TreeState) {
        self.stack.push(state);
    }

    fn pop(&mut self) {
        self.stack.pop();
    }
}

impl std::ops::Deref for TreeStates {
    type Target = TreeState;

    fn deref(&self) -> &Self::Target {
        self.stack.last().unwrap()
    }
}

impl std::ops::DerefMut for TreeStates {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.stack.last_mut().unwrap()
    }
}

struct TreeState {
    /// The highest artifact ancestor in the tree.
    current_artifact: Option<(GroupId, ArtifactType)>,
    /// The stack of ancestors that have a [`GroupKind::bbox`].
    bbox_stack: Vec<GroupId>,
}

impl TreeState {
    fn new() -> Self {
        Self { current_artifact: None, bbox_stack: Vec::new() }
    }
}

pub trait Step {
    const KIND: StepKind;
}

pub enum StepKind {
    StartTag,
    EndTag,
    EnterLogicalChild,
    LeaveLogicalChild,
}

pub struct StartTag;
impl Step for StartTag {
    const KIND: StepKind = StepKind::StartTag;
}
pub struct EndTag;
impl Step for EndTag {
    const KIND: StepKind = StepKind::EndTag;
}
pub struct EnterLogicalChild;
impl Step for EnterLogicalChild {
    const KIND: StepKind = StepKind::EnterLogicalChild;
}
pub struct LeaveLogicalChild;
impl Step for LeaveLogicalChild {
    const KIND: StepKind = StepKind::LeaveLogicalChild;
}

pub fn step<S: Step>(tree: &mut Tree, surface: &mut Surface) {
    let prev = tree.current();
    tree.prog_cursor += 1;
    let next = tree.current();

    if prev == next {
        return;
    }

    match const { S::KIND } {
        StepKind::StartTag => {
            let next_group = tree.groups.get(next);
            if tree.state.current_artifact.is_none()
                && let Some(ty) = next_group.kind.as_artifact()
            {
                tree.state.current_artifact = Some((next, ty));
                surface.start_tagged(ContentTag::Artifact(ty));
            } else if tree.ctx.bbox(&next_group.kind).is_some() {
                tree.state.bbox_stack.push(next);
            }
        }
        StepKind::EndTag => {
            // A break can only occurr on an end tag.
            if let Some(brk) = tree.breaks.get(tree.break_cursor)
                && brk.progression_idx as usize == tree.prog_cursor
            {
                tree.break_cursor += 1;
                step_break(tree, surface, prev, next, *brk);
            } else {
                tree.pop_bbox(prev);
                close_group(tree, surface, prev);
            }
        }
        StepKind::EnterLogicalChild => {
            // This can move use to a completely different position in the tree.

            // Close any artifact in the previous location.
            if tree.state.current_artifact.is_some() {
                surface.end_tagged();
            }

            // Compute the state for the new location in the tree and push it.
            let mut current_artifact = None;
            let mut bbox_stack = Vec::new();

            let mut current = next;
            while current != GroupId::INVALID {
                let group = tree.groups.get(current);
                if let Some(ty) = group.kind.as_artifact() {
                    current_artifact = Some((current, ty));
                } else if tree.ctx.bbox(&group.kind).is_some() {
                    bbox_stack.insert(0, current);
                }
                current = group.parent;
            }

            tree.state.push(TreeState { current_artifact, bbox_stack });

            // Reopen any artifact in the next location.
            if let Some(ty) = tree.parent_artifact() {
                surface.start_tagged(ContentTag::Artifact(ty));
            }
        }
        StepKind::LeaveLogicalChild => {
            // This moves back to the previous location in the tree.

            // Logical children groups are always properly nested by
            // construction because they are frames, and thus cannot span across
            // regions. This means we can find the right logical child by just
            // walking up in the parent hierarchy.
            let mut current = prev;
            loop {
                let group = tree.groups.get(current);
                if matches!(group.kind, GroupKind::LogicalChild) {
                    close_group(tree, surface, prev);
                    break;
                }
                current = group.parent;
            }

            // Close any artifact.
            if tree.state.current_artifact.is_some() {
                surface.end_tagged();
            }

            // Just pop the state off.
            tree.state.pop();

            // Reopen any artifact.
            if let Some(ty) = tree.parent_artifact() {
                surface.start_tagged(ContentTag::Artifact(ty));
            }
        }
    }
}

fn step_break(
    tree: &mut Tree,
    surface: &mut Surface,
    prev: GroupId,
    next: GroupId,
    brk: Break,
) {
    match brk.kind {
        BreakKind::Broken { num_closed_groups } => {
            // Check the closed groups for artifacts and bounding boxes.
            let mut current = prev;
            for _ in 0..num_closed_groups {
                current = close_group(tree, surface, current);
            }

            // Check the opened groups for artifacts and bounding boxes.
            let mut new_artifact = None;
            let bbox_start = tree.state.bbox_stack.len();

            let mut current = next;
            for _ in 1..num_closed_groups {
                let group = tree.groups.get(current);
                if let GroupKind::Artifact(ty) = group.kind {
                    new_artifact = Some((current, ty));
                } else {
                    tree.state.bbox_stack.insert(bbox_start, current);
                }
                current = group.parent;
            }
            if tree.state.current_artifact.is_none()
                && let Some((_, ty)) = new_artifact
            {
                tree.state.current_artifact = new_artifact;
                surface.start_tagged(ContentTag::Artifact(ty));
            }
        }
        BreakKind::Unfinished { group_to_close } => {
            close_group(tree, surface, group_to_close);
        }
    }
}

fn close_group(tree: &mut Tree, surface: &mut Surface, id: GroupId) -> GroupId {
    tree.pop_bbox(id);

    let group = tree.groups.get(id);
    let parent = group.parent;

    match &group.kind {
        GroupKind::Root(_) => unreachable!(),
        GroupKind::Artifact(_) => {
            if tree.pop_artifact(id) {
                surface.end_tagged();
            }
        }
        GroupKind::LogicalParent(_) => {
            tree.groups.push_group(parent, id);
        }
        GroupKind::LogicalChild => {
            let parent_group = tree.groups.get_mut(parent);
            if let GroupKind::LogicalParent(children) = &mut parent_group.kind {
                children.push(id);
            } else {
                tree.groups.push_group(parent, id);
            }
        }
        GroupKind::Outline(..) => {
            tree.groups.push_group(parent, id);
        }
        GroupKind::OutlineEntry(entry, _) => {
            if let GroupKind::Outline(outline, _) = tree.groups.get(parent).kind {
                let outline_ctx = tree.ctx.outlines.get_mut(outline);
                let entry = entry.clone();
                outline_ctx.insert(&mut tree.groups, parent, entry, id);
            } else {
                tree.groups.push_group(parent, id);
            }
        }
        GroupKind::Table(table, ..) => {
            let table_ctx = tree.ctx.tables.get_mut(*table);
            context::build_table(table_ctx, &mut tree.groups, id);
            tree.groups.push_group(parent, id);
        }
        GroupKind::TableCell(cell, ..) => {
            if let GroupKind::Table(table, ..) = tree.groups.get(parent).kind {
                let table_ctx = tree.ctx.tables.get_mut(table);
                table_ctx.insert(cell, id);
            } else {
                // Avoid panicking, the nesting will be validated later.
                tree.groups.push_group(parent, id);
            }
        }
        GroupKind::Grid(grid, _) => {
            let grid_ctx = tree.ctx.grids.get(*grid);
            context::build_grid(grid_ctx, &mut tree.groups, id);
            tree.groups.push_group(parent, id);
        }
        GroupKind::GridCell(cell, _) => {
            if let GroupKind::Grid(grid, _) = tree.groups.get(parent).kind {
                let grid_ctx = tree.ctx.grids.get_mut(grid);
                grid_ctx.insert(cell, id);
            } else {
                // Avoid panicking, the nesting will be validated later.
                tree.groups.push_group(parent, id);
            }
        }
        GroupKind::List(..) => {
            tree.groups.push_group(parent, id);
        }
        GroupKind::ListItemLabel(..) => {
            let list = tree.groups.get(parent).kind.as_list().expect("parent list");
            let list_ctx = tree.ctx.lists.get_mut(list);
            list_ctx.push_label(&mut tree.groups, parent, id);
        }
        GroupKind::ListItemBody(..) => {
            let list = tree.groups.get(parent).kind.as_list().expect("parent list");
            let list_ctx = tree.ctx.lists.get_mut(list);
            list_ctx.push_body(&mut tree.groups, parent, id);
        }
        GroupKind::BibEntry(..) => {
            let list = tree.groups.get(parent).kind.as_list().expect("parent list");
            let list_ctx = tree.ctx.lists.get_mut(list);
            list_ctx.push_bib_entry(&mut tree.groups, parent, id);
        }
        GroupKind::Figure(figure, ..) => {
            context::build_figure(tree, *figure, parent, id);
        }
        GroupKind::FigureCaption(..) => {
            let parent_group = tree.groups.get_mut(parent);
            if let GroupKind::Figure(figure, _, _) = &mut parent_group.kind {
                let figure_ctx = tree.ctx.figures.get_mut(*figure);
                figure_ctx.caption = Some(id);
            } else {
                tree.groups.push_group(parent, id);
            }
        }
        GroupKind::Image(..) => {
            tree.groups.push_group(parent, id);
        }
        GroupKind::Formula(..) => {
            tree.groups.push_group(parent, id);
        }
        GroupKind::Link(..) => {
            // Wrap link in reference tag if inside an outline entry.
            let mut parent = parent;
            if let GroupKind::OutlineEntry(..) = tree.groups.get(parent).kind {
                parent = tree.groups.push_tag(parent, Tag::Reference);
            }
            tree.groups.push_group(parent, id);
        }
        GroupKind::CodeBlock(..) => {
            tree.groups.push_group(parent, id);
        }
        GroupKind::CodeBlockLine(..) => {
            // The raw element is a block, wrap each line in a BLSE, so the
            // individual lines are properly wrapped and indented when reflowed.
            let par = tree.groups.push_tag(parent, Tag::P);
            tree.groups.push_group(par, id);
        }
        GroupKind::Standard(..) => {
            tree.groups.push_group(parent, id);
        }
    };

    parent
}

#[derive(Debug)]
struct TreeBuilder<'a> {
    options: &'a PdfOptions<'a>,

    /// Each [`FrameItem::Tag`] and each [`FrameItem::Group`] with a parent
    /// will append a progression to this tree. This list of progressions is
    /// used to determine the location in the tree when doing the actual PDF
    /// generation and inserting the marked content sequences.
    progressions: Vec<GroupId>,
    breaks: Vec<Break>,
    groups: Groups,
    ctx: Ctx,

    stack: TagStack,
    logical_children: FxHashMap<Location, SmallVec<[GroupId; 4]>>,
}

#[derive(Clone, Copy, Debug)]
struct Break {
    /// The index of the progression at which point the broken up groups need to
    /// be closed.
    progression_idx: u32,
    kind: BreakKind,
}

#[derive(Clone, Copy, Debug)]
enum BreakKind {
    /// Marks a point where the entries on the stack had to be broken up.
    Broken {
        /// The number of groups which have to be closed, from the current group
        /// upwards in the parent hierarchy.
        num_closed_groups: u16,
    },
    /// Marks a point where there was an uninished stack inside a grid/table
    /// cell, which was transfered to the next logical child.
    Unfinished { group_to_close: GroupId },
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

            stack: TagStack::new(),
            logical_children: FxHashMap::default(),
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
    pub fn take_unfinished_stack(
        &mut self,
        idx: usize,
    ) -> Option<std::vec::Drain<'_, StackEntry>> {
        if idx + 1 < self.items.len() { Some(self.items.drain(idx + 1..)) } else { None }
    }
}

#[derive(Debug)]
pub struct StackEntry {
    /// The location of the stack entry. If this is `None` the stack entry has
    /// to be manually popped.
    pub loc: Option<Location>,
    pub id: GroupId,
}

pub fn build(document: &PagedDocument, options: &PdfOptions) -> SourceResult<Tree> {
    let mut tree = TreeBuilder::new(document, options);
    for page in document.pages.iter() {
        visit_frame(&mut tree, &page.frame)?;
    }

    assert!(tree.stack.is_empty(), "tags weren't properly closed");
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
            let validator = options.standards.config.validator();
            let validator = validator.as_str();
            let group = tree.groups.get(located.id);
            bail!(
                group.span,
                "{validator} error: ambigous logical parent";
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

    Ok(Tree {
        prog_cursor: 0,
        progressions: tree.progressions,
        break_cursor: 0,
        breaks: tree.breaks,
        state: TreeStates::new(),
        groups: tree.groups,
        ctx: tree.ctx,
    })
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
    if let Some(stack) = tree.groups.take_unfinished_stack(parent_loc) {
        tree.stack.extend(stack);
    }
    // Move to the top of the stack, including the pushed on unfinished stack.
    tree.progressions.push(tree.stack.last().unwrap().id);

    visit_frame(tree, &group.frame)?;

    if let Some(stack) = tree.stack.take_unfinished_stack(stack_idx) {
        tree.groups.store_unfinished_stack(parent_loc, stack.collect());
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
            let tag = Tag::TD.into();
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
        push_stack(tree, elem, GroupKind::LogicalParent(SmallVec::new()))
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
            push_stack(tree, elem, GroupKind::LogicalParent(SmallVec::new()))
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
    push_stack(tree, elem, GroupKind::Standard(tag.into(), None))
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
    let entry = &tree.stack[stack_idx];
    let group = tree.groups.get(entry.id);
    if matches!(group.kind, GroupKind::TableCell(..) | GroupKind::GridCell(..)) {
        tree.insert_break(BreakKind::Unfinished { group_to_close: entry.id });
        if let Some(stack) = tree.stack.take_unfinished_stack(stack_idx) {
            tree.groups.store_unfinished_stack(loc, stack.collect());
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
        if group.kind.is_breakable(tree.options.is_pdf_ua()) {
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
            let validator = tree.options.standards.config.validator();
            let validator = validator.as_str();
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
                hint: "disable tagged pdf by passing `--disable-pdf-tags`"
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
