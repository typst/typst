//! Syntax types.

pub mod ast;
pub mod highlight;
mod kind;
mod span;

use std::fmt::{self, Debug, Formatter};
use std::ops::Range;
use std::sync::Arc;

pub use kind::*;
pub use span::*;

use self::ast::TypedNode;
use crate::diag::SourceError;
use crate::source::SourceId;

/// An inner or leaf node in the untyped syntax tree.
#[derive(Clone, PartialEq, Hash)]
pub enum SyntaxNode {
    /// A reference-counted inner node.
    Inner(Arc<InnerNode>),
    /// A leaf token.
    Leaf(NodeData),
}

impl SyntaxNode {
    /// The metadata of the node.
    pub fn data(&self) -> &NodeData {
        match self {
            Self::Inner(inner) => &inner.data,
            Self::Leaf(leaf) => leaf,
        }
    }

    /// The type of the node.
    pub fn kind(&self) -> &NodeKind {
        self.data().kind()
    }

    /// The length of the node.
    pub fn len(&self) -> usize {
        self.data().len()
    }

    /// The number of descendants, including the node itself.
    pub fn descendants(&self) -> usize {
        match self {
            Self::Inner(inner) => inner.descendants(),
            Self::Leaf(_) => 1,
        }
    }

    /// The span of the node.
    pub fn span(&self) -> Span {
        self.data().span()
    }

    /// Whether the node or its children contain an error.
    pub fn erroneous(&self) -> bool {
        match self {
            Self::Inner(node) => node.erroneous,
            Self::Leaf(data) => data.kind.is_error(),
        }
    }

    /// The error messages for this node and its descendants.
    pub fn errors(&self) -> Vec<SourceError> {
        if !self.erroneous() {
            return vec![];
        }

        match self.kind() {
            NodeKind::Error(pos, message) => {
                vec![SourceError::new(self.span().with_pos(*pos), message)]
            }
            _ => self
                .children()
                .filter(|node| node.erroneous())
                .flat_map(|node| node.errors())
                .collect(),
        }
    }

    /// The node's children.
    pub fn children(&self) -> std::slice::Iter<'_, SyntaxNode> {
        match self {
            Self::Inner(inner) => inner.children(),
            Self::Leaf(_) => [].iter(),
        }
    }

    /// Convert the node to a typed AST node.
    pub fn cast<T>(&self) -> Option<T>
    where
        T: TypedNode,
    {
        T::from_untyped(self)
    }

    /// Get the first child that can cast to the AST type `T`.
    pub fn cast_first_child<T: TypedNode>(&self) -> Option<T> {
        self.children().find_map(Self::cast)
    }

    /// Get the last child that can cast to the AST type `T`.
    pub fn cast_last_child<T: TypedNode>(&self) -> Option<T> {
        self.children().rev().find_map(Self::cast)
    }

    /// Change the type of the node.
    pub fn convert(&mut self, kind: NodeKind) {
        match self {
            Self::Inner(inner) => {
                let node = Arc::make_mut(inner);
                node.erroneous |= kind.is_error();
                node.data.kind = kind;
            }
            Self::Leaf(leaf) => leaf.kind = kind,
        }
    }

    /// Set a synthetic span for the node and all its descendants.
    pub fn synthesize(&mut self, span: Span) {
        match self {
            Self::Inner(inner) => Arc::make_mut(inner).synthesize(span),
            Self::Leaf(leaf) => leaf.synthesize(span),
        }
    }

    /// Assign spans to each node.
    pub fn numberize(&mut self, id: SourceId, within: Range<u64>) -> NumberingResult {
        match self {
            Self::Inner(inner) => Arc::make_mut(inner).numberize(id, None, within),
            Self::Leaf(leaf) => leaf.numberize(id, within),
        }
    }

    /// The upper bound of assigned numbers in this subtree.
    pub fn upper(&self) -> u64 {
        match self {
            Self::Inner(inner) => inner.upper(),
            Self::Leaf(leaf) => leaf.span().number() + 1,
        }
    }

    /// If the span points into this node, convert it to a byte range.
    pub fn range(&self, span: Span, offset: usize) -> Option<Range<usize>> {
        match self {
            Self::Inner(inner) => inner.range(span, offset),
            Self::Leaf(leaf) => leaf.range(span, offset),
        }
    }

    /// Returns all leaf descendants of this node (may include itself).
    ///
    /// This method is slow and only intended for testing.
    pub fn leafs(&self) -> Vec<Self> {
        if match self {
            Self::Inner(inner) => inner.children.is_empty(),
            Self::Leaf(_) => true,
        } {
            vec![self.clone()]
        } else {
            self.children().flat_map(Self::leafs).collect()
        }
    }
}

impl Default for SyntaxNode {
    fn default() -> Self {
        Self::Leaf(NodeData::new(NodeKind::None, 0))
    }
}

impl Debug for SyntaxNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Inner(node) => node.fmt(f),
            Self::Leaf(token) => token.fmt(f),
        }
    }
}

/// An inner node in the untyped syntax tree.
#[derive(Clone, Hash)]
pub struct InnerNode {
    /// Node metadata.
    data: NodeData,
    /// The number of nodes in the whole subtree, including this node.
    descendants: usize,
    /// Whether this node or any of its children are erroneous.
    erroneous: bool,
    /// The upper bound of this node's numbering range.
    upper: u64,
    /// This node's children, losslessly make up this node.
    children: Vec<SyntaxNode>,
}

impl InnerNode {
    /// Creates a new node with the given kind and a single child.
    pub fn with_child(kind: NodeKind, child: impl Into<SyntaxNode>) -> Self {
        Self::with_children(kind, vec![child.into()])
    }

    /// Creates a new node with the given kind and children.
    pub fn with_children(kind: NodeKind, children: Vec<SyntaxNode>) -> Self {
        let mut len = 0;
        let mut descendants = 1;
        let mut erroneous = kind.is_error();

        for child in &children {
            len += child.len();
            descendants += child.descendants();
            erroneous |= child.erroneous();
        }

        Self {
            data: NodeData::new(kind, len),
            descendants,
            erroneous,
            upper: 0,
            children,
        }
    }

    /// The node's metadata.
    pub fn data(&self) -> &NodeData {
        &self.data
    }

    /// The node's type.
    pub fn kind(&self) -> &NodeKind {
        self.data().kind()
    }

    /// The node's length.
    pub fn len(&self) -> usize {
        self.data().len()
    }

    /// The node's span.
    pub fn span(&self) -> Span {
        self.data().span()
    }

    /// The number of descendants, including the node itself.
    pub fn descendants(&self) -> usize {
        self.descendants
    }

    /// The node's children.
    pub fn children(&self) -> std::slice::Iter<'_, SyntaxNode> {
        self.children.iter()
    }

    /// Set a synthetic span for the node and all its descendants.
    pub fn synthesize(&mut self, span: Span) {
        self.data.synthesize(span);
        for child in &mut self.children {
            child.synthesize(span);
        }
    }

    /// Assign span numbers `within` an interval to this node's subtree or just
    /// a `range` of its children.
    pub fn numberize(
        &mut self,
        id: SourceId,
        range: Option<Range<usize>>,
        within: Range<u64>,
    ) -> NumberingResult {
        // Determine how many nodes we will number.
        let descendants = match &range {
            Some(range) if range.is_empty() => return Ok(()),
            Some(range) => self.children[range.clone()]
                .iter()
                .map(SyntaxNode::descendants)
                .sum::<usize>(),
            None => self.descendants,
        };

        // Determine the distance between two neighbouring assigned numbers. If
        // possible, we try to fit all numbers into the left half of `within`
        // so that there is space for future insertions.
        let space = within.end - within.start;
        let mut stride = space / (2 * descendants as u64);
        if stride == 0 {
            stride = space / self.descendants as u64;
            if stride == 0 {
                return Err(Unnumberable);
            }
        }

        // Number this node itself.
        let mut start = within.start;
        if range.is_none() {
            let end = start + stride;
            self.data.numberize(id, start .. end)?;
            self.upper = within.end;
            start = end;
        }

        // Number the children.
        let len = self.children.len();
        for child in &mut self.children[range.unwrap_or(0 .. len)] {
            let end = start + child.descendants() as u64 * stride;
            child.numberize(id, start .. end)?;
            start = end;
        }

        Ok(())
    }

    /// The upper bound of assigned numbers in this subtree.
    pub fn upper(&self) -> u64 {
        self.upper
    }

    /// If the span points into this node, convert it to a byte range.
    pub fn range(&self, span: Span, mut offset: usize) -> Option<Range<usize>> {
        // Check whether we found it.
        if let Some(range) = self.data.range(span, offset) {
            return Some(range);
        }

        // The parent of a subtree has a smaller span number than all of its
        // descendants. Therefore, we can bail out early if the target span's
        // number is smaller than our number.
        if span.number() < self.span().number() {
            return None;
        }

        let mut children = self.children.iter().peekable();
        while let Some(child) = children.next() {
            // Every node in this child's subtree has a smaller span number than
            // the next sibling. Therefore we only need to recurse if the next
            // sibling's span number is larger than the target span's number.
            if children
                .peek()
                .map_or(true, |next| next.span().number() > span.number())
            {
                if let Some(range) = child.range(span, offset) {
                    return Some(range);
                }
            }

            offset += child.len();
        }

        None
    }

    /// The node's children, mutably.
    pub(crate) fn children_mut(&mut self) -> &mut [SyntaxNode] {
        &mut self.children
    }

    /// Replaces a range of children with a replacement.
    ///
    /// May have mutated the children if it returns `Err(_)`.
    pub(crate) fn replace_children(
        &mut self,
        mut range: Range<usize>,
        replacement: Vec<SyntaxNode>,
    ) -> NumberingResult {
        let superseded = &self.children[range.clone()];

        // Compute the new byte length.
        self.data.len = self.data.len
            + replacement.iter().map(SyntaxNode::len).sum::<usize>()
            - superseded.iter().map(SyntaxNode::len).sum::<usize>();

        // Compute the new number of descendants.
        self.descendants = self.descendants
            + replacement.iter().map(SyntaxNode::descendants).sum::<usize>()
            - superseded.iter().map(SyntaxNode::descendants).sum::<usize>();

        // Determine whether we're still erroneous after the replacement. That's
        // the case if
        // - any of the new nodes is erroneous,
        // - or if we were erroneous before due to a non-superseded node.
        self.erroneous = replacement.iter().any(SyntaxNode::erroneous)
            || (self.erroneous
                && (self.children[.. range.start].iter().any(SyntaxNode::erroneous))
                || self.children[range.end ..].iter().any(SyntaxNode::erroneous));

        // Perform the replacement.
        let replacement_count = replacement.len();
        self.children.splice(range.clone(), replacement);
        range.end = range.start + replacement_count;

        // Renumber the new children. Retries until it works, taking
        // exponentially more children into account.
        let mut left = 0;
        let mut right = 0;
        let max_left = range.start;
        let max_right = self.children.len() - range.end;
        loop {
            let renumber = range.start - left .. range.end + right;

            // The minimum assignable number is either
            // - the upper bound of the node right before the to-be-renumbered
            //   children,
            // - or this inner node's span number plus one if renumbering starts
            //   at the first child.
            let start_number = renumber
                .start
                .checked_sub(1)
                .and_then(|i| self.children.get(i))
                .map_or(self.span().number() + 1, |child| child.upper());

            // The upper bound for renumbering is either
            // - the span number of the first child after the to-be-renumbered
            //   children,
            // - or this node's upper bound if renumbering ends behind the last
            //   child.
            let end_number = self
                .children
                .get(renumber.end)
                .map_or(self.upper(), |next| next.span().number());

            // Try to renumber.
            let within = start_number .. end_number;
            let id = self.span().source();
            if self.numberize(id, Some(renumber), within).is_ok() {
                return Ok(());
            }

            // If it didn't even work with all children, we give up.
            if left == max_left && right == max_right {
                return Err(Unnumberable);
            }

            // Exponential expansion to both sides.
            left = (left + 1).next_power_of_two().min(max_left);
            right = (right + 1).next_power_of_two().min(max_right);
        }
    }

    /// Update this node after changes were made to one of its children.
    pub(crate) fn update_parent(
        &mut self,
        prev_len: usize,
        new_len: usize,
        prev_descendants: usize,
        new_descendants: usize,
    ) {
        self.data.len = self.data.len + new_len - prev_len;
        self.descendants = self.descendants + new_descendants - prev_descendants;
        self.erroneous = self.children.iter().any(SyntaxNode::erroneous);
    }
}

impl From<InnerNode> for SyntaxNode {
    fn from(node: InnerNode) -> Self {
        Arc::new(node).into()
    }
}

impl From<Arc<InnerNode>> for SyntaxNode {
    fn from(node: Arc<InnerNode>) -> Self {
        Self::Inner(node)
    }
}

impl Debug for InnerNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.data.fmt(f)?;
        if !self.children.is_empty() {
            f.write_str(" ")?;
            f.debug_list().entries(&self.children).finish()?;
        }
        Ok(())
    }
}

impl PartialEq for InnerNode {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
            && self.descendants == other.descendants
            && self.erroneous == other.erroneous
            && self.children == other.children
    }
}

/// Data shared between inner and leaf nodes.
#[derive(Clone, Hash)]
pub struct NodeData {
    /// What kind of node this is (each kind would have its own struct in a
    /// strongly typed AST).
    kind: NodeKind,
    /// The byte length of the node in the source.
    len: usize,
    /// The node's span.
    span: Span,
}

impl NodeData {
    /// Create new node metadata.
    pub fn new(kind: NodeKind, len: usize) -> Self {
        Self { len, kind, span: Span::detached() }
    }

    /// The node's type.
    pub fn kind(&self) -> &NodeKind {
        &self.kind
    }

    /// The node's length.
    pub fn len(&self) -> usize {
        self.len
    }

    /// The node's span.
    pub fn span(&self) -> Span {
        self.span
    }

    /// Set a synthetic span for the node.
    pub fn synthesize(&mut self, span: Span) {
        self.span = span;
    }

    /// Assign a span to the node.
    pub fn numberize(&mut self, id: SourceId, within: Range<u64>) -> NumberingResult {
        if within.start < within.end {
            self.span = Span::new(id, (within.start + within.end) / 2);
            Ok(())
        } else {
            Err(Unnumberable)
        }
    }

    /// If the span points into this node, convert it to a byte range.
    pub fn range(&self, span: Span, offset: usize) -> Option<Range<usize>> {
        (span.with_pos(SpanPos::Full) == self.span).then(|| {
            let end = offset + self.len();
            match span.pos() {
                SpanPos::Full => offset .. end,
                SpanPos::Start => offset .. offset,
                SpanPos::End => end .. end,
            }
        })
    }
}

impl From<NodeData> for SyntaxNode {
    fn from(token: NodeData) -> Self {
        Self::Leaf(token)
    }
}

impl Debug for NodeData {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.len)
    }
}

impl PartialEq for NodeData {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind && self.len == other.len
    }
}
