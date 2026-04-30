use std::fmt::{self, Debug, Display, Formatter};
use std::ops::{Deref, Range};
use std::rc::Rc;
use std::sync::Arc;

use ecow::{EcoString, EcoVec, eco_format, eco_vec};

use crate::kind::ModeAfter;
use crate::{FileId, Span, SyntaxKind, SyntaxMode};

/// A node in the untyped syntax tree.
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct SyntaxNode(NodeKind);

/// The internal representations of a syntax node.
#[derive(Clone, Eq, PartialEq, Hash)]
enum NodeKind {
    /// A leaf node containing text.
    Leaf(LeafNode),
    /// A reference-counted inner node containing an array of children.
    Inner(Arc<InnerNode>),
    /// A warning message wrapped directly around another node.
    Warning(Arc<WarningWrapper>),
    /// An error node containing a message for some text.
    Error(Arc<ErrorNode>),
}

impl SyntaxNode {
    /// Create a new leaf node.
    pub fn leaf(kind: SyntaxKind, text: impl Into<EcoString>) -> Self {
        Self(NodeKind::Leaf(LeafNode::new(kind, text.into())))
    }

    /// Create a new inner node with children.
    pub fn inner(kind: SyntaxKind, children: Vec<SyntaxNode>) -> Self {
        Self(NodeKind::Inner(Arc::new(InnerNode::new(kind, children))))
    }

    /// Create a new error node with a user-presentable message for the given
    /// text. Note that the message is the first argument, and the text causing
    /// the error is the second argument.
    pub fn error(message: impl Into<EcoString>, text: impl Into<EcoString>) -> Self {
        Self(NodeKind::Error(Arc::new(ErrorNode::new(message.into(), text.into()))))
    }

    /// Add a warning message to an existing node.
    pub fn warn(&mut self, message: impl Into<EcoString>) {
        *self = Self(NodeKind::Warning(Arc::new(WarningWrapper::new(
            std::mem::take(self),
            message.into(),
        ))));
    }

    /// Add a user-presentable hint to an existing error or warning. Panics if
    /// this is not an error or warning node.
    #[track_caller]
    pub fn hint(&mut self, hint: impl Into<EcoString>) {
        match &mut self.0 {
            NodeKind::Leaf(_) | NodeKind::Inner(_) => {
                panic!("expected an error or warning node")
            }
            NodeKind::Warning(warn) => Arc::make_mut(warn).hints.push(hint.into()),
            NodeKind::Error(err) => Arc::make_mut(err).error.hints.push(hint.into()),
        }
    }

    /// Add mutliple hints while building an error or warning. Panics if
    /// this is not an error or warning node.
    #[track_caller]
    pub fn with_hints(mut self, hints: impl IntoIterator<Item = EcoString>) -> Self {
        match &mut self.0 {
            NodeKind::Leaf(_) | NodeKind::Inner(_) => {
                panic!("expected an error or warning node")
            }
            NodeKind::Warning(warn) => Arc::make_mut(warn).hints.extend(hints),
            NodeKind::Error(err) => Arc::make_mut(err).error.hints.extend(hints),
        }
        self
    }

    /// Create a dummy node of the given kind.
    ///
    /// Panics if `kind` is `SyntaxKind::Error`.
    #[track_caller]
    pub const fn placeholder(kind: SyntaxKind) -> Self {
        if matches!(kind, SyntaxKind::Error) {
            panic!("cannot create error placeholder");
        }
        Self(NodeKind::Leaf(LeafNode {
            kind,
            text: EcoString::new(),
            span: Span::detached(),
        }))
    }

    /// The type of the node.
    pub fn kind(&self) -> SyntaxKind {
        match &self.0 {
            NodeKind::Leaf(leaf) => leaf.kind,
            NodeKind::Inner(inner) => inner.kind,
            NodeKind::Warning(warn) => warn.child.kind(),
            NodeKind::Error(_) => SyntaxKind::Error,
        }
    }

    /// Return `true` if the length is 0.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The byte length of the node in the source text.
    pub fn len(&self) -> usize {
        match &self.0 {
            NodeKind::Leaf(leaf) => leaf.text.len(),
            NodeKind::Inner(inner) => inner.len,
            NodeKind::Warning(warn) => warn.child.len(),
            NodeKind::Error(err) => err.text.len(),
        }
    }

    /// The span of the node.
    pub fn span(&self) -> Span {
        match &self.0 {
            NodeKind::Leaf(leaf) => leaf.span,
            NodeKind::Inner(inner) => inner.span,
            NodeKind::Warning(warn) => warn.child.span(),
            NodeKind::Error(err) => err.error.span,
        }
    }

    /// The text of the node if it is a leaf or error node.
    ///
    /// Returns the empty string if this is an inner node.
    pub fn text(&self) -> &EcoString {
        static EMPTY: EcoString = EcoString::new();
        match &self.0 {
            NodeKind::Leaf(leaf) => &leaf.text,
            NodeKind::Inner(_) => &EMPTY,
            NodeKind::Warning(warn) => warn.child.text(),
            NodeKind::Error(err) => &err.text,
        }
    }

    /// Extract the text from the node.
    ///
    /// Builds the string if this is an inner node.
    pub fn into_text(self) -> EcoString {
        match self.0 {
            NodeKind::Leaf(leaf) => leaf.text,
            NodeKind::Error(err) => err.text.clone(),
            NodeKind::Inner(_) | NodeKind::Warning(_) => {
                let mut text = EcoString::with_capacity(self.len());
                self.traverse(|node| {
                    match &node.0 {
                        NodeKind::Inner(_) | NodeKind::Warning(_) => {}
                        NodeKind::Leaf(leaf) => text.push_str(&leaf.text),
                        NodeKind::Error(err) => text.push_str(&err.text),
                    }
                    node.children()
                });
                text
            }
        }
    }

    /// The node's children.
    pub fn children(&self) -> std::slice::Iter<'_, SyntaxNode> {
        match &self.0 {
            NodeKind::Leaf(_) | NodeKind::Error(_) => [].iter(),
            NodeKind::Inner(inner) => inner.children.iter(),
            NodeKind::Warning(warn) => warn.child.children(),
        }
    }

    /// Whether the node or its children contain an error and/or warning.
    pub fn erroneous(&self) -> Erroneous {
        match &self.0 {
            NodeKind::Leaf(_) => Erroneous::default(),
            NodeKind::Inner(inner) => inner.erroneous,
            NodeKind::Warning(warn) => Erroneous {
                errors: warn.child.erroneous().errors,
                warnings: true,
            },
            NodeKind::Error(_) => Erroneous { errors: true, warnings: false },
        }
    }

    /// The error and warning messages for this node and its descendants.
    pub fn errors_and_warnings(&self) -> Vec<SyntaxDiagnostic> {
        let mut vec = Vec::new();
        self.traverse(|node| match &node.0 {
            NodeKind::Inner(inner) if inner.erroneous.either() => inner.children.iter(),
            NodeKind::Inner(_) | NodeKind::Leaf(_) => [].iter(),
            NodeKind::Warning(warn) => {
                vec.push(warn.diagnostic());
                // We traverse into the wrapped child of the warning in case
                // that node is itself a warning.
                std::slice::from_ref(&warn.child).iter()
            }
            NodeKind::Error(err) => {
                vec.push(err.error.clone());
                [].iter()
            }
        });
        vec
    }

    /// Set a synthetic span for the node and all its descendants.
    pub fn synthesize(&mut self, span: Span) {
        match &mut self.0 {
            NodeKind::Leaf(leaf) => leaf.span = span,
            NodeKind::Inner(inner) => Arc::make_mut(inner).synthesize(span),
            NodeKind::Warning(warn) => Arc::make_mut(warn).child.synthesize(span),
            NodeKind::Error(err) => Arc::make_mut(err).error.span = span,
        }
    }

    /// Whether the two syntax nodes are the same apart from spans.
    pub fn spanless_eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (NodeKind::Leaf(a), NodeKind::Leaf(b)) => a.spanless_eq(b),
            (NodeKind::Inner(a), NodeKind::Inner(b)) => a.spanless_eq(b),
            (NodeKind::Warning(a), NodeKind::Warning(b)) => a.spanless_eq(b),
            (NodeKind::Error(a), NodeKind::Error(b)) => a.spanless_eq(b),
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
            NodeKind::Leaf(leaf) => leaf.kind = kind,
            NodeKind::Inner(inner) => Arc::make_mut(inner).kind = kind,
            NodeKind::Warning(warn) => Arc::make_mut(warn).child.convert_to_kind(kind),
            NodeKind::Error(_) => panic!("cannot convert error"),
        }
    }

    /// Convert the child to an error, if it isn't already one.
    pub(super) fn convert_to_error(&mut self, message: impl Into<EcoString>) {
        if !self.kind().is_error() {
            let text = std::mem::take(self).into_text();
            *self = SyntaxNode::error(message.into(), text);
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
            NodeKind::Leaf(leaf) => leaf.span = mid,
            NodeKind::Inner(inner) => Arc::make_mut(inner).numberize(id, None, within)?,
            NodeKind::Warning(warn) => Arc::make_mut(warn).child.numberize(id, within)?,
            NodeKind::Error(err) => Arc::make_mut(err).error.span = mid,
        }

        Ok(())
    }

    /// Traverse the tree in-order, calling `f` on each node and recursing on
    /// the returned nodes. Note that `f` can prune the traversal at any point
    /// by yielding `[].iter()` instead of the actual children slice of an inner
    /// node.
    fn traverse(&self, mut f: impl FnMut(&Self) -> std::slice::Iter<'_, Self>) {
        fn recursive_step(
            node: &SyntaxNode,
            f: &mut impl FnMut(&SyntaxNode) -> std::slice::Iter<'_, SyntaxNode>,
        ) {
            for child in f(node) {
                recursive_step(child, f);
            }
        }
        // We pass in `&mut impl FnMut` so our caller doesn't have to.
        recursive_step(self, &mut f);
    }

    /// Whether this is a leaf node.
    pub(super) fn is_leaf(&self) -> bool {
        match &self.0 {
            NodeKind::Leaf(_) => true,
            NodeKind::Inner(_) => false,
            NodeKind::Warning(warn) => warn.child.is_leaf(),
            // TODO: Should we also treat non-empty errors as leaves?
            NodeKind::Error(_) => false,
        }
    }

    /// Whether this is an inner node.
    pub(super) fn is_inner(&self) -> bool {
        match &self.0 {
            NodeKind::Leaf(_) => false,
            NodeKind::Inner(_) => true,
            NodeKind::Warning(warn) => warn.child.is_inner(),
            NodeKind::Error(_) => false,
        }
    }

    /// The number of descendants, including the node itself.
    pub(super) fn descendants(&self) -> usize {
        match &self.0 {
            NodeKind::Leaf(_) | NodeKind::Error(_) => 1,
            NodeKind::Inner(inner) => inner.descendants,
            NodeKind::Warning(warn) => warn.child.descendants(),
        }
    }

    /// The node's children, mutably.
    pub(super) fn children_mut(&mut self) -> &mut [SyntaxNode] {
        match &mut self.0 {
            NodeKind::Leaf(_) | NodeKind::Error(_) => &mut [],
            NodeKind::Inner(inner) => &mut Arc::make_mut(inner).children,
            NodeKind::Warning(warn) => Arc::make_mut(warn).child.children_mut(),
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
        match &mut self.0 {
            NodeKind::Leaf(_) | NodeKind::Error(_) => Ok(()),
            NodeKind::Inner(inner) => {
                Arc::make_mut(inner).replace_children(range, replacement)
            }
            NodeKind::Warning(warn) => {
                Arc::make_mut(warn).child.replace_children(range, replacement)
            }
        }
    }

    /// Update this node after changes were made to one of its children.
    pub(super) fn update_parent(
        &mut self,
        prev_len: usize,
        new_len: usize,
        prev_descendants: usize,
        new_descendants: usize,
    ) {
        match &mut self.0 {
            NodeKind::Leaf(_) | NodeKind::Error(_) => {}
            NodeKind::Inner(inner) => Arc::make_mut(inner).update_parent(
                prev_len,
                new_len,
                prev_descendants,
                new_descendants,
            ),
            NodeKind::Warning(warn) => Arc::make_mut(warn).child.update_parent(
                prev_len,
                new_len,
                prev_descendants,
                new_descendants,
            ),
        }
    }

    /// The upper bound of assigned numbers in this subtree.
    pub(super) fn upper(&self) -> u64 {
        match &self.0 {
            NodeKind::Leaf(leaf) => leaf.span.number() + 1,
            NodeKind::Inner(inner) => inner.upper,
            NodeKind::Warning(warn) => warn.child.upper(),
            NodeKind::Error(err) => err.error.span.number() + 1,
        }
    }
}

impl Debug for SyntaxNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match &self.0 {
            NodeKind::Leaf(leaf) => leaf.fmt(f),
            NodeKind::Inner(inner) => inner.fmt(f),
            NodeKind::Warning(warn) => warn.fmt(f),
            NodeKind::Error(err) => err.fmt(f),
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
    fn new(kind: SyntaxKind, text: EcoString) -> Self {
        debug_assert!(!kind.is_error());
        Self { kind, text, span: Span::detached() }
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
    /// Whether this node or any of its children contain errors or warnings.
    erroneous: Erroneous,
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
        let mut erroneous = Erroneous::default();

        for child in &children {
            len += child.len();
            descendants += child.descendants();
            erroneous = erroneous.or(child.erroneous());
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

        // Update our erroneous status after the replacement.
        // - If we weren't erroneous before, we can just use the replaced status
        // - Or, if our replacement has errors _and_ warnings, we can just use
        //   the replaced status
        // - Otherwise, we need to update based on all of the children _outside_
        //   the replaced range in case we replaced the erroneous children
        let replaced_erroneous = Erroneous::any(replacement);
        if !self.erroneous.either() || replaced_erroneous.both() {
            self.erroneous = replaced_erroneous;
        } else {
            self.erroneous = replaced_erroneous.or(Erroneous::or(
                Erroneous::any(&self.children[..range.start]),
                Erroneous::any(&self.children[range.end..]),
            ));
        }

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
        self.erroneous = Erroneous::any(&self.children);
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

/// Whether a node has errors and/or warnings in it or its children.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash)]
pub struct Erroneous {
    pub errors: bool,
    pub warnings: bool,
}

impl Erroneous {
    /// Whether there were errors or warnings.
    pub fn either(self) -> bool {
        self.errors | self.warnings
    }

    /// Whether there were both errors and warnings.
    pub fn both(self) -> bool {
        self.errors & self.warnings
    }

    /// Apply the `OR` of both fields separately.
    pub fn or(mut self, other: Self) -> Self {
        self.errors |= other.errors;
        self.warnings |= other.warnings;
        self
    }

    /// Whether any node in the given slice has errors or warnings.
    fn any(slice: &[SyntaxNode]) -> Self {
        slice
            .iter()
            .map(SyntaxNode::erroneous)
            .fold(Self::default(), Self::or)
    }
}

/// A syntactical error or warning. This is mainly used by converting it to a
/// `SourceDiagnostic` during evaluation.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct SyntaxDiagnostic {
    /// `true` if the diagnostic is an error, `false` if it's a warning.
    pub is_error: bool,
    /// The span targeted by the diagnostic.
    pub span: Span,
    /// The main diagnostic message.
    pub message: EcoString,
    /// Additional hints to the user, indicating how this issue could be avoided
    /// or worked around.
    pub hints: EcoVec<EcoString>,
}

/// An error node in the untyped syntax tree.
#[derive(Clone, Eq, PartialEq, Hash)]
struct ErrorNode {
    /// The source text of the node.
    text: EcoString,
    /// The syntax error.
    error: SyntaxDiagnostic,
}

impl ErrorNode {
    /// Create a new error node.
    fn new(message: EcoString, text: EcoString) -> Self {
        Self {
            text,
            error: SyntaxDiagnostic {
                is_error: true,
                span: Span::detached(),
                message,
                hints: eco_vec![],
            },
        }
    }

    /// Whether the two error nodes are the same apart from spans.
    fn spanless_eq(&self, other: &Self) -> bool {
        self.text == other.text
            && self.error.message == other.error.message
            && self.error.hints == other.error.hints
    }
}

impl Debug for ErrorNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if self.text.is_empty() && self.error.hints.is_empty() {
            write!(f, "Error: {:?}", self.error.message)
        } else {
            let mut out = f.debug_struct("Error:");
            out.field("text", &self.text);
            out.field("message", &self.error.message);
            for hint in &self.error.hints {
                out.field("hint", hint);
            }
            out.finish()
        }
    }
}

/// A warning in the untyped syntax tree.
///
/// Warnings transparently wrap another node and do not have spans or text of
/// their own. This means their child cannot be directly found or mutated, only
/// affected _through_ the warning. For this reason, methods on `SyntaxNode`
/// must be careful to not return a reference to the child directly.
#[derive(Clone, Eq, PartialEq, Hash)]
struct WarningWrapper {
    /// The wrapped syntax node.
    child: SyntaxNode,
    /// The warning message.
    message: EcoString,
    /// Additional hints to the user, indicating how this warning could be
    /// avoided or worked around.
    hints: EcoVec<EcoString>,
}

impl WarningWrapper {
    /// Wrap an existing syntax node in a warning node.
    fn new(child: SyntaxNode, message: EcoString) -> Self {
        Self { child, message, hints: eco_vec![] }
    }

    /// Produce the syntax diagnostic for a warning.
    fn diagnostic(&self) -> SyntaxDiagnostic {
        SyntaxDiagnostic {
            is_error: false,
            span: self.child.span(),
            message: self.message.clone(),
            hints: self.hints.clone(),
        }
    }

    /// Whether the two warnings are the same apart from spans.
    fn spanless_eq(&self, other: &Self) -> bool {
        self.message == other.message
            && self.hints == other.hints
            && self.child.spanless_eq(&other.child)
    }
}

impl Debug for WarningWrapper {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        /// This helper lets us output `hint: "msg"` instead of `"hint: msg"`
        /// while using `debug_set`.
        /// FUTURE: In Rust 1.93, we can use `fmt::from_fn` instead!
        struct FieldHelper<'a>(&'static str, &'a EcoString);
        impl Debug for FieldHelper<'_> {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                write!(f, "{}: {:?}", self.0, self.1)
            }
        }

        write!(f, "Warning: ")?;
        // Use `debug_set` instead of `debug_struct` so we don't have to add a
        // field name when outputting the child.
        let mut out = f.debug_set();
        out.entry(&FieldHelper("message", &self.message));
        for hint in &self.hints {
            out.entry(&FieldHelper("hint", hint));
        }
        out.entry(&self.child);
        out.finish()
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
    /// The underlying syntax node.
    node: &'a SyntaxNode,
    /// The parent of this node.
    parent: Option<Rc<Self>>,
    /// The index of this node in its parent's children array.
    index: usize,
    /// This node's byte offset in the source file.
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

        let mut node = self.node;
        // Descend past warnings when looking for an inner node.
        while let NodeKind::Warning(warn) = &node.0 {
            node = &warn.child;
        }
        if let NodeKind::Inner(inner) = &node.0 {
            // The parent of a subtree has a smaller span number than all of its
            // descendants. Therefore, we can bail out early if the target span's
            // number is smaller than our number.
            if span.number() < inner.span.number() {
                return None;
            }

            // Use `self.children()`, not `inner.children()` to preserve being
            // in a `LinkedNode`.
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

    /// Get the [`SyntaxMode`] we will be in when immediately after this node.
    ///
    /// Unlike some other `LinkedNode` methods, this does not treat all trivia
    /// the same: it returns `None` for both comments and the bodies of raw text
    /// and returns `Some` for whitespace (based on the parent's mode). The only
    /// other way this would return `None` is when inside a partial tree, i.e.
    /// one not rooted in `Markup`, `Math`, or `Code`.
    ///
    /// Also note that errors inherit the mode of their parent.
    pub fn mode_after(&self) -> Option<SyntaxMode> {
        match self.kind().mode_after() {
            ModeAfter::Known(mode) => Some(mode),
            // Comments and the bodies of raw text have no mode.
            ModeAfter::None => None,
            ModeAfter::Text if self.parent_kind() == Some(SyntaxKind::Raw) => None,
            ModeAfter::RawDelim if self.index == 0 => None,
            // Text not under raw is always markup.
            ModeAfter::Text => Some(SyntaxMode::Markup),
            // An opening dollar sign starts math mode.
            ModeAfter::Dollar if self.index == 0 => Some(SyntaxMode::Math),
            // Spaces at the left/right of an equation are still in math mode.
            ModeAfter::Space if self.parent_kind() == Some(SyntaxKind::Equation) => {
                Some(SyntaxMode::Math)
            }
            // The position after something embedded with a hash is still code.
            ModeAfter::Embeddable
                if self
                    .prev_sibling_with_trivia()
                    .is_some_and(|prev| prev.kind() == SyntaxKind::Hash) =>
            {
                Some(SyntaxMode::Code)
            }
            // Otherwise, we're simply based on our parent's mode.
            ModeAfter::Parent
            | ModeAfter::RawDelim
            | ModeAfter::Space
            | ModeAfter::Dollar
            | ModeAfter::Embeddable => self.parent_mode(),
        }
    }

    /// Get the [`SyntaxMode`] we will be in when immediately after the parent
    /// of this node.
    pub fn parent_mode(&self) -> Option<SyntaxMode> {
        self.parent().and_then(Self::mode_after)
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
        let parent = self.parent.as_ref()?;
        let children = parent.node.children().as_slice();
        let mut offset = self.offset;
        for (index, node) in children[..self.index].iter().enumerate().rev() {
            offset -= node.len();
            if !node.kind().is_trivia() {
                let parent = Some(parent.clone());
                return Some(Self { node, parent, index, offset });
            }
        }
        None
    }

    /// Get the first previous sibling node, including potential trivia.
    pub fn prev_sibling_with_trivia(&self) -> Option<Self> {
        let parent = self.parent.as_ref()?;
        let children = parent.node.children().as_slice();
        let (index, node) = children[..self.index].iter().enumerate().next_back()?;
        let offset = self.offset - node.len();
        let parent = Some(parent.clone());
        Some(Self { node, parent, index, offset })
    }

    /// Get the next non-trivia sibling node.
    pub fn next_sibling(&self) -> Option<Self> {
        let parent = self.parent.as_ref()?;
        let children = parent.node.children();
        let mut offset = self.offset + self.len();
        for (index, node) in children.enumerate().skip(self.index + 1) {
            if !node.kind().is_trivia() {
                let parent = Some(parent.clone());
                return Some(Self { node, parent, index, offset });
            }
            offset += node.len();
        }
        None
    }

    /// Get the next sibling node, including potential trivia.
    pub fn next_sibling_with_trivia(&self) -> Option<Self> {
        let parent = self.parent.as_ref()?;
        let children = parent.node.children();
        let (index, node) = children.enumerate().nth(self.index + 1)?;
        let offset = self.offset + self.len();
        let parent = Some(parent.clone());
        Some(Self { node, parent, index, offset })
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
    /// The parent whose children we're iterating.
    parent: Rc<LinkedNode<'a>>,
    /// The underlying syntax nodes and their indices.
    iter: std::iter::Enumerate<std::slice::Iter<'a, SyntaxNode>>,
    /// The byte offset of the next child's start.
    front: usize,
    /// The byte offset after the final child.
    back: usize,
}

impl<'a> Iterator for LinkedChildren<'a> {
    type Item = LinkedNode<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let (index, node) = self.iter.next()?;
        let offset = self.front;
        self.front += node.len();
        Some(LinkedNode {
            node,
            parent: Some(self.parent.clone()),
            index,
            offset,
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl DoubleEndedIterator for LinkedChildren<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let (index, node) = self.iter.next_back()?;
        self.back -= node.len();
        Some(LinkedNode {
            node,
            parent: Some(self.parent.clone()),
            index,
            offset: self.back,
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

    /// Test the debug output of a `SyntaxNode`.
    #[test]
    fn test_debug() {
        // A standard syntax tree:
        assert_eq!(
            format!("{:#?}", crate::parse("= Head <label>")),
            "\
Markup: 14 [
    Heading: 6 [
        HeadingMarker: \"=\",
        Space: \" \",
        Markup: 4 [
            Text: \"Head\",
        ],
    ],
    Space: \" \",
    Label: \"<label>\",
]"
        );
        // A basic syntax error:
        assert_eq!(
            format!("{:#?}", crate::parse("#")),
            "\
Markup: 1 [
    Hash: \"#\",
    Error: \"expected expression\",
]"
        );
        // A syntax error with multiple hints:
        assert_eq!(
            format!("{:#?}", crate::parse("##")),
            "\
Markup: 2 [
    Hash: \"#\",
    Error: {
        text: \"#\",
        message: \"the character `#` is not valid in code\",
        hint: \"the preceding hash is causing this to parse in code mode\",
        hint: \"try escaping the preceding hash: `\\\\#`\",
    },
]"
        );
        // A warning with a hint:
        assert_eq!(
            format!("{:#?}", crate::parse("**")),
            "\
Markup: 2 [
    Warning: {
        message: \"no text within stars\",
        hint: \"using multiple consecutive stars (e.g. **) has no additional effect\",
        Strong: 2 [
            Star: \"*\",
            Markup: 0,
            Star: \"*\",
        ],
    },
]"
        );
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
