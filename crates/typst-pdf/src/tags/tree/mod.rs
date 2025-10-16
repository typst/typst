use crate::PdfOptions;
use crate::tags::GroupId;
use crate::tags::context::{self, BBoxCtx, BBoxId, Ctx};
use crate::tags::groups::{Group, GroupKind, Groups, InternalGridCellKind};
use crate::tags::tree::build::TreeBuilder;
use crate::tags::tree::text::TextAttrs;
use ecow::EcoVec;
use krilla::surface::Surface;
use krilla::tagging::{ArtifactType, ContentTag, Tag};
use rustc_hash::FxHashMap;
use smallvec::SmallVec;
use typst_library::diag::SourceDiagnostic;
use typst_library::foundations::Packed;
use typst_library::introspection::Location;
use typst_library::layout::{Inherit, PagedDocument};
use typst_library::model::LinkMarker;

pub use build::build;
pub use text::{ResolvedTextAttrs, TextAttr, resolve_text_attrs};

mod build;
mod text;

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
    logical_children: FxHashMap<Location, SmallVec<[GroupId; 4]>>,
    pub errors: EcoVec<SourceDiagnostic>,
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
    /// The stack of text attributes.
    text_attrs: TextAttrs,
}

impl TraversalState {
    fn new() -> Self {
        Self {
            current_artifact: None,
            bbox_stack: Vec::new(),
            text_attrs: TextAttrs::new(),
        }
    }

    /// Update the traversal state when moving out of a group.
    fn pop_group(&mut self, surface: &mut Surface, id: GroupId, group: &Group) {
        if self.current_artifact.take_if(|(i, _)| *i == id).is_some() {
            surface.end_tagged();
        }
        if let Some(id) = group.kind.bbox() {
            self.bbox_stack.pop_if(|i| *i == id);
        }
        self.text_attrs.pop(id);
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
        open_group(&tree.groups, &mut tree.state, surface, next);
    }
}

pub fn step_end_tag(tree: &mut Tree, surface: &mut Surface) {
    let Some((prev, next)) = step(tree) else { return };

    if let Some(brk) = consume_break(tree) {
        step_break(tree, surface, prev, next, brk);
    } else if let Some(unfinished) = consume_unfinished(tree) {
        // In logical children the whole traversal state is popped off the
        // stack. For grid cells we're still in the same traversal state, so we
        // need to update it accordingly. The groups can't be closed since they
        // will be closed later, so we just update the state since we've still
        // moved out of them.
        let mut current = prev;
        while current != unfinished.group_to_close {
            let group = tree.groups.get(current);
            tree.state.pop_group(surface, current, group);
            current = group.parent;
        }

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
    let mut new_state = TraversalState::new();
    let mut current = next;
    let rev_iter = std::iter::from_fn(|| {
        if current == GroupId::INVALID {
            return None;
        }
        let id = current;
        let group = tree.groups.get(id);
        current = group.parent;
        Some((id, group))
    });
    open_multiple_groups(&mut new_state, surface, rev_iter);

    tree.state.push(new_state);
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
    // Close groups.
    let mut current = prev;
    for _ in 0..brk.num_closed {
        current = close_group(tree, surface, current);
    }

    // Open groups.
    let mut current = next;
    let rev_iter = std::iter::from_fn(|| {
        let id = current;
        let group = tree.groups.get(id);
        current = group.parent;
        Some((id, group))
    });
    open_multiple_groups(
        &mut tree.state,
        surface,
        rev_iter.take(brk.num_opened as usize),
    );
}

fn open_group(
    groups: &Groups,
    state: &mut TraversalState,
    surface: &mut Surface,
    id: GroupId,
) {
    let group = groups.get(id);
    if state.current_artifact.is_none()
        && let Some(ty) = group.kind.as_artifact()
    {
        state.current_artifact = Some((id, ty));
        surface.start_tagged(ContentTag::Artifact(ty));
    }
    if let Some(bbox) = &group.kind.bbox() {
        state.bbox_stack.push(*bbox);
    }
    if let GroupKind::TextAttr(attr) = &group.kind {
        state.text_attrs.push(id, attr.clone());
    }
}

/// Since the groups need to be opened in order, but we can only iterate the
/// parent hierarchy from bottom to top, this cannot simply call [`open_group`].
fn open_multiple_groups<'a>(
    state: &mut TraversalState,
    surface: &mut Surface,
    rev_iter: impl Iterator<Item = (GroupId, &'a Group)>,
) {
    let mut new_artifact = None;
    let bbox_start = state.bbox_stack.len();
    let text_attr_start = state.text_attrs.len();

    for (id, group) in rev_iter {
        if let Some(ty) = group.kind.as_artifact() {
            new_artifact = Some((id, ty));
        }
        if let Some(bbox) = group.kind.bbox() {
            state.bbox_stack.insert(bbox_start, bbox);
        }
        if let GroupKind::TextAttr(attr) = &group.kind {
            state.text_attrs.insert(text_attr_start, id, attr.clone());
        }
    }

    if state.current_artifact.is_none()
        && let Some((_, ty)) = new_artifact
    {
        state.current_artifact = new_artifact;
        surface.start_tagged(ContentTag::Artifact(ty));
    }
}

fn close_group(tree: &mut Tree, surface: &mut Surface, id: GroupId) -> GroupId {
    let group = tree.groups.get(id);
    let direct_parent = group.parent;
    let semantic_parent = semantic_parent(tree, direct_parent);

    tree.state.pop_group(surface, id, group);

    match &group.kind {
        GroupKind::Root(_) => unreachable!(),
        GroupKind::Artifact(_) => {
            tree.groups.push_group(direct_parent, id);
        }
        GroupKind::LogicalParent(elem) => {
            let loc = elem.location().unwrap();
            // Insert logical children when closing the logical parent, so they
            // are at the end of the group.
            if let Some(children) = tree.logical_children.get(&loc) {
                tree.groups.extend_groups(id, children.iter().copied());
            }
            tree.groups.push_group(direct_parent, id);
        }
        GroupKind::LogicalChild(inherit, logical_parent) => {
            // `GroupKind::LogicalParent` handles inserting of children at its
            // end, see above. Children of table/grid cells are always ordered
            // correctly and are treated a little bit differently
            if tree.groups.get(semantic_parent).kind.is_grid_layout_cell() {
                tree.groups.push_group(direct_parent, id);
            } else if *inherit == Inherit::No {
                let logical_parent_is_in_artifact = 'artifact: {
                    let mut current = tree.groups.get(*logical_parent).parent;
                    while current != GroupId::INVALID {
                        let group = tree.groups.get(current);
                        if group.kind.is_artifact() {
                            break 'artifact true;
                        }
                        current = group.parent;
                    }
                    false
                };

                // If this logical child is of kind `LogicalChildKind::Insert`
                // and not inside of an artifact, inserting it into a parent
                // that is inside of an artifact would mean the content will be
                // discarded. If that's the case, ignore the logical parent
                // structure and insert it wherever it appeared in the frame
                // tree.
                if tree.parent_artifact().is_none() && logical_parent_is_in_artifact {
                    tree.groups.push_group(direct_parent, id);
                }
            }
        }
        GroupKind::Outline(..) => {
            tree.groups.push_group(direct_parent, id);
        }
        GroupKind::OutlineEntry(entry, _) => {
            if let GroupKind::Outline(outline, _) = tree.groups.get(semantic_parent).kind
            {
                let outline_ctx = tree.ctx.outlines.get_mut(outline);
                let entry = entry.clone();
                tree.groups.get_mut(id).parent = semantic_parent;
                outline_ctx.insert(&mut tree.groups, semantic_parent, entry, id);
            } else {
                tree.groups.push_group(direct_parent, id);
            }
        }
        GroupKind::Table(table, ..) => {
            context::build_table(tree, *table, id);
            tree.groups.push_group(direct_parent, id);
        }
        &GroupKind::TableCell(ref cell, tag, _) => {
            let cell = cell.clone();
            if let Some(table) = move_into(tree, semantic_parent, id, GroupKind::as_table)
            {
                let table_ctx = tree.ctx.tables.get_mut(table);
                table_ctx.insert(&cell, tag, id);
            } else {
                // Avoid panicking, the nesting will be validated later.
                tree.groups.push_group(direct_parent, id);
            }
        }
        GroupKind::Grid(grid, _) => {
            let grid_ctx = tree.ctx.grids.get(*grid);
            context::build_grid(grid_ctx, &mut tree.groups, id);
            tree.groups.push_group(direct_parent, id);
        }
        GroupKind::GridCell(cell, _) => {
            let cell = cell.clone();
            if let Some(grid) = move_into(tree, semantic_parent, id, GroupKind::as_grid) {
                let grid_ctx = tree.ctx.grids.get_mut(grid);
                grid_ctx.insert(&cell, id);
            } else {
                // Avoid panicking, the nesting will be validated later.
                tree.groups.push_group(direct_parent, id);
            }
        }
        GroupKind::InternalGridCell(internal) => {
            // Replace with the actual group kind.
            tree.groups.get_mut(id).kind = internal.to_kind();
            tree.groups.push_group(direct_parent, id);
        }
        GroupKind::List(..) => {
            tree.groups.push_group(direct_parent, id);
        }
        GroupKind::ListItemLabel(..) => {
            if let Some(list) = move_into(tree, semantic_parent, id, GroupKind::as_list) {
                let list_ctx = tree.ctx.lists.get_mut(list);
                list_ctx.push_label(&mut tree.groups, semantic_parent, id);
            } else {
                // Avoid panicking, the nesting will be validated later.
                tree.groups.push_group(direct_parent, id);
            }
        }
        GroupKind::ListItemBody(..) => {
            if let Some(list) = move_into(tree, semantic_parent, id, GroupKind::as_list) {
                let list_ctx = tree.ctx.lists.get_mut(list);
                list_ctx.push_body(&mut tree.groups, semantic_parent, id);
            } else {
                // Avoid panicking, the nesting will be validated later.
                tree.groups.push_group(direct_parent, id);
            }
        }
        GroupKind::TermsItemLabel(..) => {
            if let GroupKind::TermsItemBody(lbl, _) =
                &mut tree.groups.get_mut(semantic_parent).kind
            {
                *lbl = Some(id);
            } else {
                // Avoid panicking, the nesting will be validated later.
                tree.groups.push_group(direct_parent, id);
            }
        }
        &GroupKind::TermsItemBody(lbl, ..) => {
            if let Some(list) = move_into(tree, semantic_parent, id, GroupKind::as_list) {
                let list_ctx = tree.ctx.lists.get_mut(list);
                if let Some(lbl) = lbl {
                    tree.groups.get_mut(lbl).parent = semantic_parent;
                    list_ctx.push_label(&mut tree.groups, semantic_parent, lbl);
                }
                list_ctx.push_body(&mut tree.groups, semantic_parent, id);
            } else {
                // Avoid panicking, the nesting will be validated later.
                tree.groups.push_group(direct_parent, id);
            }
        }
        GroupKind::BibEntry(..) => {
            if let Some(list) = move_into(tree, semantic_parent, id, GroupKind::as_list) {
                let list_ctx = tree.ctx.lists.get_mut(list);
                list_ctx.push_bib_entry(&mut tree.groups, semantic_parent, id);
            } else {
                // Avoid panicking, the nesting will be validated later.
                tree.groups.push_group(direct_parent, id);
            }
        }
        GroupKind::Figure(figure, ..) => {
            context::build_figure(tree, *figure, direct_parent, id);
        }
        GroupKind::FigureCaption(..) => {
            if let GroupKind::Figure(figure, ..) = tree.groups.get(semantic_parent).kind {
                let figure_ctx = tree.ctx.figures.get_mut(figure);
                figure_ctx.caption = Some(id);
            } else {
                tree.groups.push_group(direct_parent, id);
            }
        }
        GroupKind::Image(..) => {
            tree.groups.push_group(direct_parent, id);
        }
        GroupKind::Formula(..) => {
            tree.groups.push_group(direct_parent, id);
        }
        GroupKind::Link(..) => {
            // Wrap link in reference tag if inside an outline entry.
            let mut parent = direct_parent;
            if let GroupKind::OutlineEntry(..) = tree.groups.get(direct_parent).kind {
                parent = tree.groups.push_tag(parent, Tag::Reference);
            }
            tree.groups.push_group(parent, id);
        }
        GroupKind::CodeBlock(..) => {
            tree.groups.push_group(direct_parent, id);
        }
        GroupKind::CodeBlockLine(..) => {
            tree.groups.push_group(direct_parent, id);
        }
        GroupKind::Par(..) => {
            tree.groups.push_group(direct_parent, id);
        }
        GroupKind::TextAttr(..) => {
            tree.groups.push_group(direct_parent, id);
        }
        GroupKind::Transparent => {
            tree.groups.push_group(direct_parent, id);
        }
        GroupKind::Standard(..) => {
            tree.groups.push_group(direct_parent, id);
        }
    };

    direct_parent
}

fn move_into<T>(
    tree: &mut Tree,
    semantic_parent: GroupId,
    child: GroupId,
    f: impl FnOnce(&GroupKind) -> Option<T>,
) -> Option<T> {
    let res = f(&tree.groups.get(semantic_parent).kind);
    if res.is_some() {
        tree.groups.get_mut(child).parent = semantic_parent;
    }
    res
}

fn semantic_parent(tree: &Tree, direct_parent: GroupId) -> GroupId {
    let mut parent = direct_parent;
    loop {
        let group = tree.groups.get(parent);
        // While paragraphs, do have a semantic meaning, they are automatically
        // generated and may interfere with other more strongly structured
        // nesting groups. For example the `TermsItemLabel` might be wrapped by
        // a paragraph, out of which it is moved into the parent `LI`.
        let non_semantic = matches!(
            group.kind,
            GroupKind::InternalGridCell(InternalGridCellKind::Transparent)
                | GroupKind::Par(_)
                | GroupKind::TextAttr(_)
                | GroupKind::Transparent
        );
        if !non_semantic {
            return parent;
        }

        parent = group.parent;
    }
}
