use krilla::tagging::{self as kt, TagGroup};
use krilla::tagging::{Node, Tag};

use crate::tags::resolve::{ElementKind, element_kind};

pub struct Accumulator<'a> {
    pub nesting: ElementKind,
    pub buf: &'a mut Vec<Node>,
    // Whether the last node is a `Span` used to wrap marked content sequences
    // inside a grouping element. Groupings element may not contain marked
    // content sequences directly.
    grouping_span: Option<Vec<Node>>,
}

impl std::ops::Drop for Accumulator<'_> {
    fn drop(&mut self) {
        self.push_grouping_span();
    }
}

impl<'a> Accumulator<'a> {
    pub fn new(nesting: ElementKind, buf: &'a mut Vec<Node>) -> Self {
        Self { nesting, buf, grouping_span: None }
    }

    fn push_buf(&mut self, node: Node) {
        self.buf.push(node);
    }

    fn push_grouping_span(&mut self) {
        if let Some(span_nodes) = self.grouping_span.take() {
            let tag = Tag::Span.with_placement(Some(kt::Placement::Block));
            let group = TagGroup::with_children(tag, span_nodes);
            self.push_buf(group.into());
        }
    }

    pub fn push(&mut self, mut node: Node) {
        if self.nesting == ElementKind::Grouping {
            match &mut node {
                Node::Group(group) => {
                    self.push_grouping_span();

                    // Ensure ILSE have block placement when inside grouping elements.
                    if element_kind(&group.tag) == ElementKind::Inline {
                        group.tag.set_placement(Some(kt::Placement::Block));
                    }

                    self.push_buf(node);
                }
                Node::Leaf(_) => {
                    let span_nodes = self.grouping_span.get_or_insert_default();
                    span_nodes.push(node);
                }
            }
        } else {
            self.push_buf(node);
        }
    }

    pub fn extend(&mut self, nodes: impl ExactSizeIterator<Item = Node>) {
        self.buf.reserve(nodes.len());
        for node in nodes {
            self.push(node);
        }
    }

    // Postfix drop.
    pub fn finish(self) {}
}
