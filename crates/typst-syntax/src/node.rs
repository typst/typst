use std::fmt::{self, Debug, Display, Formatter};
use std::ops::{Deref, Range};
use std::rc::Rc;
use std::sync::Arc;

use ecow::{EcoString, EcoVec, eco_format, eco_vec};

use crate::{FileId, Span, SyntaxKind, SyntaxMode};

/// A node in the untyped syntax tree.
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct SyntaxNode(Repr);

/// The three internal representations.
#[derive(Clone, Eq, PartialEq, Hash)]
enum Repr {
    /// A leaf node.
    Leaf(LeafNode),
    /// A reference-counted inner node.
    Inner(Arc<InnerNode>),
    /// An error node.
    Error(Arc<ErrorNode>),
}

impl SyntaxNode {
    /// Create a new leaf node.
    pub fn leaf(kind: SyntaxKind, text: impl Into<EcoString>) -> Self {
        Self(Repr::Leaf(LeafNode::new(kind, text)))
    }

    /// Create a new inner node with children.
    pub fn inner(kind: SyntaxKind, children: Vec<SyntaxNode>) -> Self {
        Self(Repr::Inner(Arc::new(InnerNode::new(kind, children))))
    }

    /// Create a new error node.
    pub fn error(error: SyntaxError, text: impl Into<EcoString>) -> Self {
        Self(Repr::Error(Arc::new(ErrorNode::new(error, text))))
    }

    /// Create a dummy node of the given kind.
    ///
    /// Panics if `kind` is `SyntaxKind::Error`.
    #[track_caller]
    pub const fn placeholder(kind: SyntaxKind) -> Self {
        if matches!(kind, SyntaxKind::Error) {
            panic!("cannot create error placeholder");
        }
        Self(Repr::Leaf(LeafNode {
            kind,
            text: EcoString::new(),
            span: Span::detached(),
        }))
    }

    /// The type of the node.
    pub fn kind(&self) -> SyntaxKind {
        match &self.0 {
            Repr::Leaf(leaf) => leaf.kind,
            Repr::Inner(inner) => inner.kind,
            Repr::Error(_) => SyntaxKind::Error,
        }
    }

    /// Return `true` if the length is 0.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The byte length of the node in the source text.
    pub fn len(&self) -> usize {
        match &self.0 {
            Repr::Leaf(leaf) => leaf.len(),
            Repr::Inner(inner) => inner.len,
            Repr::Error(node) => node.len(),
        }
    }

    /// The span of the node.
    pub fn span(&self) -> Span {
        match &self.0 {
            Repr::Leaf(leaf) => leaf.span,
            Repr::Inner(inner) => inner.span,
            Repr::Error(node) => node.error.span,
        }
    }

    /// The text of the node if it is a leaf or error node.
    ///
    /// Returns the empty string if this is an inner node.
    pub fn text(&self) -> &EcoString {
        static EMPTY: EcoString = EcoString::new();
        match &self.0 {
            Repr::Leaf(leaf) => &leaf.text,
            Repr::Inner(_) => &EMPTY,
            Repr::Error(node) => &node.text,
        }
    }

    /// Extract the text from the node.
    ///
    /// Builds the string if this is an inner node.
    pub fn into_text(self) -> EcoString {
        match self.0 {
            Repr::Leaf(leaf) => leaf.text,
            Repr::Inner(inner) => {
                inner.children.iter().cloned().map(Self::into_text).collect()
            }
            Repr::Error(node) => node.text.clone(),
        }
    }

    /// The node's children.
    pub fn children(&self) -> std::slice::Iter<'_, SyntaxNode> {
        match &self.0 {
            Repr::Leaf(_) | Repr::Error(_) => [].iter(),
            Repr::Inner(inner) => inner.children.iter(),
        }
    }

    /// Whether the node or its children contain an error.
    pub fn erroneous(&self) -> bool {
        match &self.0 {
            Repr::Leaf(_) => false,
            Repr::Inner(inner) => inner.erroneous,
            Repr::Error(_) => true,
        }
    }

    /// The error messages for this node and its descendants.
    pub fn errors(&self) -> Vec<SyntaxError> {
        if !self.erroneous() {
            return vec![];
        }

        if let Repr::Error(node) = &self.0 {
            vec![node.error.clone()]
        } else {
            self.children()
                .filter(|node| node.erroneous())
                .flat_map(|node| node.errors())
                .collect()
        }
    }

    /// Add a user-presentable hint if this is an error node.
    pub fn hint(&mut self, hint: impl Into<EcoString>) {
        if let Repr::Error(node) = &mut self.0 {
            Arc::make_mut(node).hint(hint);
        }
    }

    /// Set a synthetic span for the node and all its descendants.
    pub fn synthesize(&mut self, span: Span) {
        match &mut self.0 {
            Repr::Leaf(leaf) => leaf.span = span,
            Repr::Inner(inner) => Arc::make_mut(inner).synthesize(span),
            Repr::Error(node) => Arc::make_mut(node).error.span = span,
        }
    }

    /// Whether the two syntax nodes are the same apart from spans.
    pub fn spanless_eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (Repr::Leaf(a), Repr::Leaf(b)) => a.spanless_eq(b),
            (Repr::Inner(a), Repr::Inner(b)) => a.spanless_eq(b),
            (Repr::Error(a), Repr::Error(b)) => a.spanless_eq(b),
            _ => false,
        }
    }
}

impl SyntaxNode {
    /// Convert the child to another kind.
    ///
    /// Don't use this for converting to an error!
    #[track_caller]
    pub(super) fn convert_to_kind(&mut self, kind: SyntaxKind) {
        debug_assert!(!kind.is_error());
        match &mut self.0 {
            Repr::Leaf(leaf) => leaf.kind = kind,
            Repr::Inner(inner) => Arc::make_mut(inner).kind = kind,
            Repr::Error(_) => panic!("cannot convert error"),
        }
    }

    /// Convert the child to an error, if it isn't already one.
    pub(super) fn convert_to_error(&mut self, message: impl Into<EcoString>) {
        if !self.kind().is_error() {
            let text = std::mem::take(self).into_text();
            *self = SyntaxNode::error(SyntaxError::new(message), text);
        }
    }

    /// Convert the child to an error stating that the given thing was
    /// expected, but the current kind was found.
    pub(super) fn expected(&mut self, expected: &str) {
        let kind = self.kind();
        self.convert_to_error(eco_format!("expected {expected}, found {}", kind.name()));
        if kind.is_keyword() && matches!(expected, "identifier" | "pattern") {
            self.hint(eco_format!(
                "keyword `{text}` is not allowed as an identifier; try `{text}_` instead",
                text = self.text(),
            ));
        }
    }

    /// Convert the child to an error stating it was unexpected.
    pub(super) fn unexpected(&mut self) {
        self.convert_to_error(eco_format!("unexpected {}", self.kind().name()));
    }

    /// Assign spans to each node.
    pub(super) fn numberize(
        &mut self,
        id: FileId,
        within: Range<u64>,
    ) -> NumberingResult {
        if within.start >= within.end {
            return Err(Unnumberable);
        }

        let mid = Span::from_number(id, (within.start + within.end) / 2).unwrap();
        match &mut self.0 {
            Repr::Leaf(leaf) => leaf.span = mid,
            Repr::Inner(inner) => Arc::make_mut(inner).numberize(id, None, within)?,
            Repr::Error(node) => Arc::make_mut(node).error.span = mid,
        }

        Ok(())
    }

    /// Whether this is a leaf node.
    pub(super) fn is_leaf(&self) -> bool {
        matches!(self.0, Repr::Leaf(_))
    }

    /// The number of descendants, including the node itself.
    pub(super) fn descendants(&self) -> usize {
        match &self.0 {
            Repr::Leaf(_) | Repr::Error(_) => 1,
            Repr::Inner(inner) => inner.descendants,
        }
    }

    /// The node's children, mutably.
    pub(super) fn children_mut(&mut self) -> &mut [SyntaxNode] {
        match &mut self.0 {
            Repr::Leaf(_) | Repr::Error(_) => &mut [],
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

    /// The upper bound of assigned numbers in this subtree.
    pub(super) fn upper(&self) -> u64 {
        match &self.0 {
            Repr::Leaf(leaf) => leaf.span.number() + 1,
            Repr::Inner(inner) => inner.upper,
            Repr::Error(node) => node.error.span.number() + 1,
        }
    }
}

impl Debug for SyntaxNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match &self.0 {
            Repr::Leaf(leaf) => leaf.fmt(f),
            Repr::Inner(inner) => inner.fmt(f),
            Repr::Error(node) => node.fmt(f),
        }
    }
}

impl Default for SyntaxNode {
    fn default() -> Self {
        Self::leaf(SyntaxKind::End, EcoString::new())
    }
}

/// A leaf node in the untyped syntax tree.
#[derive(Clone, Eq, PartialEq, Hash)]
struct LeafNode {
    /// What kind of node this is (each kind would have its own struct in a
    /// strongly typed AST).
    kind: SyntaxKind,
    /// The source text of the node.
    text: EcoString,
    /// The node's span.
    span: Span,
}

impl LeafNode {
    /// Create a new leaf node.
    #[track_caller]
    fn new(kind: SyntaxKind, text: impl Into<EcoString>) -> Self {
        debug_assert!(!kind.is_error());
        Self { kind, text: text.into(), span: Span::detached() }
    }

    /// The byte length of the node in the source text.
    fn len(&self) -> usize {
        self.text.len()
    }

    /// Whether the two leaf nodes are the same apart from spans.
    fn spanless_eq(&self, other: &Self) -> bool {
        self.kind == other.kind && self.text == other.text
    }
}

impl Debug for LeafNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}: {:?}", self.kind, self.text)
    }
}

/// An inner node in the untyped syntax tree.
#[derive(Clone, Eq, PartialEq, Hash)]
struct InnerNode {
    /// What kind of node this is (each kind would have its own struct in a
    /// strongly typed AST).
    kind: SyntaxKind,
    /// The byte length of the node in the source.
    len: usize,
    /// The node's span.
    span: Span,
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
    #[track_caller]
    fn new(kind: SyntaxKind, children: Vec<SyntaxNode>) -> Self {
        debug_assert!(!kind.is_error());

        let mut len = 0;
        let mut descendants = 1;
        let mut erroneous = false;

        for child in &children {
            len += child.len();
            descendants += child.descendants();
            erroneous |= child.erroneous();
        }

        Self {
            kind,
            len,
            span: Span::detached(),
            descendants,
            erroneous,
            upper: 0,
            children,
        }
    }

    /// Set a synthetic span for the node and all its descendants.
    fn synthesize(&mut self, span: Span) {
        self.span = span;
        self.upper = span.number();
        for child in &mut self.children {
            child.synthesize(span);
        }
    }

    /// Assign span numbers `within` an interval to this node's subtree or just
    /// a `range` of its children.
    fn numberize(
        &mut self,
        id: FileId,
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
            self.span = Span::from_number(id, (start + end) / 2).unwrap();
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

    /// Whether the two inner nodes are the same apart from spans.
    fn spanless_eq(&self, other: &Self) -> bool {
        self.kind == other.kind
            && self.len == other.len
            && self.descendants == other.descendants
            && self.erroneous == other.erroneous
            && self.children.len() == other.children.len()
            && self
                .children
                .iter()
                .zip(&other.children)
                .all(|(a, b)| a.spanless_eq(b))
    }

    /// Replaces a range of children with a replacement.
    ///
    /// May have mutated the children if it returns `Err(_)`.
    fn replace_children(
        &mut self,
        mut range: Range<usize>,
        replacement: Vec<SyntaxNode>,
    ) -> NumberingResult {
        let Some(id) = self.span.id() else { return Err(Unnumberable) };
        let mut replacement_range = 0..replacement.len();

        // Trim off common prefix.
        while range.start < range.end
            && replacement_range.start < replacement_range.end
            && self.children[range.start]
                .spanless_eq(&replacement[replacement_range.start])
        {
            range.start += 1;
            replacement_range.start += 1;
        }

        // Trim off common suffix.
        while range.start < range.end
            && replacement_range.start < replacement_range.end
            && self.children[range.end - 1]
                .spanless_eq(&replacement[replacement_range.end - 1])
        {
            range.end -= 1;
            replacement_range.end -= 1;
        }

        let mut replacement_vec = replacement;
        let replacement = &replacement_vec[replacement_range.clone()];
        let superseded = &self.children[range.clone()];

        // Compute the new byte length.
        self.len = self.len + replacement.iter().map(SyntaxNode::len).sum::<usize>()
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
        self.children
            .splice(range.clone(), replacement_vec.drain(replacement_range.clone()));
        range.end = range.start + replacement_range.len();

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
                .map_or(self.span.number() + 1, |child| child.upper());

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
        self.len = self.len + new_len - prev_len;
        self.descendants = self.descendants + new_descendants - prev_descendants;
        self.erroneous = self.children.iter().any(SyntaxNode::erroneous);
    }
}

impl Debug for InnerNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.len)?;
        if !self.children.is_empty() {
            f.write_str(" ")?;
            f.debug_list().entries(&self.children).finish()?;
        }
        Ok(())
    }
}

/// An error node in the untyped syntax tree.
#[derive(Clone, Eq, PartialEq, Hash)]
struct ErrorNode {
    /// The source text of the node.
    text: EcoString,
    /// The syntax error.
    error: SyntaxError,
}

impl ErrorNode {
    /// Create new error node.
    fn new(error: SyntaxError, text: impl Into<EcoString>) -> Self {
        Self { text: text.into(), error }
    }

    /// The byte length of the node in the source text.
    fn len(&self) -> usize {
        self.text.len()
    }

    /// Add a user-presentable hint to this error node.
    fn hint(&mut self, hint: impl Into<EcoString>) {
        self.error.hints.push(hint.into());
    }

    /// Whether the two leaf nodes are the same apart from spans.
    fn spanless_eq(&self, other: &Self) -> bool {
        self.text == other.text && self.error.spanless_eq(&other.error)
    }
}

impl Debug for ErrorNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Error: {:?} ({})", self.text, self.error.message)
    }
}

/// A syntactical error.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct SyntaxError {
    /// The node's span.
    pub span: Span,
    /// The error message.
    pub message: EcoString,
    /// Additional hints to the user, indicating how this error could be avoided
    /// or worked around.
    pub hints: EcoVec<EcoString>,
}

impl SyntaxError {
    /// Create a new detached syntax error.
    pub fn new(message: impl Into<EcoString>) -> Self {
        Self {
            span: Span::detached(),
            message: message.into(),
            hints: eco_vec![],
        }
    }

    /// Whether the two errors are the same apart from spans.
    fn spanless_eq(&self, other: &Self) -> bool {
        self.message == other.message && self.hints == other.hints
    }
}

/// A syntax node in a context.
///
/// Knows its exact offset in the file and provides access to its
/// children, parent and siblings.
///
/// **Note that all sibling and leaf accessors skip over trivia!**
#[derive(Clone)]
pub struct LinkedNode<'a> {
    node: &'a SyntaxNode,
    parent: Option<Rc<Self>>,
    index: usize,
    offset: usize,
}

impl<'a> LinkedNode<'a> {
    /// Start a new traversal at a root node.
    pub fn new(root: &'a SyntaxNode) -> Self {
        Self { node: root, parent: None, index: 0, offset: 0 }
    }

    /// Get the contained syntax node.
    pub fn get(&self) -> &'a SyntaxNode {
        self.node
    }

    /// The index of this node in its parent's children list.
    pub fn index(&self) -> usize {
        self.index
    }

    /// The absolute byte offset of this node in the source file.
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// The byte range of this node in the source file.
    pub fn range(&self) -> Range<usize> {
        self.offset..self.offset + self.node.len()
    }

    /// An iterator over this node's children.
    pub fn children(&self) -> LinkedChildren<'a> {
        LinkedChildren {
            parent: Rc::new(self.clone()),
            iter: self.node.children().enumerate(),
            front: self.offset,
            back: self.offset + self.len(),
        }
    }

    /// Find a descendant with the given span.
    pub fn find(&self, span: Span) -> Option<LinkedNode<'a>> {
        if self.span() == span {
            return Some(self.clone());
        }

        if let Repr::Inner(inner) = &self.0 {
            // The parent of a subtree has a smaller span number than all of its
            // descendants. Therefore, we can bail out early if the target span's
            // number is smaller than our number.
            if span.number() < inner.span.number() {
                return None;
            }

            let mut children = self.children().peekable();
            while let Some(child) = children.next() {
                // Every node in this child's subtree has a smaller span number than
                // the next sibling. Therefore we only need to recurse if the next
                // sibling's span number is larger than the target span's number.
                if children
                    .peek()
                    .is_none_or(|next| next.span().number() > span.number())
                    && let Some(found) = child.find(span)
                {
                    return Some(found);
                }
            }
        }

        None
    }
}

/// Access to parents and siblings.
impl LinkedNode<'_> {
    /// Get this node's parent.
    pub fn parent(&self) -> Option<&Self> {
        self.parent.as_deref()
    }

    /// Get the first previous non-trivia sibling node.
    pub fn prev_sibling(&self) -> Option<Self> {
        let parent = self.parent()?;
        let index = self.index.checked_sub(1)?;
        let node = parent.node.children().nth(index)?;
        let offset = self.offset - node.len();
        let prev = Self { node, parent: self.parent.clone(), index, offset };
        if prev.kind().is_trivia() { prev.prev_sibling() } else { Some(prev) }
    }

    /// Get the next non-trivia sibling node.
    pub fn next_sibling(&self) -> Option<Self> {
        let parent = self.parent()?;
        let index = self.index.checked_add(1)?;
        let node = parent.node.children().nth(index)?;
        let offset = self.offset + self.node.len();
        let next = Self { node, parent: self.parent.clone(), index, offset };
        if next.kind().is_trivia() { next.next_sibling() } else { Some(next) }
    }

    /// Get the kind of this node's parent.
    pub fn parent_kind(&self) -> Option<SyntaxKind> {
        Some(self.parent()?.node.kind())
    }

    /// Get the kind of this node's first previous non-trivia sibling.
    pub fn prev_sibling_kind(&self) -> Option<SyntaxKind> {
        Some(self.prev_sibling()?.node.kind())
    }

    /// Get the kind of this node's next non-trivia sibling.
    pub fn next_sibling_kind(&self) -> Option<SyntaxKind> {
        Some(self.next_sibling()?.node.kind())
    }
}

/// Indicates whether the cursor is before the related byte index, or after.
#[derive(Debug, Clone)]
pub enum Side {
    Before,
    After,
}

/// Access to leaves.
impl LinkedNode<'_> {
    /// Get the rightmost non-trivia leaf before this node.
    pub fn prev_leaf(&self) -> Option<Self> {
        let mut node = self.clone();
        while let Some(prev) = node.prev_sibling() {
            if let Some(leaf) = prev.rightmost_leaf() {
                return Some(leaf);
            }
            node = prev;
        }
        self.parent()?.prev_leaf()
    }

    /// Find the leftmost contained non-trivia leaf.
    pub fn leftmost_leaf(&self) -> Option<Self> {
        if self.is_leaf() && !self.kind().is_trivia() && !self.kind().is_error() {
            return Some(self.clone());
        }

        for child in self.children() {
            if let Some(leaf) = child.leftmost_leaf() {
                return Some(leaf);
            }
        }

        None
    }

    /// Get the leaf immediately before the specified byte offset.
    fn leaf_before(&self, cursor: usize) -> Option<Self> {
        if self.node.children().len() == 0 && cursor <= self.offset + self.len() {
            return Some(self.clone());
        }

        let mut offset = self.offset;
        let count = self.node.children().len();
        for (i, child) in self.children().enumerate() {
            let len = child.len();
            if (offset < cursor && cursor <= offset + len)
                || (offset == cursor && i + 1 == count)
            {
                return child.leaf_before(cursor);
            }
            offset += len;
        }

        None
    }

    /// Get the leaf after the specified byte offset.
    fn leaf_after(&self, cursor: usize) -> Option<Self> {
        if self.node.children().len() == 0 && cursor < self.offset + self.len() {
            return Some(self.clone());
        }

        let mut offset = self.offset;
        for child in self.children() {
            let len = child.len();
            if offset <= cursor && cursor < offset + len {
                return child.leaf_after(cursor);
            }
            offset += len;
        }

        None
    }

    /// Get the leaf at the specified byte offset.
    pub fn leaf_at(&self, cursor: usize, side: Side) -> Option<Self> {
        match side {
            Side::Before => self.leaf_before(cursor),
            Side::After => self.leaf_after(cursor),
        }
    }

    /// Find the rightmost contained non-trivia leaf.
    pub fn rightmost_leaf(&self) -> Option<Self> {
        if self.is_leaf() && !self.kind().is_trivia() {
            return Some(self.clone());
        }

        for child in self.children().rev() {
            if let Some(leaf) = child.rightmost_leaf() {
                return Some(leaf);
            }
        }

        None
    }

    /// Get the leftmost non-trivia leaf after this node.
    pub fn next_leaf(&self) -> Option<Self> {
        let mut node = self.clone();
        while let Some(next) = node.next_sibling() {
            if let Some(leaf) = next.leftmost_leaf() {
                return Some(leaf);
            }
            node = next;
        }
        self.parent()?.next_leaf()
    }
}

impl LinkedNode<'_> {
    #[inline]
    fn as_parent(&self) -> Option<SyntaxMode> {
        self.parent().and_then(|parent| parent.mode())
    }
    #[inline]
    fn as_parent_or_default(&self, default: SyntaxMode) -> Option<SyntaxMode> {
        self.parent().map_or(Some(default), |parent| parent.mode())
    }
    /// Get the `SyntaxMode` of this node.
    ///
    /// Would be `None` if can not determine.
    pub fn mode(&self) -> Option<SyntaxMode> {
        match self.kind() {
            SyntaxKind::End => None,
            SyntaxKind::Error => None,

            SyntaxKind::Shebang => None,
            SyntaxKind::LineComment => None,
            SyntaxKind::BlockComment => None,

            SyntaxKind::Markup => Some(SyntaxMode::Markup),
            SyntaxKind::Text => Some(SyntaxMode::Markup),
            // Either in math or in markup.
            SyntaxKind::Space => self.as_parent_or_default(SyntaxMode::Markup),
            // Either in math or in markup.
            SyntaxKind::Linebreak => self.as_parent_or_default(SyntaxMode::Markup),
            SyntaxKind::Parbreak => Some(SyntaxMode::Markup),
            // Either in math or in markup.
            SyntaxKind::Escape => self.as_parent_or_default(SyntaxMode::Markup),
            SyntaxKind::Shorthand => Some(SyntaxMode::Markup),
            SyntaxKind::SmartQuote => Some(SyntaxMode::Markup),
            SyntaxKind::Strong => Some(SyntaxMode::Markup),
            SyntaxKind::Emph => Some(SyntaxMode::Markup),
            SyntaxKind::Raw => Some(SyntaxMode::Markup),
            SyntaxKind::RawLang => Some(SyntaxMode::Markup),
            SyntaxKind::RawDelim => Some(SyntaxMode::Markup),
            SyntaxKind::RawTrimmed => Some(SyntaxMode::Markup),
            SyntaxKind::Link => Some(SyntaxMode::Markup),
            SyntaxKind::Label => Some(SyntaxMode::Markup),
            SyntaxKind::Ref => Some(SyntaxMode::Markup),
            SyntaxKind::RefMarker => Some(SyntaxMode::Markup),
            SyntaxKind::Heading => Some(SyntaxMode::Markup),
            SyntaxKind::HeadingMarker => Some(SyntaxMode::Markup),
            SyntaxKind::ListItem => Some(SyntaxMode::Markup),
            SyntaxKind::ListMarker => Some(SyntaxMode::Markup),
            SyntaxKind::EnumItem => Some(SyntaxMode::Markup),
            SyntaxKind::EnumMarker => Some(SyntaxMode::Markup),
            SyntaxKind::TermItem => Some(SyntaxMode::Markup),
            SyntaxKind::TermMarker => Some(SyntaxMode::Markup),
            SyntaxKind::Equation => Some(SyntaxMode::Math),

            SyntaxKind::Math => Some(SyntaxMode::Math),
            SyntaxKind::MathText => Some(SyntaxMode::Math),
            SyntaxKind::MathIdent => Some(SyntaxMode::Math),
            SyntaxKind::MathShorthand => Some(SyntaxMode::Math),
            SyntaxKind::MathAlignPoint => Some(SyntaxMode::Math),
            SyntaxKind::MathDelimited => Some(SyntaxMode::Math),
            SyntaxKind::MathAttach => Some(SyntaxMode::Math),
            SyntaxKind::MathPrimes => Some(SyntaxMode::Math),
            SyntaxKind::MathFrac => Some(SyntaxMode::Math),
            SyntaxKind::MathRoot => Some(SyntaxMode::Math),

            SyntaxKind::Hash => Some(SyntaxMode::Code),
            // Punctuations can be in all three modes.
            SyntaxKind::LeftBrace => self.as_parent(),
            SyntaxKind::RightBrace => self.as_parent(),
            SyntaxKind::LeftBracket => self.as_parent(),
            SyntaxKind::RightBracket => self.as_parent(),
            SyntaxKind::LeftParen => self.as_parent(),
            SyntaxKind::RightParen => self.as_parent(),
            SyntaxKind::Comma => self.as_parent(),
            SyntaxKind::Semicolon => self.as_parent(),
            SyntaxKind::Colon => self.as_parent(),
            // Either in code or in math.
            SyntaxKind::Star => self.as_parent(),
            // Either in math or in markup.
            SyntaxKind::Underscore => self.as_parent(),
            SyntaxKind::Dollar => Some(SyntaxMode::Math),
            SyntaxKind::Plus => Some(SyntaxMode::Code),
            SyntaxKind::Minus => Some(SyntaxMode::Code),
            // Either in code or in math.
            SyntaxKind::Slash => self.as_parent(),
            // Either in code or in math.
            SyntaxKind::Hat => self.as_parent(),
            // Either in code or in math.
            SyntaxKind::Dot => self.as_parent(),
            // Either in code or in markup.
            SyntaxKind::Eq => self.as_parent(),
            SyntaxKind::EqEq => Some(SyntaxMode::Code),
            SyntaxKind::ExclEq => Some(SyntaxMode::Code),
            SyntaxKind::Lt => Some(SyntaxMode::Code),
            SyntaxKind::LtEq => Some(SyntaxMode::Code),
            SyntaxKind::Gt => Some(SyntaxMode::Code),
            SyntaxKind::GtEq => Some(SyntaxMode::Code),
            SyntaxKind::PlusEq => Some(SyntaxMode::Code),
            SyntaxKind::HyphEq => Some(SyntaxMode::Code),
            SyntaxKind::StarEq => Some(SyntaxMode::Code),
            SyntaxKind::SlashEq => Some(SyntaxMode::Code),
            // Either in math or in code,
            // as a part of `Spread`
            SyntaxKind::Dots => self.as_parent(),
            SyntaxKind::Arrow => Some(SyntaxMode::Code),
            SyntaxKind::Root => Some(SyntaxMode::Math),
            SyntaxKind::Bang => Some(SyntaxMode::Math),

            SyntaxKind::Not => Some(SyntaxMode::Code),
            SyntaxKind::And => Some(SyntaxMode::Code),
            SyntaxKind::Or => Some(SyntaxMode::Code),
            SyntaxKind::None => Some(SyntaxMode::Code),
            SyntaxKind::Auto => Some(SyntaxMode::Code),
            SyntaxKind::Let => Some(SyntaxMode::Code),
            SyntaxKind::Set => Some(SyntaxMode::Code),
            SyntaxKind::Show => Some(SyntaxMode::Code),
            SyntaxKind::Context => Some(SyntaxMode::Code),
            SyntaxKind::If => Some(SyntaxMode::Code),
            SyntaxKind::Else => Some(SyntaxMode::Code),
            SyntaxKind::For => Some(SyntaxMode::Code),
            SyntaxKind::In => Some(SyntaxMode::Code),
            SyntaxKind::While => Some(SyntaxMode::Code),
            SyntaxKind::Break => Some(SyntaxMode::Code),
            SyntaxKind::Continue => Some(SyntaxMode::Code),
            SyntaxKind::Return => Some(SyntaxMode::Code),
            SyntaxKind::Import => Some(SyntaxMode::Code),
            SyntaxKind::Include => Some(SyntaxMode::Code),
            SyntaxKind::As => Some(SyntaxMode::Code),

            SyntaxKind::Code => Some(SyntaxMode::Code),
            // `Ident` is in math if it's parent is in math and it's previous sibling is not a `Hash`.
            // Otherwise, it's in code.
            SyntaxKind::Ident => {
                if self.parent().and_then(|parent| parent.mode())
                    == Some(SyntaxMode::Math)
                    && self
                        .prev_sibling_kind()
                        .is_none_or(|kind| kind != SyntaxKind::Hash)
                {
                    Some(SyntaxMode::Math)
                } else {
                    Some(SyntaxMode::Code)
                }
            }
            SyntaxKind::Bool => Some(SyntaxMode::Code),
            SyntaxKind::Int => Some(SyntaxMode::Code),
            SyntaxKind::Float => Some(SyntaxMode::Code),
            SyntaxKind::Numeric => Some(SyntaxMode::Code),
            SyntaxKind::Str => Some(SyntaxMode::Code),
            SyntaxKind::CodeBlock => Some(SyntaxMode::Code),
            SyntaxKind::ContentBlock => Some(SyntaxMode::Markup),
            SyntaxKind::Parenthesized => Some(SyntaxMode::Code),
            SyntaxKind::Array => Some(SyntaxMode::Code),
            SyntaxKind::Dict => Some(SyntaxMode::Code),
            // Either in code or in math,
            // it's in math only when it's within an `Args` who is in math.
            SyntaxKind::Named => {
                if self.parent_kind() == Some(SyntaxKind::Args)
                    && self.parent().and_then(|parent| parent.mode())
                        == Some(SyntaxMode::Math)
                {
                    Some(SyntaxMode::Math)
                } else {
                    Some(SyntaxMode::Code)
                }
            }
            SyntaxKind::Keyed => Some(SyntaxMode::Code),
            SyntaxKind::Unary => Some(SyntaxMode::Code),
            SyntaxKind::Binary => Some(SyntaxMode::Code),
            // Mode of `FieldAccess` and `FuncCall` is determined by the leftmost leaf
            // `callee` of `FuncCall` and leftmost expr of a `FieldAccess` chain.
            // It's either in code or in math, the latter case is only possible if
            // the leftmost leaf is a `MathIdent`.
            SyntaxKind::FieldAccess => {
                self.leftmost_leaf().map(|leaf| match leaf.kind() {
                    SyntaxKind::MathIdent => SyntaxMode::Math,
                    _ => SyntaxMode::Code,
                })
            }
            SyntaxKind::FuncCall => {
                self.leftmost_leaf().map(|leaf| match leaf.kind() {
                    // Eagerly return
                    SyntaxKind::MathIdent => SyntaxMode::Math,
                    _ => SyntaxMode::Code,
                })
            }
            // `Args` is always within a `FuncCall`.
            SyntaxKind::Args => self.as_parent(),
            // `Spread` is always within an `Args`.
            SyntaxKind::Spread => self.as_parent(),
            SyntaxKind::Closure => Some(SyntaxMode::Code),
            SyntaxKind::Params => Some(SyntaxMode::Code),
            SyntaxKind::LetBinding => Some(SyntaxMode::Code),
            SyntaxKind::SetRule => Some(SyntaxMode::Code),
            SyntaxKind::ShowRule => Some(SyntaxMode::Code),
            SyntaxKind::Contextual => Some(SyntaxMode::Code),
            SyntaxKind::Conditional => Some(SyntaxMode::Code),
            SyntaxKind::WhileLoop => Some(SyntaxMode::Code),
            SyntaxKind::ForLoop => Some(SyntaxMode::Code),
            SyntaxKind::ModuleImport => Some(SyntaxMode::Code),
            SyntaxKind::ImportItems => Some(SyntaxMode::Code),
            SyntaxKind::ImportItemPath => Some(SyntaxMode::Code),
            SyntaxKind::RenamedImportItem => Some(SyntaxMode::Code),
            SyntaxKind::ModuleInclude => Some(SyntaxMode::Code),
            SyntaxKind::LoopBreak => Some(SyntaxMode::Code),
            SyntaxKind::LoopContinue => Some(SyntaxMode::Code),
            SyntaxKind::FuncReturn => Some(SyntaxMode::Code),
            SyntaxKind::Destructuring => Some(SyntaxMode::Code),
            SyntaxKind::DestructAssignment => Some(SyntaxMode::Code),
        }
    }
}

impl Deref for LinkedNode<'_> {
    type Target = SyntaxNode;

    /// Dereference to a syntax node. Note that this shortens the lifetime, so
    /// you may need to use [`get()`](Self::get) instead in some situations.
    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl Debug for LinkedNode<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.node.fmt(f)
    }
}

/// An iterator over the children of a linked node.
pub struct LinkedChildren<'a> {
    parent: Rc<LinkedNode<'a>>,
    iter: std::iter::Enumerate<std::slice::Iter<'a, SyntaxNode>>,
    front: usize,
    back: usize,
}

impl<'a> Iterator for LinkedChildren<'a> {
    type Item = LinkedNode<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(index, node)| {
            let offset = self.front;
            self.front += node.len();
            LinkedNode {
                node,
                parent: Some(self.parent.clone()),
                index,
                offset,
            }
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl DoubleEndedIterator for LinkedChildren<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back().map(|(index, node)| {
            self.back -= node.len();
            LinkedNode {
                node,
                parent: Some(self.parent.clone()),
                index,
                offset: self.back,
            }
        })
    }
}

impl ExactSizeIterator for LinkedChildren<'_> {}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Source;

    #[track_caller]
    fn test_mode_internal(
        source: &Source,
        cursor: usize,
        expected_mode: Option<SyntaxMode>,
    ) {
        let node = LinkedNode::new(source.root()).leaf_at(cursor, Side::After).unwrap();
        assert_eq!(node.mode(), expected_mode);
    }

    #[track_caller]
    fn test_mode(source_str: &str, cursor: usize, expected_mode: Option<SyntaxMode>) {
        let source = Source::detached(source_str);
        test_mode_internal(&source, cursor, expected_mode);
    }

    #[track_caller]
    fn test_modes(
        source_str: &str,
        cursors: impl IntoIterator<Item = usize>,
        expected_mode: Option<SyntaxMode>,
    ) {
        let source = Source::detached(source_str);
        for cursor in cursors {
            test_mode_internal(&source, cursor, expected_mode);
        }
    }

    #[test]
    fn test_linked_node_mode() {
        // Shebang, LineComment, BlockComment
        test_mode("#! typ", 0, None);
        test_mode("// xxx", 0, None);
        test_mode("/* xxx */", 0, None);

        // Link
        test_mode("https://typst.org", 1, Some(SyntaxMode::Markup));

        // Text, Escaped
        test_modes("a\\bcd", [0, 1], Some(SyntaxMode::Markup));

        // Space, Linebreak, Parbreak
        test_modes("a   c\n\n d\\\nef", [1, 5, 9], Some(SyntaxMode::Markup));

        // SmartQuote
        test_modes("\"abc\"", [0, 4], Some(SyntaxMode::Markup));

        // Shorthand
        test_mode("a-?b", 1, Some(SyntaxMode::Markup));

        // Emph
        test_mode("_abcd_", 0, Some(SyntaxMode::Markup));

        // Raw, RawLang, RawDelim, RawTrimmed
        test_modes("```typ {}   ```", [0, 3], Some(SyntaxMode::Markup));
        test_mode("```  xx  ```", 3, Some(SyntaxMode::Markup));

        // Label
        test_mode("<label>", 0, Some(SyntaxMode::Markup));

        // Ref, RefMarker
        test_mode("@label", 0, Some(SyntaxMode::Markup));

        // Heading, HeadingMarker
        test_mode("= heading", 0, Some(SyntaxMode::Markup));

        // List, ListMarker
        test_mode("- heading", 0, Some(SyntaxMode::Markup));

        // Enum, EnumMarker
        test_mode("+ heading", 0, Some(SyntaxMode::Markup));

        // Term, TermMarker
        test_mode("/ heading", 0, Some(SyntaxMode::Markup));

        // Code Block
        test_modes("#{x;1}", [0, 1, 2, 3, 4], Some(SyntaxMode::Code));

        // Parenthesized
        test_mode("#(x)", 1, Some(SyntaxMode::Code));

        // Array
        test_modes("#(1,2,x)", [1, 2], Some(SyntaxMode::Code));

        // Dict, Named, Keyed
        test_modes("#(first:1, \"last\": 1)", [1, 7, 17], Some(SyntaxMode::Code));

        // Unary
        test_mode("#{-x}", 2, Some(SyntaxMode::Code));

        // Binary, Slash in code
        test_mode("#{a / b}", 4, Some(SyntaxMode::Code));

        // FieldAccess
        test_mode("#a.b", 2, Some(SyntaxMode::Code));

        // FieldAccess corner cases
        test_mode("#$$.at()", 3, Some(SyntaxMode::Code));
        test_mode("#[].at()", 3, Some(SyntaxMode::Code));

        // Args, FuncCall, Spread
        test_modes("#f(x, ..y)", [2, 4, 6], Some(SyntaxMode::Code));

        // Closure, Params
        test_modes("#{(x) => {}}", [3, 6], Some(SyntaxMode::Code));

        // LetBinding
        test_modes("#let x = 1", [1, 7], Some(SyntaxMode::Code));

        // SetRule
        test_modes("#set text()", [1, 4], Some(SyntaxMode::Code));

        // ShowRule
        test_modes("#show text : it => it", [1, 11], Some(SyntaxMode::Code));

        // Contextual
        test_modes("#context 1", [1, 8], Some(SyntaxMode::Code));

        // WhileLoop
        test_modes("#while true {break;continue;}", [1, 13, 19], Some(SyntaxMode::Code));

        // ForLoop
        test_modes("#for a in b {}", [1, 7], Some(SyntaxMode::Code));

        // Conditional
        test_modes("#if true {} else {}", [1, 12], Some(SyntaxMode::Code));

        // ModuleImport, ImportItems, ImportItemPath, RenamedImport
        test_modes(
            "#import \"lib.typ\" : a, b as d, e.f",
            [2, 8, 21, 25, 32],
            Some(SyntaxMode::Code),
        );

        // ModuleInclude
        test_mode("#include \"lib.typ\"", 1, Some(SyntaxMode::Code));

        // FuncReturn
        test_mode("#let f() = { return 1 }", 13, Some(SyntaxMode::Code));

        // Destructuring, DestructAssignment
        test_modes("#{(x, _, ..y) = (1, 2, ..z)}", [2, 14], Some(SyntaxMode::Code));

        // Code inside Markup
        test_mode("= #1.1", 3, Some(SyntaxMode::Code));

        // Dollar
        test_mode("$ $", 0, Some(SyntaxMode::Math));

        // MathIdent
        test_mode("$arrow$", 1, Some(SyntaxMode::Math));

        // MathText
        test_mode("$123.32$", 1, Some(SyntaxMode::Math));

        // Space in Math
        test_mode("$1 2 3$", 2, Some(SyntaxMode::Math));

        // Operator in Math
        test_modes("$+12 * y!", [1, 5, 8], Some(SyntaxMode::Math));

        // MathFrac, Slash in math
        test_mode("$1/2$", 2, Some(SyntaxMode::Math));

        // MathPrimes
        test_mode("$f''$", 2, Some(SyntaxMode::Math));

        // MathAttach, Hat, Underscore in math
        test_modes("$f_(x)^y$", [2, 3, 6, 7], Some(SyntaxMode::Math));

        // MathShorthand
        test_mode("$a>=b$", 2, Some(SyntaxMode::Math));

        // MathRoot
        test_modes("$√x$", [1, 2], Some(SyntaxMode::Math));

        // MathAlignment
        test_mode("$&x$", 1, Some(SyntaxMode::Math));

        // Escape
        test_mode("$\\#$", 1, Some(SyntaxMode::Math));

        // FuncCall in math
        test_modes(
            "$ff(x, sin(y), abs(z))$",
            [3, 4, 5, 7, 10, 15, 18],
            Some(SyntaxMode::Math),
        );

        // FuncCall in math: Dots, Spread, Named
        test_modes("$ff(..args, named: key)$", [4, 6, 16, 17], Some(SyntaxMode::Math));

        // FieldAccess in math
        test_mode("$arrow.r$", 6, Some(SyntaxMode::Math));

        // Hash
        test_mode("$#$", 1, Some(SyntaxMode::Code));

        // Ident
        test_mode("$#pa$", 2, Some(SyntaxMode::Code));

        // ContentBlock
        test_mode("$#[x]$", 2, Some(SyntaxMode::Markup));

        // CodeBlock
        test_mode("$#{x}$", 2, Some(SyntaxMode::Code));

        // FuncCall
        test_modes(
            "$#f(x, ..args, named: key)$",
            [3, 4, 5, 7, 9, 19, 20],
            Some(SyntaxMode::Code),
        );

        // Nested
        test_mode("$#$x$$", 2, Some(SyntaxMode::Math));

        // Context 1
        test_mode("$#context 1$", 10, Some(SyntaxMode::Code));

        // Context Space
        test_mode("$#context $", 9, Some(SyntaxMode::Code));

        // FieldAccess
        test_modes("$#std.align$", [5, 6], Some(SyntaxMode::Code));
    }

    #[test]
    fn test_linked_node() {
        let source = Source::detached("#set text(12pt, red)");

        // Find "text" with Before.
        let node = LinkedNode::new(source.root()).leaf_at(7, Side::Before).unwrap();
        assert_eq!(node.offset(), 5);
        assert_eq!(node.text(), "text");

        // Find "text" with After.
        let node = LinkedNode::new(source.root()).leaf_at(7, Side::After).unwrap();
        assert_eq!(node.offset(), 5);
        assert_eq!(node.text(), "text");

        // Go back to "#set". Skips the space.
        let prev = node.prev_sibling().unwrap();
        assert_eq!(prev.offset(), 1);
        assert_eq!(prev.text(), "set");
    }

    #[test]
    fn test_linked_node_non_trivia_leaf() {
        let source = Source::detached("#set fun(12pt, red)");
        let leaf = LinkedNode::new(source.root()).leaf_at(6, Side::Before).unwrap();
        let prev = leaf.prev_leaf().unwrap();
        assert_eq!(leaf.text(), "fun");
        assert_eq!(prev.text(), "set");

        // Check position 9 with Before.
        let source = Source::detached("#let x = 10");
        let leaf = LinkedNode::new(source.root()).leaf_at(9, Side::Before).unwrap();
        let prev = leaf.prev_leaf().unwrap();
        let next = leaf.next_leaf().unwrap();
        assert_eq!(prev.text(), "=");
        assert_eq!(leaf.text(), " ");
        assert_eq!(next.text(), "10");

        // Check position 9 with After.
        let source = Source::detached("#let x = 10");
        let leaf = LinkedNode::new(source.root()).leaf_at(9, Side::After).unwrap();
        let prev = leaf.prev_leaf().unwrap();
        assert!(leaf.next_leaf().is_none());
        assert_eq!(prev.text(), "=");
        assert_eq!(leaf.text(), "10");
    }
}
