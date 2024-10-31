use std::num::NonZeroUsize;

use pdf_writer::{Finish, Pdf, Ref, TextStr};
use typst::foundations::{NativeElement, Packed, StyleChain};
use typst::layout::Abs;
use typst::model::HeadingElem;

use crate::{AbsExt, TextStrExt, WithEverything};

/// Construct the outline for the document.
pub(crate) fn write_outline(
    chunk: &mut Pdf,
    alloc: &mut Ref,
    ctx: &WithEverything,
) -> Option<Ref> {
    let mut tree: Vec<HeadingNode> = vec![];

    // Stores the level of the topmost skipped ancestor of the next bookmarked
    // heading. A skipped heading is a heading with 'bookmarked: false', that
    // is, it is not added to the PDF outline, and so is not in the tree.
    // Therefore, its next descendant must be added at its level, which is
    // enforced in the manner shown below.
    let mut last_skipped_level = None;
    let elements = ctx.document.introspector.query(&HeadingElem::elem().select());

    for elem in elements.iter() {
        if let Some(page_ranges) = &ctx.options.page_ranges {
            if !page_ranges
                .includes_page(ctx.document.introspector.page(elem.location().unwrap()))
            {
                // Don't bookmark headings in non-exported pages
                continue;
            }
        }

        let heading = elem.to_packed::<HeadingElem>().unwrap();
        let leaf = HeadingNode::leaf(heading);

        if leaf.bookmarked {
            let mut children = &mut tree;

            // Descend the tree through the latest bookmarked heading of each
            // level until either:
            // - you reach a node whose children would be brothers of this
            // heading (=> add the current heading as a child of this node);
            // - you reach a node with no children (=> this heading probably
            // skipped a few nesting levels in Typst, or one or more ancestors
            // of this heading weren't bookmarked, so add it as a child of this
            // node, which is its deepest bookmarked ancestor);
            // - or, if the latest heading(s) was(/were) skipped
            // ('bookmarked: false'), then stop if you reach a node whose
            // children would be brothers of the latest skipped heading
            // of lowest level (=> those skipped headings would be ancestors
            // of the current heading, so add it as a 'brother' of the least
            // deep skipped ancestor among them, as those ancestors weren't
            // added to the bookmark tree, and the current heading should not
            // be mistakenly added as a descendant of a brother of that
            // ancestor.)
            //
            // That is, if you had a bookmarked heading of level N, a skipped
            // heading of level N, a skipped heading of level N + 1, and then
            // a bookmarked heading of level N + 2, that last one is bookmarked
            // as a level N heading (taking the place of its topmost skipped
            // ancestor), so that it is not mistakenly added as a descendant of
            // the previous level N heading.
            //
            // In other words, a heading can be added to the bookmark tree
            // at most as deep as its topmost skipped direct ancestor (if it
            // exists), or at most as deep as its actual nesting level in Typst
            // (not exceeding whichever is the most restrictive depth limit
            // of those two).
            while children.last().is_some_and(|last| {
                last_skipped_level.map_or(true, |l| last.level < l)
                    && last.level < leaf.level
            }) {
                children = &mut children.last_mut().unwrap().children;
            }

            // Since this heading was bookmarked, the next heading, if it is a
            // child of this one, won't have a skipped direct ancestor (indeed,
            // this heading would be its most direct ancestor, and wasn't
            // skipped). Therefore, it can be added as a child of this one, if
            // needed, following the usual rules listed above.
            last_skipped_level = None;
            children.push(leaf);
        } else if last_skipped_level.map_or(true, |l| leaf.level < l) {
            // Only the topmost / lowest-level skipped heading matters when you
            // have consecutive skipped headings (since none of them are being
            // added to the bookmark tree), hence the condition above.
            // This ensures the next bookmarked heading will be placed
            // at most as deep as its topmost skipped ancestors. Deeper
            // ancestors do not matter as the nesting structure they create
            // won't be visible in the PDF outline.
            last_skipped_level = Some(leaf.level);
        }
    }

    if tree.is_empty() {
        return None;
    }

    let root_id = alloc.bump();
    let start_ref = *alloc;
    let len = tree.len();

    let mut prev_ref = None;
    for (i, node) in tree.iter().enumerate() {
        prev_ref = Some(write_outline_item(
            ctx,
            chunk,
            alloc,
            node,
            root_id,
            prev_ref,
            i + 1 == len,
        ));
    }

    chunk
        .outline(root_id)
        .first(start_ref)
        .last(Ref::new(
            alloc.get() - tree.last().map(|child| child.len() as i32).unwrap_or(1),
        ))
        .count(tree.len() as i32);

    Some(root_id)
}

/// A heading in the outline panel.
#[derive(Debug)]
struct HeadingNode<'a> {
    element: &'a Packed<HeadingElem>,
    level: NonZeroUsize,
    bookmarked: bool,
    children: Vec<HeadingNode<'a>>,
}

impl<'a> HeadingNode<'a> {
    fn leaf(element: &'a Packed<HeadingElem>) -> Self {
        HeadingNode {
            level: element.resolve_level(StyleChain::default()),
            // 'bookmarked' set to 'auto' falls back to the value of 'outlined'.
            bookmarked: element
                .bookmarked(StyleChain::default())
                .unwrap_or_else(|| element.outlined(StyleChain::default())),
            element,
            children: Vec::new(),
        }
    }

    fn len(&self) -> usize {
        1 + self.children.iter().map(Self::len).sum::<usize>()
    }
}

/// Write an outline item and all its children.
fn write_outline_item(
    ctx: &WithEverything,
    chunk: &mut Pdf,
    alloc: &mut Ref,
    node: &HeadingNode,
    parent_ref: Ref,
    prev_ref: Option<Ref>,
    is_last: bool,
) -> Ref {
    let id = alloc.bump();
    let next_ref = Ref::new(id.get() + node.len() as i32);

    let mut outline = chunk.outline_item(id);
    outline.parent(parent_ref);

    if !is_last {
        outline.next(next_ref);
    }

    if let Some(prev_rev) = prev_ref {
        outline.prev(prev_rev);
    }

    if let Some(last_immediate_child) = node.children.last() {
        outline.first(Ref::new(id.get() + 1));
        outline.last(Ref::new(next_ref.get() - last_immediate_child.len() as i32));
        outline.count(-(node.children.len() as i32));
    }

    let body = node.element.body();
    outline.title(TextStr::trimmed(body.plain_text().trim()));

    let loc = node.element.location().unwrap();
    let pos = ctx.document.introspector.position(loc);
    let index = pos.page.get() - 1;

    // Don't link to non-exported pages.
    if let Some((Some(page), Some(page_ref))) =
        ctx.pages.get(index).zip(ctx.globals.pages.get(index))
    {
        let y = (pos.point.y - Abs::pt(10.0)).max(Abs::zero());
        outline.dest().page(*page_ref).xyz(
            pos.point.x.to_f32(),
            (page.content.size.y - y).to_f32(),
            None,
        );
    }

    outline.finish();

    let mut prev_ref = None;
    for (i, child) in node.children.iter().enumerate() {
        prev_ref = Some(write_outline_item(
            ctx,
            chunk,
            alloc,
            child,
            id,
            prev_ref,
            i + 1 == node.children.len(),
        ));
    }

    id
}
