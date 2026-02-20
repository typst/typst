use krilla::tagging::{self as kt, Node, Tag, TagGroup};

use crate::tags::resolve::{ElementKind, element_kind};

pub struct Accumulator {
    pub nesting: ElementKind,
    buf: Vec<Node>,
    // An intermediate `Span` node to collect marked cotnent sequences.
    // Groupings element may not contain marked content sequences directly, so
    // they are wrapped into a `Span`.
    grouping_span: Option<Vec<Node>>,
}

impl Accumulator {
    /// Create a new accumulator.
    fn new(nesting: ElementKind) -> Self {
        Self { nesting, buf: Vec::new(), grouping_span: None }
    }

    /// Create a new accumulator.
    pub fn root() -> Self {
        Self::new(ElementKind::Grouping)
    }

    /// Create a new nested accumulator. This will flush any intermediate
    /// grouping span, to ensure correct ordering of nested groups.
    pub fn nest(&mut self, nesting: ElementKind) -> Self {
        self.flush_grouping_span();
        Self::new(nesting)
    }

    /// Flush any intermediate grouping span into the nodes array.
    fn flush_grouping_span(&mut self) {
        if let Some(span_nodes) = self.grouping_span.take() {
            let tag = Tag::Span.with_placement(Some(kt::Placement::Block));
            let group = TagGroup::with_children(tag, span_nodes);
            self.buf.push(group.into());
        }
    }

    /// Push a node into this accumulator.
    pub fn push(&mut self, mut node: Node) {
        if self.nesting == ElementKind::Grouping {
            match &mut node {
                Node::Group(group) => {
                    self.flush_grouping_span();

                    // Ensure ILSE have block placement when inside grouping elements.
                    if element_kind(&group.tag) == ElementKind::Inline {
                        group.tag.set_placement(Some(kt::Placement::Block));
                    }

                    self.buf.push(node);
                }
                Node::Leaf(_) => {
                    let span_nodes = self.grouping_span.get_or_insert_default();
                    span_nodes.push(node);
                }
            }
        } else {
            self.buf.push(node);
        }
    }

    /// Reserve additional capacity inside the node buffer.
    pub fn reserve(&mut self, additional: usize) {
        self.buf.reserve(additional);
    }

    /// Push multiple nodes into this accumulator.
    pub fn extend(&mut self, nodes: impl ExactSizeIterator<Item = Node>) {
        self.buf.reserve(nodes.len());
        for node in nodes {
            self.push(node);
        }
    }

    // Finish accumulating and return the nodes.
    pub fn finish(mut self) -> Vec<Node> {
        self.flush_grouping_span();
        self.buf
    }
}
