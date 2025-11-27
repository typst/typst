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
use std::ops::ControlFlow;

use ecow::EcoVec;
use krilla::tagging::{ArtifactType, ListNumbering, Tag, TagKind};
use rustc_hash::FxHashMap;
use smallvec::SmallVec;
use typst_library::diag::{
    At, ExpectInternal, SourceDiagnostic, SourceResult, assert_internal, bail, error,
    panic_internal,
};
use typst_library::foundations::{Content, ContextElem};
use typst_library::introspection::Location;
use typst_library::layout::{
    Frame, FrameItem, FrameParent, GridCell, GridElem, GroupItem, HideElem, Inherit,
    PagedDocument, PlaceElem, RepeatElem,
};
use typst_library::math::EquationElem;
use typst_library::model::{
    EmphElem, EnumElem, FigureCaption, FigureElem, FootnoteElem, FootnoteEntry,
    HeadingElem, LinkMarker, ListElem, Outlinable, OutlineEntry, ParElem, QuoteElem,
    StrongElem, TableCell, TableElem, TermsElem, TitleElem,
};
use typst_library::pdf::{ArtifactElem, PdfMarkerTag, PdfMarkerTagKind};
use typst_library::text::{
    HighlightElem, OverlineElem, RawElem, RawLine, StrikeElem, SubElem, SuperElem,
    UnderlineElem,
};
use typst_library::visualize::ImageElem;
use typst_syntax::Span;

use crate::PdfOptions;
use crate::tags::GroupId;
use crate::tags::context::{Ctx, FigureCtx, GridCtx, ListCtx, OutlineCtx, TableCtx};
use crate::tags::groups::{BreakOpportunity, BreakPriority, GroupKind, Groups};
use crate::tags::tree::text::TextAttr;
use crate::tags::tree::{Break, TraversalStates, Tree, Unfinished};
use crate::tags::util::{ArtifactKindExt, PropertyValCopied};

pub struct TreeBuilder<'a> {
    options: &'a PdfOptions<'a>,

    /// Each [`FrameItem::Tag`] and each [`FrameItem::Group`] with a parent
    /// will append a progression to this tree. This list of progressions is
    /// used to determine the location in the tree when doing the actual PDF
    /// generation and inserting the marked content sequences.
    progressions: Vec<GroupId>,
    breaks: Vec<Break>,
    unfinished: Vec<Unfinished>,
    groups: Groups,
    ctx: Ctx,
    logical_children: FxHashMap<Location, SmallVec<[GroupId; 4]>>,
    errors: EcoVec<SourceDiagnostic>,

    stack: TagStack,
    /// Currently only used for table/grid cells that are broken across multiple
    /// regions, and thus can have opening/closing introspection tags that are
    /// in completely different frames, due to the logical parenting mechanism.
    unfinished_stacks: FxHashMap<Location, Vec<StackEntry>>,
}

impl<'a> TreeBuilder<'a> {
    pub fn new(document: &PagedDocument, options: &'a PdfOptions) -> Self {
        let doc_lang = document.info.locale.custom();
        let mut groups = Groups::new();
        let doc = groups.new_virtual(
            GroupId::INVALID,
            Span::detached(),
            GroupKind::Root(doc_lang),
        );
        Self {
            options,
            progressions: vec![doc],
            breaks: Vec::new(),
            unfinished: Vec::new(),
            groups,
            ctx: Ctx::new(),
            logical_children: FxHashMap::default(),
            errors: EcoVec::new(),

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
            unfinished_cursor: 0,
            unfinished: self.unfinished,
            state: TraversalStates::new(),
            groups: self.groups,
            ctx: self.ctx,
            logical_children: self.logical_children,
            errors: self.errors,
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
}

#[derive(Debug)]
struct TagStack {
    items: Vec<StackEntry>,
}

impl std::ops::Deref for TagStack {
    type Target = Vec<StackEntry>;

    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

impl std::ops::DerefMut for TagStack {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.items
    }
}

impl TagStack {
    fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Remove all stack entries after the idx.
    fn take_unfinished_stack(&mut self, idx: usize) -> Option<Vec<StackEntry>> {
        if idx + 1 < self.items.len() {
            Some(self.items.drain(idx + 1..).collect())
        } else {
            None
        }
    }
}

#[derive(Debug, Copy, Clone)]
struct StackEntry {
    /// The location of the stack entry. If this is `None` the stack entry has
    /// to be manually popped.
    loc: Option<Location>,
    id: GroupId,
    prog_idx: u32,
}

pub fn build(document: &PagedDocument, options: &PdfOptions) -> SourceResult<Tree> {
    let mut tree = TreeBuilder::new(document, options);
    for page in document.pages.iter() {
        visit_frame(&mut tree, &page.frame)?;
    }

    if let Some(last) = tree.stack.last() {
        panic_internal("tags weren't properly closed")
            .at(tree.groups.get(last.id).span)?;
    }
    assert_internal(tree.unfinished_stacks.is_empty(), "tags weren't properly closed")
        .at(Span::detached())?;
    assert_internal(
        tree.progressions.first() == tree.progressions.last(),
        "tags weren't properly closed",
    )
    .at(Span::detached())?;

    // Insert logical children into the tree.
    #[allow(clippy::iter_over_hash_type)]
    for (loc, children) in tree.logical_children.iter() {
        let located = (tree.groups.by_loc(loc))
            .expect_internal("parent group")
            .at(Span::detached())?;

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
            let child = tree.groups.get_mut(*child);

            let GroupKind::LogicalChild(inherit, logical_parent) = &mut child.kind else {
                unreachable!()
            };
            *logical_parent = located.id;

            // Move the child into its logical parent, so artifact, bbox, and
            // text attributes are inherited.
            if *inherit == Inherit::Yes {
                child.parent = located.id;
            }
        }
    }

    #[cfg(debug_assertions)]
    for group in tree.groups.list.iter().skip(1) {
        assert_ne!(group.parent, GroupId::INVALID);
    }

    Ok(tree.finish())
}

fn visit_frame(tree: &mut TreeBuilder, frame: &Frame) -> SourceResult<()> {
    for (_, item) in frame.items() {
        match item {
            FrameItem::Group(group) => visit_group_frame(tree, group)?,
            FrameItem::Tag(typst_library::introspection::Tag::Start(elem, flags)) => {
                if flags.tagged {
                    visit_start_tag(tree, elem);
                }
            }
            FrameItem::Tag(typst_library::introspection::Tag::End(loc, _, flags)) => {
                if flags.tagged {
                    visit_end_tag(tree, *loc)?;
                }
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
    let Some(parent) = group.parent else {
        return visit_frame(tree, &group.frame);
    };

    // Push the logical child.
    let prev = tree.current();
    let stack_idx = tree.stack.len();
    let id = push_logical_child(tree, parent);
    tree.progressions.push(id);

    // Handle the group frame.
    visit_frame(tree, &group.frame)?;

    // Pop logical child.
    pop_logical_child(tree, parent, stack_idx);
    tree.progressions.push(prev);

    Ok(())
}

fn push_logical_child(tree: &mut TreeBuilder, parent: FrameParent) -> GroupId {
    let id = tree.groups.new_virtual(
        match parent.inherit {
            Inherit::Yes => GroupId::INVALID,
            Inherit::No => tree.current(),
        },
        Span::detached(),
        GroupKind::LogicalChild(parent.inherit, GroupId::INVALID),
    );

    tree.logical_children.entry(parent.location).or_default().push(id);

    push_stack_entry(tree, None, id);
    if let Some(stack) = tree.unfinished_stacks.remove(&parent.location) {
        tree.stack.extend(stack);
    }
    // Move to the top of the stack, including the pushed on unfinished stack.
    tree.stack.last().unwrap().id
}

fn pop_logical_child(tree: &mut TreeBuilder, parent: FrameParent, stack_idx: usize) {
    if let Some(stack) = tree.stack.take_unfinished_stack(stack_idx) {
        tree.unfinished_stacks.insert(parent.location, stack);
        tree.unfinished.push(Unfinished {
            prog_idx: tree.progressions.len() as u32,
            group_to_close: tree.stack[stack_idx].id,
        });
    }
    tree.stack.pop().expect("stack entry");
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
    // Artifacts
    #[allow(clippy::redundant_pattern_matching)]
    if let Some(_) = elem.to_packed::<HideElem>() {
        push_artifact(tree, elem, ArtifactType::Other)
    } else if let Some(artifact) = elem.to_packed::<ArtifactElem>() {
        let kind = artifact.kind.val();
        push_artifact(tree, elem, kind.to_krilla())
    } else if let Some(_) = elem.to_packed::<RepeatElem>() {
        push_artifact(tree, elem, ArtifactType::Other)

    // Elements
    } else if let Some(tag) = elem.to_packed::<PdfMarkerTag>() {
        match &tag.kind {
            PdfMarkerTagKind::OutlineBody => {
                let id = tree.ctx.outlines.push(OutlineCtx::new());
                push_group(tree, elem, GroupKind::Outline(id, None))
            }
            PdfMarkerTagKind::Bibliography(numbered) => {
                let numbering =
                    if *numbered { ListNumbering::Decimal } else { ListNumbering::None };
                let id = tree.ctx.lists.push(ListCtx::new());
                push_group(tree, elem, GroupKind::List(id, numbering, None))
            }
            PdfMarkerTagKind::BibEntry => {
                push_group(tree, elem, GroupKind::BibEntry(None))
            }
            PdfMarkerTagKind::ListItemLabel => {
                push_group(tree, elem, GroupKind::ListItemLabel(None))
            }
            PdfMarkerTagKind::ListItemBody => {
                push_group(tree, elem, GroupKind::ListItemBody(None))
            }
            PdfMarkerTagKind::TermsItemLabel => {
                push_group(tree, elem, GroupKind::TermsItemLabel(None))
            }
            PdfMarkerTagKind::TermsItemBody => {
                push_group(tree, elem, GroupKind::TermsItemBody(None, None))
            }
            PdfMarkerTagKind::Label => push_tag(tree, elem, Tag::Lbl),
        }
    } else if let Some(link) = elem.to_packed::<LinkMarker>() {
        push_group(tree, elem, GroupKind::Link(link.clone(), None))
    } else if let Some(_) = elem.to_packed::<TitleElem>() {
        push_tag(tree, elem, Tag::Title)
    } else if let Some(entry) = elem.to_packed::<OutlineEntry>() {
        push_group(tree, elem, GroupKind::OutlineEntry(entry.clone(), None))
    } else if let Some(_) = elem.to_packed::<ListElem>() {
        // TODO: infer numbering from `list.marker`
        let numbering = ListNumbering::Circle;
        let id = tree.ctx.lists.push(ListCtx::new());
        push_group(tree, elem, GroupKind::List(id, numbering, None))
    } else if let Some(_) = elem.to_packed::<EnumElem>() {
        // TODO: infer numbering from `enum.numbering`
        let numbering = ListNumbering::Decimal;
        let id = tree.ctx.lists.push(ListCtx::new());
        push_group(tree, elem, GroupKind::List(id, numbering, None))
    } else if let Some(_) = elem.to_packed::<TermsElem>() {
        let numbering = ListNumbering::None;
        let id = tree.ctx.lists.push(ListCtx::new());
        push_group(tree, elem, GroupKind::List(id, numbering, None))
    } else if let Some(figure) = elem.to_packed::<FigureElem>() {
        let lang = figure.locale;
        let bbox = tree.ctx.new_bbox();
        let group_id = tree.groups.list.next_id();
        let figure_id = tree.ctx.figures.push(FigureCtx::new(group_id, figure.clone()));
        push_group(tree, elem, GroupKind::Figure(figure_id, bbox, lang))
    } else if let Some(_) = elem.to_packed::<FigureCaption>() {
        let bbox = tree.ctx.new_bbox();
        push_group(tree, elem, GroupKind::FigureCaption(bbox, None))
    } else if let Some(image) = elem.to_packed::<ImageElem>() {
        let lang = image.locale;
        let bbox = tree.ctx.new_bbox();
        push_group(tree, elem, GroupKind::Image(image.clone(), bbox, lang))
    } else if let Some(equation) = elem.to_packed::<EquationElem>() {
        let lang = equation.locale;
        let bbox = tree.ctx.new_bbox();
        push_group(tree, elem, GroupKind::Formula(equation.clone(), bbox, lang))
    } else if let Some(table) = elem.to_packed::<TableElem>() {
        let group_id = tree.groups.list.next_id();
        let table_id = tree.ctx.tables.next_id();
        tree.ctx.tables.push(TableCtx::new(group_id, table_id, table.clone()));
        let bbox = tree.ctx.new_bbox();
        push_group(tree, elem, GroupKind::Table(table_id, bbox, None))
    } else if let Some(cell) = elem.to_packed::<TableCell>() {
        // Only repeated table headers and footer cells are laid out multiple
        // times. Mark duplicate headers as artifacts, since they have no
        // semantic meaning in the tag tree, which doesn't use page breaks for
        // it's semantic structure.
        let kind = if cell.is_repeated.val() {
            GroupKind::Artifact(ArtifactType::Other)
        } else {
            let tag = tree.groups.tags.push(Tag::TD);
            GroupKind::TableCell(cell.clone(), tag, None)
        };
        push_located(tree, elem, kind)
    } else if let Some(grid) = elem.to_packed::<GridElem>() {
        let group_id = tree.groups.list.next_id();
        let id = tree.ctx.grids.push(GridCtx::new(group_id, grid));
        push_group(tree, elem, GroupKind::Grid(id, None))
    } else if let Some(cell) = elem.to_packed::<GridCell>() {
        // The grid cells are collected into a grid to ensure proper reading
        // order even when using rowspans, which may be laid out later than
        // other cells in the same row.
        let kind = if !matches!(tree.parent_kind(), GroupKind::Grid(..)) {
            // If there is no grid parent, this means a grid layouter is used
            // internally.
            GroupKind::Transparent
        } else if cell.is_repeated.val() {
            // Only repeated grid headers and footer cells are laid out multiple
            // times. Mark duplicate headers as artifacts, since they have no
            // semantic meaning in the tag tree, which doesn't use page breaks
            // for it's semantic structure.
            GroupKind::Artifact(ArtifactType::Other)
        } else {
            GroupKind::GridCell(cell.clone(), None)
        };
        push_located(tree, elem, kind)
    } else if let Some(heading) = elem.to_packed::<HeadingElem>() {
        let level = heading.level().try_into().unwrap_or(NonZeroU16::MAX);
        let title = heading.body.plain_text().to_string();
        if title.is_empty() && tree.options.is_pdf_ua() {
            let contains_context = heading.body.traverse(&mut |c| {
                if c.is::<ContextElem>() {
                    return ControlFlow::Break(());
                }
                ControlFlow::Continue(())
            });
            let validator = tree.options.standards.config.validator().as_str();
            tree.errors.push(if contains_context.is_break() {
                error!(
                    heading.span(),
                    "{validator} error: heading title could not be determined";
                    hint: "this seems to be caused by a context expression within the heading";
                    hint: "consider wrapping the entire heading in a context expression instead"
                )
            } else {
                error!(heading.span(), "{validator} error: heading title is empty")
            });
        }
        push_tag(tree, elem, Tag::Hn(level, Some(title)))
    } else if let Some(_) = elem.to_packed::<FootnoteElem>() {
        push_located(tree, elem, GroupKind::LogicalParent(elem.clone()))
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
            push_group(tree, elem, GroupKind::CodeBlock(None))
        } else {
            push_tag(tree, elem, Tag::Code)
        }
    } else if let Some(_) = elem.to_packed::<RawLine>() {
        // If the raw element is inline, the content can be inserted directly.
        if matches!(tree.parent_kind(), GroupKind::CodeBlock(..)) {
            push_group(tree, elem, GroupKind::CodeBlockLine(None))
        } else {
            no_progress(tree)
        }
    } else if let Some(place) = elem.to_packed::<PlaceElem>() {
        if place.float.val() {
            push_located(tree, elem, GroupKind::LogicalParent(elem.clone()))
        } else {
            no_progress(tree)
        }
    } else if let Some(_) = elem.to_packed::<ParElem>() {
        push_weak(tree, elem, GroupKind::Par(None))

    // Text attributes
    } else if let Some(_strong) = elem.to_packed::<StrongElem>() {
        push_text_attr(tree, elem, TextAttr::Strong)
    } else if let Some(_emph) = elem.to_packed::<EmphElem>() {
        push_text_attr(tree, elem, TextAttr::Emph)
    } else if let Some(sub) = elem.to_packed::<SubElem>() {
        push_text_attr(tree, elem, TextAttr::SubScript(sub.clone()))
    } else if let Some(sup) = elem.to_packed::<SuperElem>() {
        push_text_attr(tree, elem, TextAttr::SuperScript(sup.clone()))
    } else if let Some(highlight) = elem.to_packed::<HighlightElem>() {
        push_text_attr(tree, elem, TextAttr::Highlight(highlight.clone()))
    } else if let Some(underline) = elem.to_packed::<UnderlineElem>() {
        push_text_attr(tree, elem, TextAttr::Underline(underline.clone()))
    } else if let Some(overline) = elem.to_packed::<OverlineElem>() {
        push_text_attr(tree, elem, TextAttr::Overline(overline.clone()))
    } else if let Some(strike) = elem.to_packed::<StrikeElem>() {
        push_text_attr(tree, elem, TextAttr::Strike(strike.clone()))
    } else {
        no_progress(tree)
    }
}

fn no_progress(tree: &TreeBuilder) -> GroupId {
    tree.current()
}

fn push_tag(tree: &mut TreeBuilder, elem: &Content, tag: impl Into<TagKind>) -> GroupId {
    let id = tree.groups.tags.push(tag.into());
    push_group(tree, elem, GroupKind::Standard(id, None))
}

fn push_text_attr(tree: &mut TreeBuilder, elem: &Content, attr: TextAttr) -> GroupId {
    push_group(tree, elem, GroupKind::TextAttr(attr))
}

fn push_artifact(tree: &mut TreeBuilder, elem: &Content, ty: ArtifactType) -> GroupId {
    push_group(tree, elem, GroupKind::Artifact(ty))
}

fn push_group(tree: &mut TreeBuilder, elem: &Content, kind: GroupKind) -> GroupId {
    let loc = elem.location().expect("elem to have a location");
    let span = elem.span();
    let parent = tree.current();
    let id = tree.groups.new_virtual(parent, span, kind);
    push_stack_entry(tree, Some(loc), id)
}

fn push_located(tree: &mut TreeBuilder, elem: &Content, kind: GroupKind) -> GroupId {
    let loc = elem.location().expect("elem to have a location");
    let span = elem.span();
    let parent = tree.current();
    let id = tree.groups.new_located(loc, parent, span, kind);
    push_stack_entry(tree, Some(loc), id)
}

fn push_weak(tree: &mut TreeBuilder, elem: &Content, kind: GroupKind) -> GroupId {
    let loc = elem.location().expect("elem to have a location");
    let span = elem.span();
    let parent = tree.current();
    let id = tree.groups.new_weak(parent, span, kind);
    push_stack_entry(tree, Some(loc), id)
}

fn push_stack_entry(
    tree: &mut TreeBuilder,
    loc: Option<Location>,
    id: GroupId,
) -> GroupId {
    let prog_idx = tree.progressions.len() as u32;
    let entry = StackEntry { loc, id, prog_idx };
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

    let entry = tree.stack[stack_idx];
    let outer = tree.groups.get(entry.id);

    // There are overlapping tags in the tag tree. Figure out whether breaking
    // up the current tag stack is semantically ok, and how to do it.
    let is_pdf_ua = tree.options.is_pdf_ua();
    let mut inner_break_priority = Some(BreakPriority::MAX);
    let mut inner_non_breakable_span = Span::detached();
    let mut inner_non_breakable_in_pdf_ua = false;
    for e in tree.stack.iter().skip(stack_idx + 1) {
        let group = tree.groups.get(e.id);
        let opportunity = tree.groups.breakable(&group.kind);
        let Some(priority) = opportunity.get(is_pdf_ua) else {
            if inner_non_breakable_span.is_detached() {
                inner_non_breakable_span = group.span;
            }
            if let BreakOpportunity::NoPdfUa(_) = opportunity {
                inner_non_breakable_in_pdf_ua = true;
            }
            inner_break_priority = None;
            continue;
        };

        if let Some(inner) = &mut inner_break_priority {
            *inner = (*inner).min(priority)
        }
    }

    let outer_break_opportunity = tree.groups.breakable(&outer.kind);
    let outer_break_priority = outer_break_opportunity.get(is_pdf_ua);

    match (outer_break_priority, inner_break_priority) {
        (Some(outer_priority), Some(inner_priority)) => {
            // Prefer splitting up the inner groups.
            if inner_priority >= outer_priority {
                Ok(split_inner_groups(tree, outer.parent, stack_idx))
            } else {
                Ok(split_outer_group(tree, outer.parent, stack_idx))
            }
        }
        (Some(_), None) => Ok(split_outer_group(tree, outer.parent, stack_idx)),
        (None, Some(_)) => Ok(split_inner_groups(tree, outer.parent, stack_idx)),
        (None, None) => {
            let non_breakable_span = if inner_non_breakable_span.is_detached() {
                outer.span
            } else {
                inner_non_breakable_span
            };

            let non_breakable_in_pdf_ua = inner_non_breakable_in_pdf_ua
                || matches!(outer_break_opportunity, BreakOpportunity::NoPdfUa(_));

            if non_breakable_in_pdf_ua {
                let validator = tree.options.standards.config.validator().as_str();
                bail!(
                    non_breakable_span,
                    "{validator} error: invalid document structure, \
                     this element's PDF tag would be split up";
                    hint: "this is probably caused by paragraph grouping";
                    hint: "maybe you've used a `parbreak`, `colbreak`, or `pagebreak`"
                );
            } else {
                bail!(
                    non_breakable_span,
                    "invalid document structure, \
                     this element's PDF tag would be split up";
                    hint: "please report this as a bug"
                );
            }
        }
    }
}

/// Consider the following introspection tags:
/// ```txt
/// start a
///   start b
///     start c
/// end   a
///     end   c
///   end   b
/// ```
/// This will split the inner groups, producing the following tag tree:
/// ```yml
/// - a:
///   - b:
///     - c:
/// - b:
///   - c:
/// ```
fn split_inner_groups(
    tree: &mut TreeBuilder,
    mut parent: GroupId,
    stack_idx: usize,
) -> GroupId {
    // Since the broken groups won't be visited again in any future progression,
    // they'll need to be closed when this progression is visited.
    let num_closed = (tree.stack.len() - stack_idx) as u16;
    tree.breaks.push(Break {
        prog_idx: tree.progressions.len() as u32,
        num_closed,
        num_opened: num_closed - 1,
    });

    // Remove the closed entry.
    tree.stack.remove(stack_idx);

    // Duplicate all broken entries.
    for entry in tree.stack.iter_mut().skip(stack_idx) {
        let new_id = tree.groups.break_group(entry.id, parent);
        *entry = StackEntry {
            loc: entry.loc,
            id: new_id,
            prog_idx: tree.progressions.len() as u32,
        };
        parent = new_id;
    }

    // We're now in a new duplicated group
    tree.parent()
}

/// Consider the following introspection tags:
/// ```txt
/// OPEN a
///   OPEN b
///     OPEN c
/// END  a
///     END  c
///   END  b
/// ```
/// This will split the outer group, producing the following tag tree:
/// ```yml
/// - a:
/// - b:
///   - a:
///   - c:
///     - a:
/// ```
fn split_outer_group(
    tree: &mut TreeBuilder,
    parent: GroupId,
    stack_idx: usize,
) -> GroupId {
    let prev = tree.current();

    // Remove the closed entry;
    let outer = tree.stack.remove(stack_idx);

    // Move the nested group out of the outer entry.
    tree.groups.get_mut(tree.stack[stack_idx].id).parent = parent;

    let mut entry_iter = tree.stack.iter().skip(stack_idx).peekable();
    while let Some(entry) = entry_iter.next() {
        let next_entry = entry_iter.peek().map(|e| e.id);

        let nested = tree.groups.break_group(outer.id, entry.id);

        // Move all children of the stack entry into the nested group.
        for (id, group) in tree.groups.list.ids().zip(tree.groups.list.iter_mut()).rev() {
            // Avoid searching *all* groups! The children of this group are guaranteed to be
            // created after the outer group and thus have a higher ID.
            if id == outer.id {
                break;
            }

            // Don't move the nested group into itself, or the next stack entry
            // into the nested group.
            if group.parent == entry.id && id != nested && Some(id) != next_entry {
                group.parent = nested;
            }
        }

        // Update progressions to jump into the inner entry instead.
        let prev = tree.progressions[entry.prog_idx as usize];
        for prog in tree.progressions[entry.prog_idx as usize..].iter_mut() {
            if *prog == prev {
                *prog = nested;
            }
        }

        // Either update an existing break, or insert a new one.
        let mut break_idx = Some(tree.breaks.len());
        for (i, brk) in tree.breaks.iter_mut().enumerate().rev() {
            if brk.prog_idx == entry.prog_idx {
                brk.num_closed += 1;
                brk.num_opened += 1;
                break_idx = None;
                break;
            } else if brk.prog_idx < entry.prog_idx {
                break_idx = Some(i + 1);
                break;
            }
        }
        if let Some(idx) = break_idx {
            // Insert a break to close the previous broken group, and enter
            // the new group.
            let brk = Break {
                prog_idx: entry.prog_idx,
                num_closed: 1,
                num_opened: 2,
            };
            tree.breaks.insert(idx, brk);
        }
    }

    // We're still in the same group, but the outer group has been split up.
    debug_assert_eq!(tree.parent(), prev);

    tree.parent()
}
