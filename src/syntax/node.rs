use std::fmt::{self, Debug, Display, Formatter};
use std::ops::Range;
use std::sync::Arc;

use super::ast::AstNode;
use super::{SourceId, Span, SyntaxKind};
use crate::diag::SourceError;

/// A node in the untyped syntax tree.
#[derive(Clone, PartialEq, Hash)]
pub struct SyntaxNode(Repr);

/// The two internal representations.
#[derive(Clone, PartialEq, Hash)]
enum Repr {
    /// A leaf node.
    Leaf(NodeData),
    /// A reference-counted inner node.
    Inner(Arc<InnerNode>),
}

impl SyntaxNode {
    /// Create a new leaf node.
    pub fn leaf(kind: SyntaxKind, len: usize) -> Self {
        Self(Repr::Leaf(NodeData::new(kind, len)))
    }

    /// Create a new inner node with children.
    pub fn inner(kind: SyntaxKind, children: Vec<SyntaxNode>) -> Self {
        Self(Repr::Inner(Arc::new(InnerNode::with_children(kind, children))))
    }

    /// The type of the node.
    pub fn kind(&self) -> &SyntaxKind {
        &self.data().kind
    }

    /// Take the kind out of the node.
    pub fn take(self) -> SyntaxKind {
        match self.0 {
            Repr::Leaf(leaf) => leaf.kind,
            Repr::Inner(inner) => inner.data.kind.clone(),
        }
    }

    /// The length of the node.
    pub fn len(&self) -> usize {
        self.data().len
    }

    /// The span of the node.
    pub fn span(&self) -> Span {
        self.data().span
    }

    /// The number of descendants, including the node itself.
    pub fn descendants(&self) -> usize {
        match &self.0 {
            Repr::Inner(inner) => inner.descendants,
            Repr::Leaf(_) => 1,
        }
    }

    /// The node's children.
    pub fn children(&self) -> std::slice::Iter<'_, SyntaxNode> {
        match &self.0 {
            Repr::Inner(inner) => inner.children.iter(),
            Repr::Leaf(_) => [].iter(),
        }
    }

    /// Convert the node to a typed AST node.
    pub fn cast<T>(&self) -> Option<T>
    where
        T: AstNode,
    {
        T::from_untyped(self)
    }

    /// Get the first child that can cast to the AST type `T`.
    pub fn cast_first_child<T: AstNode>(&self) -> Option<T> {
        self.children().find_map(Self::cast)
    }

    /// Get the last child that can cast to the AST type `T`.
    pub fn cast_last_child<T: AstNode>(&self) -> Option<T> {
        self.children().rev().find_map(Self::cast)
    }

    /// Whether the node or its children contain an error.
    pub fn erroneous(&self) -> bool {
        match &self.0 {
            Repr::Inner(node) => node.erroneous,
            Repr::Leaf(data) => data.kind.is_error(),
        }
    }

    /// The error messages for this node and its descendants.
    pub fn errors(&self) -> Vec<SourceError> {
        if !self.erroneous() {
            return vec![];
        }

        match self.kind() {
            SyntaxKind::Error(pos, message) => {
                vec![SourceError::new(self.span(), message.clone()).with_pos(*pos)]
            }
            _ => self
                .children()
                .filter(|node| node.erroneous())
                .flat_map(|node| node.errors())
                .collect(),
        }
    }

    /// Change the type of the node.
    pub(super) fn convert(&mut self, kind: SyntaxKind) {
        match &mut self.0 {
            Repr::Inner(inner) => {
                let node = Arc::make_mut(inner);
                node.erroneous |= kind.is_error();
                node.data.kind = kind;
            }
            Repr::Leaf(leaf) => leaf.kind = kind,
        }
    }

    /// Set a synthetic span for the node and all its descendants.
    pub(super) fn synthesize(&mut self, span: Span) {
        match &mut self.0 {
            Repr::Inner(inner) => Arc::make_mut(inner).synthesize(span),
            Repr::Leaf(leaf) => leaf.synthesize(span),
        }
    }

    /// Assign spans to each node.
    pub(super) fn numberize(
        &mut self,
        id: SourceId,
        within: Range<u64>,
    ) -> NumberingResult {
        match &mut self.0 {
            Repr::Inner(inner) => Arc::make_mut(inner).numberize(id, None, within),
            Repr::Leaf(leaf) => leaf.numberize(id, within),
        }
    }

    /// If the span points into this node, convert it to a byte range.
    pub(super) fn range(&self, span: Span, offset: usize) -> Option<Range<usize>> {
        match &self.0 {
            Repr::Inner(inner) => inner.range(span, offset),
            Repr::Leaf(leaf) => leaf.range(span, offset),
        }
    }

    /// Whether this is a leaf node.
    pub(super) fn is_leaf(&self) -> bool {
        matches!(self.0, Repr::Leaf(_))
    }

    /// The node's children, mutably.
    pub(super) fn children_mut(&mut self) -> &mut [SyntaxNode] {
        match &mut self.0 {
            Repr::Leaf(_) => &mut [],
            Repr::Inner(inner) => &mut Arc::make_mut(inner).children,
        }
    }

    /// Replaces a range of children with a replacement.
    ///
    /// May have mutated the children if it returns `Err(_)`.
    pub(super) fn replace_children(
        &mut self,
        range: Range<usize>,
        replacement: Vec<SyntaxNode>,
    ) -> NumberingResult {
        if let Repr::Inner(inner) = &mut self.0 {
            Arc::make_mut(inner).replace_children(range, replacement)?;
        }
        Ok(())
    }

    /// Update this node after changes were made to one of its children.
    pub(super) fn update_parent(
        &mut self,
        prev_len: usize,
        new_len: usize,
        prev_descendants: usize,
        new_descendants: usize,
    ) {
        if let Repr::Inner(inner) = &mut self.0 {
            Arc::make_mut(inner).update_parent(
                prev_len,
                new_len,
                prev_descendants,
                new_descendants,
            );
        }
    }

    /// The metadata of the node.
    fn data(&self) -> &NodeData {
        match &self.0 {
            Repr::Inner(inner) => &inner.data,
            Repr::Leaf(leaf) => leaf,
        }
    }

    /// The upper bound of assigned numbers in this subtree.
    fn upper(&self) -> u64 {
        match &self.0 {
            Repr::Inner(inner) => inner.upper,
            Repr::Leaf(leaf) => leaf.span.number() + 1,
        }
    }
}

impl Debug for SyntaxNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match &self.0 {
            Repr::Inner(node) => node.fmt(f),
            Repr::Leaf(node) => node.fmt(f),
        }
    }
}

impl Default for SyntaxNode {
    fn default() -> Self {
        Self::leaf(SyntaxKind::None, 0)
    }
}

/// An inner node in the untyped syntax tree.
#[derive(Clone, Hash)]
struct InnerNode {
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
    /// Create a new inner node with the given kind and children.
    fn with_children(kind: SyntaxKind, children: Vec<SyntaxNode>) -> Self {
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

    /// Set a synthetic span for the node and all its descendants.
    fn synthesize(&mut self, span: Span) {
        self.data.synthesize(span);
        for child in &mut self.children {
            child.synthesize(span);
        }
    }

    /// Assign span numbers `within` an interval to this node's subtree or just
    /// a `range` of its children.
    fn numberize(
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

        // Number the node itself.
        let mut start = within.start;
        if range.is_none() {
            let end = start + stride;
            self.data.numberize(id, start..end)?;
            self.upper = within.end;
            start = end;
        }

        // Number the children.
        let len = self.children.len();
        for child in &mut self.children[range.unwrap_or(0..len)] {
            let end = start + child.descendants() as u64 * stride;
            child.numberize(id, start..end)?;
            start = end;
        }

        Ok(())
    }

    /// If the span points into this node, convert it to a byte range.
    fn range(&self, span: Span, mut offset: usize) -> Option<Range<usize>> {
        // Check whether we found it.
        if let Some(range) = self.data.range(span, offset) {
            return Some(range);
        }

        // The parent of a subtree has a smaller span number than all of its
        // descendants. Therefore, we can bail out early if the target span's
        // number is smaller than our number.
        if span.number() < self.data.span.number() {
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

    /// Replaces a range of children with a replacement.
    ///
    /// May have mutated the children if it returns `Err(_)`.
    fn replace_children(
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
                && (self.children[..range.start].iter().any(SyntaxNode::erroneous))
                || self.children[range.end..].iter().any(SyntaxNode::erroneous));

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
            let renumber = range.start - left..range.end + right;

            // The minimum assignable number is either
            // - the upper bound of the node right before the to-be-renumbered
            //   children,
            // - or this inner node's span number plus one if renumbering starts
            //   at the first child.
            let start_number = renumber
                .start
                .checked_sub(1)
                .and_then(|i| self.children.get(i))
                .map_or(self.data.span.number() + 1, |child| child.upper());

            // The upper bound for renumbering is either
            // - the span number of the first child after the to-be-renumbered
            //   children,
            // - or this node's upper bound if renumbering ends behind the last
            //   child.
            let end_number = self
                .children
                .get(renumber.end)
                .map_or(self.upper, |next| next.span().number());

            // Try to renumber.
            let within = start_number..end_number;
            let id = self.data.span.source();
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
    fn update_parent(
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

/// Data shared between leaf and inner nodes.
#[derive(Clone, Hash)]
struct NodeData {
    /// What kind of node this is (each kind would have its own struct in a
    /// strongly typed AST).
    kind: SyntaxKind,
    /// The byte length of the node in the source.
    len: usize,
    /// The node's span.
    span: Span,
}

impl NodeData {
    /// Create new node metadata.
    fn new(kind: SyntaxKind, len: usize) -> Self {
        Self { len, kind, span: Span::detached() }
    }

    /// Set a synthetic span for the node.
    fn synthesize(&mut self, span: Span) {
        self.span = span;
    }

    /// Assign a span to the node.
    fn numberize(&mut self, id: SourceId, within: Range<u64>) -> NumberingResult {
        if within.start < within.end {
            self.span = Span::new(id, (within.start + within.end) / 2);
            Ok(())
        } else {
            Err(Unnumberable)
        }
    }

    /// If the span points into this node, convert it to a byte range.
    fn range(&self, span: Span, offset: usize) -> Option<Range<usize>> {
        (self.span == span).then(|| offset..offset + self.len)
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

/// Result of numbering a node within an interval.
pub(super) type NumberingResult = Result<(), Unnumberable>;

/// Indicates that a node cannot be numbered within a given interval.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(super) struct Unnumberable;

impl Display for Unnumberable {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad("cannot number within this interval")
    }
}

impl std::error::Error for Unnumberable {}
