use pdf_writer::{Finish, Ref, TextStr};

use super::{LengthExt, PdfContext, RefExt};
use crate::geom::{Length, Point};
use crate::util::EcoString;

/// A heading that can later be linked in the outline panel.
#[derive(Debug, Clone)]
pub struct Heading {
    pub content: EcoString,
    pub level: usize,
    pub position: Point,
    pub page: Ref,
}

/// A node in the outline tree.
#[derive(Debug, Clone)]
pub struct HeadingNode {
    pub heading: Heading,
    pub children: Vec<HeadingNode>,
}

impl HeadingNode {
    pub fn leaf(heading: Heading) -> Self {
        HeadingNode { heading, children: Vec::new() }
    }

    pub fn len(&self) -> usize {
        1 + self.children.iter().map(Self::len).sum::<usize>()
    }

    pub fn insert(&mut self, other: Heading, level: usize) -> bool {
        if level >= other.level {
            return false;
        }

        if let Some(child) = self.children.last_mut() {
            if child.insert(other.clone(), level + 1) {
                return true;
            }
        }

        self.children.push(Self::leaf(other));
        true
    }
}

/// Write an outline item and all its children.
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
        outline.count(-1 * node.children.len() as i32);
    }

    outline.title(TextStr(&node.heading.content));
    outline.dest_direct().page(node.heading.page).xyz(
        node.heading.position.x.to_f32(),
        (node.heading.position.y + Length::pt(3.0)).to_f32(),
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
