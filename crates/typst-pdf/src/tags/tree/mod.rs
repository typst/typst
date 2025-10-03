use crate::PdfOptions;
use crate::tags::GroupId;
use crate::tags::context::{self, BBoxCtx, BBoxId, Ctx};
use crate::tags::groups::{GroupKind, Groups};
use crate::tags::tree::build::TreeBuilder;
use krilla::surface::Surface;
use krilla::tagging::{ArtifactType, ContentTag, Tag};
use rustc_hash::FxHashMap;
use smallvec::SmallVec;
use typst_library::foundations::Packed;
use typst_library::introspection::Location;
use typst_library::layout::PagedDocument;
use typst_library::model::LinkMarker;

pub use build::build;

mod build;

pub struct Tree {
    /// Points at the current group in the `progressions` list.
    prog_cursor: usize,
    progressions: Vec<GroupId>,
    /// Points at the next break in the `breaks` list.
    break_cursor: usize,
    breaks: Vec<Break>,
    /// Points at the next intem in the `unfinished` list.
    unfinished_cursor: usize,
    unfinished: Vec<Unfinished>,
    state: TraversalStates,
    pub groups: Groups,
    pub ctx: Ctx,
    pub logical_children: FxHashMap<Location, SmallVec<[GroupId; 4]>>,
}

impl Tree {
    pub fn empty(document: &PagedDocument, options: &PdfOptions) -> Self {
        TreeBuilder::new(document, options).finish()
    }

    pub fn current(&self) -> GroupId {
        self.progressions[self.prog_cursor]
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
        Some(self.ctx.bboxes.get_mut(id))
    }

    pub fn assert_finished_traversal(&self) {
        assert_eq!(
            self.prog_cursor + 1,
            self.progressions.len(),
            "tree traversal didn't complete properly"
        );
        assert_eq!(
            self.break_cursor,
            self.breaks.len(),
            "tree traversal didn't complete properly"
        );
        assert_eq!(
            self.unfinished_cursor,
            self.unfinished.len(),
            "tree traversal didn't complete properly"
        );
    }
}

/// A stack of traversal states, the topmost entry represents the current state
/// in the tree. New stack entries are pushed on when entering a logical child
/// and popped off when leaving one.
struct TraversalStates {
    /// Always non-empty.
    stack: Vec<TraversalState>,
}

impl TraversalStates {
    fn new() -> Self {
        Self { stack: vec![TraversalState::new()] }
    }

    fn push(&mut self, state: TraversalState) {
        self.stack.push(state);
    }

    fn pop(&mut self) {
        self.stack.pop();
    }
}

impl std::ops::Deref for TraversalStates {
    type Target = TraversalState;

    fn deref(&self) -> &Self::Target {
        self.stack.last().unwrap()
    }
}

impl std::ops::DerefMut for TraversalStates {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.stack.last_mut().unwrap()
    }
}

/// Stores frequently accessed properties about the current tree traversal
/// position. This is an optimization to avoid searching all ancestors; instead
/// this is updated on each step.
struct TraversalState {
    /// The highest artifact ancestor in the tree.
    current_artifact: Option<(GroupId, ArtifactType)>,
    /// The stack of ancestors that have a [`GroupKind::bbox`].
    bbox_stack: Vec<BBoxId>,
}

impl TraversalState {
    fn new() -> Self {
        Self { current_artifact: None, bbox_stack: Vec::new() }
    }

    fn pop_artifact(&mut self, id: GroupId) -> bool {
        self.current_artifact.take_if(|(i, _)| *i == id).is_some()
    }

    fn pop_bbox(&mut self, id: BBoxId) {
        self.bbox_stack.pop_if(|i| *i == id);
    }
}

/// Marks a point where the entries on the stack were split up.
#[derive(Debug, Copy, Clone)]
struct Break {
    /// The index of the progression at which point the broken up groups need to
    /// be closed.
    prog_idx: u32,
    /// The number of groups which have to be closed, from the previous
    /// group upwards in the parent hierarchy.
    num_closed: u16,
    /// The number of groups which have to be closed, from the next group
    /// upwards in the parent hierarchy.
    num_opened: u16,
}

/// Marks a point at the end of a logical child or parent where the stack was
/// not fully closed, and the open groups were handled in the next logical
/// child.
#[derive(Debug, Copy, Clone)]
struct Unfinished {
    /// The index of the progression at which point the broken up groups need to
    /// be closed.
    prog_idx: u32,
    group_to_close: GroupId,
}

pub fn step_start_tag(tree: &mut Tree, surface: &mut Surface) {
    let Some((prev, next)) = step(tree) else { return };

    if let Some(brk) = consume_break(tree) {
        step_break(tree, surface, prev, next, brk);
    } else {
        let next_group = tree.groups.get(next);
        if tree.state.current_artifact.is_none()
            && let Some(ty) = next_group.kind.as_artifact()
        {
            tree.state.current_artifact = Some((next, ty));
            surface.start_tagged(ContentTag::Artifact(ty));
        } else if let Some(id) = &next_group.kind.bbox() {
            tree.state.bbox_stack.push(*id);
        }
    }
}

pub fn step_end_tag(tree: &mut Tree, surface: &mut Surface) {
    let Some((prev, next)) = step(tree) else { return };

    if let Some(brk) = consume_break(tree) {
        step_break(tree, surface, prev, next, brk);
    } else if let Some(unfinished) = consume_unfinished(tree) {
        close_group(tree, surface, unfinished.group_to_close);
    } else {
        close_group(tree, surface, prev);
    }
}

/// This can move to a completely different position in the tree.
pub fn enter_logical_child(tree: &mut Tree, surface: &mut Surface) {
    let Some((_, next)) = step(tree) else { return };

    // Close any artifact in the previous location.
    if tree.parent_artifact().is_some() {
        surface.end_tagged();
    }

    // Compute the traversal state for the new location in the tree and push it.
    let mut current_artifact = None;
    let mut bbox_stack = Vec::new();

    let mut current = next;
    while current != GroupId::INVALID {
        let group = tree.groups.get(current);
        if let Some(ty) = group.kind.as_artifact() {
            current_artifact = Some((current, ty));
        } else if let Some(id) = group.kind.bbox() {
            bbox_stack.insert(0, id);
        }
        current = group.parent;
    }

    tree.state.push(TraversalState { current_artifact, bbox_stack });

    // Reopen any artifact in the logical child.
    if let Some(ty) = tree.parent_artifact() {
        surface.start_tagged(ContentTag::Artifact(ty));
    }
}

/// This moves back to the previous location in the tree.
pub fn leave_logical_child(tree: &mut Tree, surface: &mut Surface) {
    let Some((prev, _)) = step(tree) else { return };

    // The stack within a logical child, could also be unfinished, in
    // which case a `BreakKind::Unfinished` is inserted to close the
    // `LogicalChild` group.
    if let Some(unfinished) = consume_unfinished(tree) {
        close_group(tree, surface, unfinished.group_to_close);
    } else {
        close_group(tree, surface, prev);
    }

    // Close any artifact in the logical child.
    if tree.parent_artifact().is_some() {
        surface.end_tagged();
    }

    tree.state.pop();

    // Reopen any artifact in the restored location of the tree.
    if let Some(ty) = tree.parent_artifact() {
        surface.start_tagged(ContentTag::Artifact(ty));
    }
}

fn step(tree: &mut Tree) -> Option<(GroupId, GroupId)> {
    let prev = tree.current();
    tree.prog_cursor += 1;
    let next = tree.current();

    // We didn't move into a new group, no actions are necessary.
    if prev == next {
        return None;
    }

    Some((prev, next))
}

fn consume_break(tree: &mut Tree) -> Option<Break> {
    let brk = *tree.breaks.get(tree.break_cursor)?;
    if brk.prog_idx as usize == tree.prog_cursor {
        tree.break_cursor += 1;
        return Some(brk);
    }
    None
}

fn consume_unfinished(tree: &mut Tree) -> Option<Unfinished> {
    let unfinished = *tree.unfinished.get(tree.unfinished_cursor)?;
    if unfinished.prog_idx as usize == tree.prog_cursor {
        tree.unfinished_cursor += 1;
        return Some(unfinished);
    }
    None
}

fn step_break(
    tree: &mut Tree,
    surface: &mut Surface,
    prev: GroupId,
    next: GroupId,
    brk: Break,
) {
    // Check the closed groups for artifacts and bounding boxes.
    let mut current = prev;
    for _ in 0..brk.num_closed {
        current = close_group(tree, surface, current);
    }

    // Check the opened groups for artifacts and bounding boxes.
    let mut new_artifact = None;
    let bbox_start = tree.state.bbox_stack.len();

    let mut current = next;
    for _ in 0..brk.num_opened {
        let group = tree.groups.get(current);
        if let GroupKind::Artifact(ty) = group.kind {
            new_artifact = Some((current, ty));
        } else if let Some(id) = group.kind.bbox() {
            tree.state.bbox_stack.insert(bbox_start, id);
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

fn close_group(tree: &mut Tree, surface: &mut Surface, id: GroupId) -> GroupId {
    let group = tree.groups.get(id);
    let parent = group.parent;

    if let Some(id) = group.kind.bbox() {
        tree.state.pop_bbox(id);
    }

    match &group.kind {
        GroupKind::Root(_) => unreachable!(),
        GroupKind::Artifact(_) => {
            if tree.state.pop_artifact(id) {
                surface.end_tagged();
            }
        }
        GroupKind::LogicalParent(elem) => {
            let loc = elem.location().unwrap();
            // Insert logical children when closing the logical parent, so they
            // are at the end of the group.
            if let Some(children) = tree.logical_children.get(&loc) {
                tree.groups.extend_groups(id, children.iter().copied());
            }
            tree.groups.push_group(parent, id);
        }
        GroupKind::LogicalChild => {
            if let GroupKind::LogicalParent(_) = tree.groups.get(parent).kind {
                // `GroupKind::LogicalParent` handles inserting of children at
                // its end, see above.
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
            context::build_table(tree, *table, id);
            tree.groups.push_group(parent, id);
        }
        GroupKind::TableCell(cell, tag, _) => {
            if let GroupKind::Table(table, _, _) = tree.groups.get(parent).kind {
                let table_ctx = tree.ctx.tables.get_mut(table);
                table_ctx.insert(cell, *tag, id);
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
        GroupKind::TermsItemLabel(..) => {
            let parent_group = tree.groups.get_mut(parent);
            let grand_parent = parent_group.parent;
            // Move the terms label out of the body.
            if let GroupKind::TermsItemBody(lbl, _) = &mut parent_group.kind {
                *lbl = Some(id);
            // The terms body might contain a paragraph, so check if the grand
            // parent is a terms body.
            } else if let GroupKind::Par(..) = parent_group.kind
                && let GroupKind::TermsItemBody(lbl, _) =
                    &mut tree.groups.get_mut(grand_parent).kind
            {
                *lbl = Some(id);
            } else {
                tree.groups.push_group(parent, id);
            }
        }
        &GroupKind::TermsItemBody(lbl, ..) => {
            let list = tree.groups.get(parent).kind.as_list().expect("parent list");
            let list_ctx = tree.ctx.lists.get_mut(list);
            if let Some(lbl) = lbl {
                tree.groups.get_mut(lbl).parent = parent;
                list_ctx.push_label(&mut tree.groups, parent, lbl);
            }
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
            tree.groups.push_group(parent, id);
        }
        GroupKind::Par(..) => {
            tree.groups.push_group(parent, id);
        }
        GroupKind::Standard(..) => {
            tree.groups.push_group(parent, id);
        }
    };

    parent
}
