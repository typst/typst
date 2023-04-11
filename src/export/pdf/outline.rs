use ecow::EcoString;
use pdf_writer::{Finish, Ref, TextStr};

use super::{AbsExt, PdfContext, RefExt};
use crate::geom::{Abs, Point};

/// A heading in the outline panel.
#[derive(Debug, Clone)]
pub struct HeadingNode {
    pub content: EcoString,
    pub level: usize,
    pub position: Point,
    pub page: Ref,
    pub children: Vec<HeadingNode>,
}

impl HeadingNode {
    pub fn len(&self) -> usize {
        1 + self.children.iter().map(Self::len).sum::<usize>()
    }

    #[allow(unused)]
    pub fn try_insert(&mut self, child: Self, level: usize) -> bool {
        if level >= child.level {
            return false;
        }

        if let Some(last) = self.children.last_mut() {
            if last.try_insert(child.clone(), level + 1) {
                return true;
            }
        }

        self.children.push(child);
        true
    }
}

/// Write an outline item and all its children.
#[tracing::instrument(skip_all)]
pub fn write_outline_item(
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

    outline.title(TextStr(&node.content));
    outline.dest_direct().page(node.page).xyz(
        node.position.x.to_f32(),
        (node.position.y + Abs::pt(3.0)).to_f32(),
        None,
    );

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
