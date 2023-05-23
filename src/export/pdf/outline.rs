use std::num::NonZeroUsize;

use pdf_writer::{Finish, Ref, TextStr};

use super::{AbsExt, PdfContext, RefExt};
use crate::geom::Abs;
use crate::model::Content;

/// Construct the outline for the document.
#[tracing::instrument(skip_all)]
pub fn write_outline(ctx: &mut PdfContext) -> Option<Ref> {
    let mut tree: Vec<HeadingNode> = vec![];
    for heading in ctx.introspector.query(&item!(heading_func).select()) {
        let leaf = HeadingNode::leaf((*heading).clone());

        let mut children = &mut tree;
        while children.last().map_or(false, |last| last.level < leaf.level) {
            children = &mut children.last_mut().unwrap().children;
        }

        children.push(leaf);
    }

    if tree.is_empty() {
        return None;
    }

    let root_id = ctx.alloc.bump();
    let start_ref = ctx.alloc;
    let len = tree.len();

    let mut prev_ref = None;
    for (i, node) in tree.iter().enumerate() {
        prev_ref = Some(write_outline_item(ctx, node, root_id, prev_ref, i + 1 == len));
    }

    ctx.writer
        .outline(root_id)
        .first(start_ref)
        .last(Ref::new(ctx.alloc.get() - 1))
        .count(tree.len() as i32);

    Some(root_id)
}

/// A heading in the outline panel.
#[derive(Debug, Clone)]
struct HeadingNode {
    element: Content,
    level: NonZeroUsize,
    children: Vec<HeadingNode>,
}

impl HeadingNode {
    fn leaf(element: Content) -> Self {
        HeadingNode {
            level: element.expect_field::<NonZeroUsize>("level"),
            element,
            children: Vec::new(),
        }
    }

    fn len(&self) -> usize {
        1 + self.children.iter().map(Self::len).sum::<usize>()
    }
}

/// Write an outline item and all its children.
#[tracing::instrument(skip_all)]
fn write_outline_item(
    ctx: &mut PdfContext,
    node: &HeadingNode,
    parent_ref: Ref,
    prev_ref: Option<Ref>,
    is_last: bool,
) -> Ref {
    let id = ctx.alloc.bump();
    let next_ref = Ref::new(id.get() + node.len() as i32);

    let mut outline = ctx.writer.outline_item(id);
    outline.parent(parent_ref);

    if !is_last {
        outline.next(next_ref);
    }

    if let Some(prev_rev) = prev_ref {
        outline.prev(prev_rev);
    }

    if !node.children.is_empty() {
        let current_child = Ref::new(id.get() + 1);
        outline.first(current_child);
        outline.last(Ref::new(next_ref.get() - 1));
        outline.count(-(node.children.len() as i32));
    }

    let body = node.element.expect_field::<Content>("body");
    outline.title(TextStr(body.plain_text().trim()));

    let loc = node.element.location().unwrap();
    let pos = ctx.introspector.position(loc);
    let index = pos.page.get() - 1;
    if let Some(&height) = ctx.page_heights.get(index) {
        let y = (pos.point.y - Abs::pt(10.0)).max(Abs::zero());
        outline.dest().page(ctx.page_refs[index]).xyz(
            pos.point.x.to_f32(),
            height - y.to_f32(),
            None,
        );
    }

    outline.finish();

    let mut prev_ref = None;
    for (i, child) in node.children.iter().enumerate() {
        prev_ref = Some(write_outline_item(
            ctx,
            child,
            id,
            prev_ref,
            i + 1 == node.children.len(),
        ));
    }

    id
}
