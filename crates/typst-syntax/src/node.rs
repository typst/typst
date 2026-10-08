use std::cell::LazyCell;
use std::fmt::{self, Debug, Display, Formatter};
use std::ops::{Deref, Range};
use std::rc::Rc;
use std::sync::Arc;

use ecow::{EcoString, EcoVec, eco_format, eco_vec};
use typst_utils::debug;

use crate::kind::ModeAfter;
use crate::{
    DiagSpan, FileId, RangeMapper, Span, SpanKind, SpanNumber, Spanned, SubRange,
    SyntaxKind, SyntaxMode,
};

/// A node in the untyped syntax tree.
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct SyntaxNode {
    /// The underlying node data, potentially with wrapped warning messages.
    data: Node,
    /// The node's span, at the top-level to guarantee efficient access.
    span: Span,
    // We would love to move the `SyntaxKind` up here as well, but keeping it in
    // `Node` saves 8 bytes :/
}

/// The data for nodes in the tree, plus their [`SyntaxKind`]. May actually be a
/// warning message wrapping a child [`Node`].
///
/// Contains the [`SyntaxKind`] at the top-level for efficient access. This
/// requires being careful when mutating the kind, as warnings store this type
/// as their child, which duplicates the kind. Deduplicating the syntax kinds
/// would require a whole other enum type, and makes mutable access too painful.
///
/// The only other invariant for syntax kinds is that error nodes always contain
/// [`SyntaxKind::Error`], but leaf and inner nodes never do. The syntax kind of
/// a warning depends on what it wraps.
///
/// The simplest way to get the underlying data by descending into the children
/// of warnings is via a loop and match like below. The [`SyntaxNode::node_ref`]
/// helper does this for the by-reference case, but mutation is usually more
/// involved, so only gets the [`SyntaxNode::inner_and_span_mut`] helper.
/// ```ignore
/// let mut data = &mut node.data;
/// let value = loop {
///     match data {
///         Leaf(_, _) | Inner(_, _) | Error(_, _) => break "value",
///         Warning(warn, _) => data = &mut Arc::make_mut(warn).child,
///     }
/// };
/// ```
#[derive(Clone, Eq, PartialEq, Hash)]
enum Node {
    Leaf(EcoString, SyntaxKind),
    Inner(Arc<InnerNode>, SyntaxKind),
    Error(Arc<ErrorNode>, SyntaxKind),
    Warning(Arc<WarningWrapper>, SyntaxKind),
}

/// Data attached to a node, accessed by reference via [`SyntaxNode::node_ref`].
enum NodeRef<'a> {
    Leaf(&'a EcoString),
    Inner(&'a Arc<InnerNode>),
    Error(&'a Arc<ErrorNode>),
}

impl SyntaxNode {
    /// Access the underlying node data by reference, descending past warnings.
    fn node_ref(&self) -> NodeRef<'_> {
        let mut data = &self.data;
        loop {
            match data {
                Node::Leaf(text, _) => break NodeRef::Leaf(text),
                Node::Inner(inner, _) => break NodeRef::Inner(inner),
                Node::Error(err, _) => break NodeRef::Error(err),
                Node::Warning(warn, _) => data = &warn.child,
            }
        }
    }

    /// Access an inner node and the node's overall span mutably, descending
    /// past warnings. If this only returned the `&mut InnerNode`, the caller
    /// wouldn't be able to also get a mutable reference to the span since the
    /// inner node would borrow mutably from `self`.
    fn inner_and_span_mut(&mut self) -> Option<(&mut InnerNode, &mut Span)> {
        let mut data = &mut self.data;
        loop {
            match data {
                Node::Leaf(_, _) | Node::Error(_, _) => break None,
                Node::Inner(inner, _) => {
                    break Some((Arc::make_mut(inner), &mut self.span));
                }
                Node::Warning(warn, _) => data = &mut Arc::make_mut(warn).child,
            }
        }
    }

    /// Access the hints for an error or warning mutably.
    fn hints_mut(&mut self) -> Option<&mut EcoVec<(EcoString, Option<SubRange>)>> {
        match &mut self.data {
            Node::Leaf(_, _) | Node::Inner(_, _) => None,
            Node::Error(err, _) => Some(&mut Arc::make_mut(err).hints),
            Node::Warning(warn, _) => Some(&mut Arc::make_mut(warn).hints),
        }
    }
}

impl SyntaxNode {
    /// Create a new leaf node.
    #[track_caller]
    pub fn leaf(kind: SyntaxKind, text: impl Into<EcoString>) -> Self {
        debug_assert!(!kind.is_error());
        Self {
            data: Node::Leaf(text.into(), kind),
            span: Span::detached(),
        }
    }

    /// Create a new inner node with children.
    #[track_caller]
    pub fn inner(kind: SyntaxKind, children: Vec<SyntaxNode>) -> Self {
        debug_assert!(!kind.is_error());
        Self {
            data: Node::Inner(Arc::new(InnerNode::new(children)), kind),
            span: Span::detached(),
        }
    }

    /// Create a new error node with a user-presentable message for the given
    /// text. Note that the message is the first argument, and the text causing
    /// the error is the second argument.
    pub fn error(message: impl Into<EcoString>, text: impl Into<EcoString>) -> Self {
        Self {
            data: Node::Error(
                Arc::new(ErrorNode::new(message.into(), text.into())),
                SyntaxKind::Error,
            ),
            span: Span::detached(),
        }
    }

    /// Add a warning message to an existing node.
    ///
    /// Calling this requires that we either recursively update the parent
    /// node's `diagnosis` (as in reparsing) or we are in the process of
    /// constructing the parent (as in normal parsing).
    pub fn warn(&mut self, message: impl Into<EcoString>) {
        let kind = self.kind();
        let child = std::mem::replace(&mut self.data, Node::Leaf(EcoString::new(), kind));
        let warn = Arc::new(WarningWrapper::new(child, None, message.into()));
        self.data = Node::Warning(warn, kind);
    }

    /// Add a warning around this node at a particular sub-range of the node's
    /// text. Panics if the range is empty or exceeds the length of the wrapped
    /// text.
    ///
    /// Calling this requires that we either recursively update the parent
    /// node's `diagnosis` (as in reparsing) or we are in the process of
    /// constructing the parent (as in normal parsing).
    #[track_caller]
    pub fn warn_at(
        &mut self,
        Range { start, end }: Range<usize>,
        message: impl Into<EcoString>,
    ) {
        assert!(end <= self.len()); // This isn't checked by `SubRange::new`.
        let sub_range = SubRange::new(start, end).expect("a valid sub-range");
        let kind = self.kind();
        let child = std::mem::replace(&mut self.data, Node::Leaf(EcoString::new(), kind));
        let warn = Arc::new(WarningWrapper::new(child, Some(sub_range), message.into()));
        self.data = Node::Warning(warn, kind);
    }

    /// Add a user-presentable hint to an existing error or warning. Panics if
    /// this is not an error or warning.
    #[track_caller]
    pub fn hint(&mut self, hint: impl Into<EcoString>) {
        let hints = self.hints_mut().expect("expected an error or warning");
        hints.push((hint.into(), None));
    }

    /// Add a user-presentable hint to an existing error or warning at a
    /// sub-range of the text. Panics if the range is empty or exceeds the
    /// length of the wrapped text. Panics if this is not an error or warning
    /// node.
    #[track_caller]
    pub fn hint_at(
        &mut self,
        Range { start, end }: Range<usize>,
        hint: impl Into<EcoString>,
    ) {
        assert!(end <= self.len()); // This isn't checked by `SubRange::new`.
        let sub_range = SubRange::new(start, end).expect("a valid sub-range");
        let hints = self.hints_mut().expect("expected an error or warning");
        hints.push((hint.into(), Some(sub_range)));
    }

    /// Add multiple hints while building an error or warning. Panics if this is
    /// not an error or warning.
    #[track_caller]
    pub fn with_hints(mut self, new_hints: impl IntoIterator<Item = EcoString>) -> Self {
        let hints = self.hints_mut().expect("expected an error or warning");
        let iter = new_hints.into_iter().map(|h| (h, None));
        hints.extend(iter);
        self
    }

    /// Create a dummy node of the given kind.
    ///
    /// Panics if `kind` is [`SyntaxKind::Error`].
    #[track_caller]
    pub const fn placeholder(kind: SyntaxKind) -> Self {
        if kind.is_error() {
            panic!("cannot create error placeholder");
        }
        Self {
            data: Node::Leaf(EcoString::new(), kind),
            span: Span::detached(),
        }
    }

    /// The type of the node.
    pub fn kind(&self) -> SyntaxKind {
        match self.data {
            Node::Leaf(_, kind)
            | Node::Inner(_, kind)
            | Node::Error(_, kind)
            | Node::Warning(_, kind) => kind,
        }
    }

    /// Return `true` if the length is 0.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The byte length of the node in the source text.
    pub fn len(&self) -> usize {
        match self.node_ref() {
            NodeRef::Leaf(text) => text.len(),
            NodeRef::Inner(inner) => inner.len,
            NodeRef::Error(err) => err.text.len(),
        }
    }

    /// The span of the node.
    pub fn span(&self) -> Span {
        self.span
    }

    /// The text of the node if it is a leaf or error node.
    ///
    /// Returns the empty string if this is an inner node.
    pub fn leaf_text(&self) -> &EcoString {
        static EMPTY: EcoString = EcoString::new();
        match self.node_ref() {
            NodeRef::Leaf(text) => text,
            NodeRef::Inner(_) => &EMPTY,
            NodeRef::Error(err) => &err.text,
        }
    }

    /// Clone the full text from the node. If this is an inner node, it will
    /// traverse the tree to build the text which may be expensive.
    pub fn full_text(&self) -> EcoString {
        match &self.data {
            Node::Leaf(leaf, _) => leaf.clone(),
            Node::Error(err, _) => err.text.clone(),
            Node::Inner(_, _) | Node::Warning(_, _) => {
                let mut buffer = EcoString::with_capacity(self.len());
                self.traverse(|node| {
                    match node.node_ref() {
                        NodeRef::Leaf(text) => buffer.push_str(text),
                        NodeRef::Inner(_) => {}
                        NodeRef::Error(err) => buffer.push_str(&err.text),
                    }
                    node.children()
                });
                buffer
            }
        }
    }

    /// The node's children.
    pub fn children(&self) -> std::slice::Iter<'_, SyntaxNode> {
        match self.node_ref() {
            NodeRef::Leaf(_) | NodeRef::Error(_) => [].iter(),
            NodeRef::Inner(inner) => inner.children.iter(),
        }
    }

    /// Whether the node has diagnostic errors and/or warnings in it or its
    /// children. [`Diagnosis`] has public fields, so you can write
    /// `node.diagnosis().errors` to determine if a node is erroneous.
    ///
    /// This can be used to determine whether [`Self::errors_and_warnings`] will
    /// return an empty vector without traversing the tree if it will not.
    pub fn diagnosis(&self) -> Diagnosis {
        let diagnosis = match self.node_ref() {
            NodeRef::Leaf(_) => Diagnosis::default(),
            NodeRef::Inner(inner) => inner.diagnosis,
            NodeRef::Error(_) => Diagnosis { errors: true, warnings: false },
        };
        match &self.data {
            Node::Warning(_, _) => Diagnosis { warnings: true, errors: diagnosis.errors },
            _ => diagnosis,
        }
    }

    /// The error and warning diagnostics for this node and its descendants.
    pub fn errors_and_warnings(&self) -> (Vec<SyntaxDiagnostic>, Vec<SyntaxDiagnostic>) {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        self.traverse(|node| {
            let mut data = &node.data;
            loop {
                match data {
                    Node::Inner(inner, _) if inner.diagnosis.either() => {
                        break inner.children.iter();
                    }
                    Node::Leaf(_, _) | Node::Inner(_, _) => break [].iter(),
                    Node::Error(err, _) => {
                        errors.push(err.diagnostic(node.span));
                        break [].iter();
                    }
                    Node::Warning(warn, _) => {
                        warnings.push(warn.diagnostic(node.span));
                        data = &warn.child;
                    }
                }
            }
        });
        (errors, warnings)
    }

    /// Set a synthetic span for the node and all its descendants, and add hints
    /// with the original indices to any syntax errors or warnings.
    pub fn synthesize(&mut self, span: Span) {
        self.synthesize_with(
            0,
            &|_, _| span,
            // Sub-ranges are removed since the overall range is not accurate.
            &|_, sub_range| *sub_range = None,
            &|offset, len| {
                Some(if len == 0 {
                    eco_format!("at index `{offset}`")
                } else {
                    eco_format!("from index `{offset}` to `{}`", offset + len)
                })
            },
        );
    }

    /// Set a raw range span for each node.
    ///
    /// The range is determined by mapping the node's ranges through the given
    /// `mapper`.
    ///
    /// Returns an error with the mapper's length if it was shorter than the
    /// length of the source text.
    pub fn synthesize_mapped(
        &mut self,
        id: FileId,
        mapper: &RangeMapper,
    ) -> Result<(), EcoString> {
        if self.len() > mapper.total_len() {
            // TODO: Should we error if not exactly equal?
            return Err(eco_format!(
                "text length ({}) is greater than mapper length ({})",
                self.len(),
                mapper.total_len(),
            ));
        }
        self.synthesize_with(
            0,
            &|offset, len| Span::from_range(id, mapper.map(offset..offset + len)),
            &|offset, sub_range| {
                if let Some(sr) = sub_range {
                    *sr = mapper.map_sub_range(offset, *sr);
                }
            },
            &|_, _| None,
        );
        Ok(())
    }

    /// Set a custom span for each node given its offset and length, and update
    /// any sub-ranges based on their offset.
    ///
    /// Should be called with `offset = 0` on the root node.
    fn synthesize_with(
        &mut self,
        mut offset: usize,
        map_span: &impl Fn(usize, usize) -> Span,
        update_sub_range: &impl Fn(usize, &mut Option<SubRange>),
        add_hint: &impl Fn(usize, usize) -> Option<EcoString>,
    ) {
        let len = self.len();
        self.span = map_span(offset, len);
        let mut data = &mut self.data;
        loop {
            match data {
                Node::Leaf(_, _) => break,
                Node::Inner(inner, _) => {
                    let inner = Arc::make_mut(inner);
                    inner.upper = self.span.number();
                    for child in &mut inner.children {
                        child.synthesize_with(
                            offset,
                            map_span,
                            update_sub_range,
                            add_hint,
                        );
                        offset += child.len();
                    }
                    break;
                }
                Node::Error(err, _) => {
                    let err = Arc::make_mut(err);
                    for (_hint, sub_range) in err.hints.make_mut() {
                        update_sub_range(offset, sub_range);
                    }
                    if let Some(hint) = add_hint(offset, len) {
                        err.hints.push((hint, None));
                    }
                    break;
                }
                Node::Warning(warn, _) => {
                    let warn = Arc::make_mut(warn);
                    let (orig_offset, orig_len) = match warn.sub_range {
                        Some(sub_range) => {
                            let range = sub_range.to_absolute(offset);
                            (range.start, range.len())
                        }
                        None => (offset, len),
                    };
                    update_sub_range(offset, &mut warn.sub_range);
                    for (_hint, sub_range) in warn.hints.make_mut() {
                        update_sub_range(offset, sub_range);
                    }
                    if let Some(hint) = add_hint(orig_offset, orig_len) {
                        warn.hints.push((hint, None));
                    }
                    data = &mut warn.child;
                }
            }
        }
    }

    /// Whether the two syntax nodes are the same apart from spans.
    pub fn spanless_eq(&self, other: &Self) -> bool {
        self.kind() == other.kind() && {
            let mut data_a = &self.data;
            let mut data_b = &other.data;
            loop {
                match (data_a, data_b) {
                    (Node::Leaf(a, _), Node::Leaf(b, _)) => break a == b,
                    (Node::Inner(a, _), Node::Inner(b, _)) => {
                        break a.spanless_eq(b);
                    }
                    (Node::Error(a, _), Node::Error(b, _)) => break a == b,
                    (Node::Warning(a, _), Node::Warning(b, _))
                        if a.sub_range == b.sub_range
                            && a.message == b.message
                            && a.hints == b.hints =>
                    {
                        data_a = &a.child;
                        data_b = &b.child;
                    }
                    _ => break false,
                }
            }
        }
    }
}

impl SyntaxNode {
    /// Convert the child to another kind.
    ///
    /// Panics if trying to convert to or from an error.
    #[track_caller]
    pub(super) fn convert_to_kind(&mut self, new_kind: SyntaxKind) {
        if new_kind.is_error() {
            panic!("cannot convert to an error, use `convert_to_error` instead");
        } else if self.kind().is_error() {
            // `.kind()` checks both errors and warnings that wrap errors.
            panic!("cannot convert an error to a different kind");
        }
        // Must assign through warnings as well, since they duplicate the kind.
        let mut data = &mut self.data;
        loop {
            match data {
                Node::Leaf(_, kind) | Node::Inner(_, kind) => {
                    *kind = new_kind;
                    break;
                }
                Node::Error(_, _) => unreachable!(),
                Node::Warning(warn, kind) => {
                    *kind = new_kind;
                    data = &mut Arc::make_mut(warn).child;
                }
            }
        }
    }

    /// Convert the child to an error, if it isn't already one.
    pub(super) fn convert_to_error(&mut self, message: impl Into<EcoString>) {
        if !self.kind().is_error() {
            let text = std::mem::take(self).full_text();
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
                text = self.leaf_text(),
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
            Err(Unnumberable)
        } else if let Some((inner, span)) = self.inner_and_span_mut() {
            inner.numberize(span, id, None, within)
        } else {
            self.span =
                Span::from_number(id, SpanNumber((within.start + within.end) / 2));
            Ok(())
        }
    }

    /// Traverse the tree in-order, calling `f` on each node before recursing on
    /// the returned node slice (normally just `node.children()`).
    ///
    /// Note that `f` can prune the traversal at any point by yielding
    /// `[].iter()` instead of the actual children slice of an inner node.
    pub(super) fn traverse(
        &self,
        mut f: impl FnMut(&Self) -> std::slice::Iter<'_, Self>,
    ) {
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
        matches!(self.node_ref(), NodeRef::Leaf(_))
        // TODO: Should we also treat non-empty errors as leaves?
    }

    /// Whether this is an inner node.
    pub(super) fn is_inner(&self) -> bool {
        matches!(self.node_ref(), NodeRef::Inner(_))
    }

    /// The number of descendants, including the node itself.
    pub(super) fn descendants(&self) -> usize {
        match self.node_ref() {
            NodeRef::Leaf(_) | NodeRef::Error(_) => 1,
            NodeRef::Inner(inner) => inner.descendants,
        }
    }

    /// The node's children, mutably.
    pub(super) fn children_mut(&mut self) -> &mut [SyntaxNode] {
        if let Some((inner, _)) = self.inner_and_span_mut() {
            &mut inner.children
        } else {
            &mut []
        }
    }

    /// Replace a range of children and update this node.
    /// [`Self::renumber_children`] should be called immediately after this.
    ///
    /// To increase the stability of span numbers, this does not replace nodes
    /// at the front/back that are equal to the previous nodes. Returns the
    /// final indices of the inserted nodes.
    pub(super) fn replace_children(
        &mut self,
        range: Range<usize>,
        replacement: Vec<SyntaxNode>,
    ) -> Range<usize> {
        if let Some((inner, _)) = self.inner_and_span_mut() {
            inner.replace_children(range, replacement)
        } else {
            unreachable!();
        }
    }

    /// Try to assign span numbers to a range of children. If there are not
    /// enough numbers, retries taking exponentially more children into account.
    pub(super) fn renumber_children(&mut self, range: Range<usize>) -> NumberingResult {
        if let Some((inner, span)) = self.inner_and_span_mut() {
            inner.renumber_children(span, range)
        } else {
            Ok(())
        }
    }

    /// Update this node after one or more of its children were replaced or
    /// updated.
    pub(super) fn update_parent(
        &mut self,
        prev_len: usize,
        new_len: usize,
        prev_descendants: usize,
        new_descendants: usize,
        prev_diagnosis: Diagnosis,
        new_diagnosis: Diagnosis,
    ) {
        if let Some((inner, _)) = self.inner_and_span_mut() {
            inner.update_parent(
                prev_len,
                new_len,
                prev_descendants,
                new_descendants,
                prev_diagnosis,
                new_diagnosis,
            );
        }
    }

    /// The upper bound of assigned numbers in this subtree.
    pub(super) fn upper(&self) -> u64 {
        match self.node_ref() {
            NodeRef::Leaf(_) | NodeRef::Error(_) => self.span.number() + 1,
            NodeRef::Inner(inner) => inner.upper,
        }
    }
}

impl Debug for SyntaxNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.data.debug_fmt_span(f, self.span)
    }
}

impl Node {
    /// Write with debug formatting using the node's overall span.
    fn debug_fmt_span(&self, f: &mut Formatter, span: Span) -> fmt::Result {
        match self {
            Node::Leaf(text, kind) => write!(f, "{kind:?}: {text:?}"),
            Node::Inner(inner, kind) => inner.debug_fmt_kind(f, *kind),
            Node::Error(err, _) => err.debug_fmt_span(f, span),
            Node::Warning(warn, _) => warn.debug_fmt_span(f, span),
        }
    }
}

impl Default for SyntaxNode {
    fn default() -> Self {
        Self::leaf(SyntaxKind::End, EcoString::new())
    }
}

/// An inner node in the untyped syntax tree.
#[derive(Clone, Eq, PartialEq, Hash)]
struct InnerNode {
    /// The byte length of the node in the source.
    len: usize,
    /// The number of nodes in the whole subtree, including this node.
    descendants: usize,
    /// Whether this node or any of its children contain an error/warning
    /// diagnostic.
    diagnosis: Diagnosis,
    /// The upper bound of this node's numbering range.
    upper: u64,
    /// This node's children, losslessly make up this node.
    children: Vec<SyntaxNode>,
}

impl InnerNode {
    /// Create a new inner node with the given children.
    fn new(children: Vec<SyntaxNode>) -> Self {
        let mut len = 0;
        let mut descendants = 1;
        let mut diagnosis = Diagnosis::default();

        for child in &children {
            len += child.len();
            descendants += child.descendants();
            diagnosis = diagnosis.or(child.diagnosis());
        }

        Self { len, descendants, diagnosis, upper: 0, children }
    }

    /// Assign span numbers `within` an interval to this node's subtree or just
    /// a `range` of its children.
    fn numberize(
        &mut self,
        span: &mut Span,
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
            stride = space / descendants as u64;
            if stride == 0 {
                return Err(Unnumberable);
            }
        }

        // Number the node itself.
        let mut start = within.start;
        if range.is_none() {
            let end = start + stride;
            *span = Span::from_number(id, SpanNumber((start + end) / 2));
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
        self.len == other.len
            && self.descendants == other.descendants
            && self.diagnosis == other.diagnosis
            && self.children.len() == other.children.len()
            && self
                .children
                .iter()
                .zip(&other.children)
                .all(|(a, b)| a.spanless_eq(b))
    }

    /// Replace a range of children and calls [`Self::update_parent`].
    /// [`Self::renumber_children`] should be called immediately after this.
    ///
    /// To increase the stability of span numbers, this does not replace nodes
    /// at the front/back that are equal to the previous nodes. Returns the
    /// final indices of the inserted nodes.
    fn replace_children(
        &mut self,
        mut range: Range<usize>,
        mut replacement: Vec<SyntaxNode>,
    ) -> Range<usize> {
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

        // Compute child information for the `update_parent` call.
        let new = &replacement[replacement_range.clone()];
        let new_len = new.iter().map(SyntaxNode::len).sum::<usize>();
        let new_desc = new.iter().map(SyntaxNode::descendants).sum::<usize>();
        let new_diag = Diagnosis::any(new);

        let prev = &self.children[range.clone()];
        let prev_len = prev.iter().map(SyntaxNode::len).sum::<usize>();
        let prev_desc = prev.iter().map(SyntaxNode::descendants).sum::<usize>();
        let prev_diag = Diagnosis::any(prev);

        let replaced_range = range.start..range.start + new.len();

        self.children.splice(range, replacement.drain(replacement_range));
        self.update_parent(prev_len, new_len, prev_desc, new_desc, prev_diag, new_diag);

        replaced_range
    }

    /// Try to assign span numbers to a range of children. If there are not
    /// enough numbers, retries taking exponentially more children into account.
    fn renumber_children(
        &mut self,
        span: &mut Span,
        range: Range<usize>,
    ) -> NumberingResult {
        let Some(id) = span.id() else { return Err(Unnumberable) };

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
                .map_or(span.number() + 1, |child| child.upper());

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
            if self.numberize(span, id, Some(renumber), within).is_ok() {
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

    /// Update this node after one or more of its children were replaced or
    /// updated. The new children must already be in `self.children`.
    ///
    /// `prev_*` variables should come from the previous range of children.
    /// `new_*` are for the new range of children.
    fn update_parent(
        &mut self,
        prev_len: usize,
        new_len: usize,
        prev_descendants: usize,
        new_descendants: usize,
        prev_diagnosis: Diagnosis,
        new_diagnosis: Diagnosis,
    ) {
        // Length and descendants are numerical, can update with algebra.
        // Subtract the previous values first to avoid potential overflow.
        self.len = (self.len - prev_len) + new_len;
        self.descendants = (self.descendants - prev_descendants) + new_descendants;

        // Our diagnosis is the `OR` of all our children's diagnoses.
        if !self.diagnosis.either() || new_diagnosis.both() {
            // We can avoid iterating our children if our previous overall state
            // had no errors/warnings, or if the new children have both.
            self.diagnosis = new_diagnosis;
        } else if prev_diagnosis != new_diagnosis {
            // Otherwise, we recompute using all the children since we don't
            // know if there were multiple children with errors or warnings.
            self.diagnosis = Diagnosis::any(&self.children);
        }
    }

    /// Write with debug formatting using the node's syntax kind.
    fn debug_fmt_kind(&self, f: &mut Formatter, kind: SyntaxKind) -> fmt::Result {
        write!(f, "{kind:?}: {}", self.len)?;
        if !self.children.is_empty() {
            f.write_str(" ")?;
            f.debug_list().entries(&self.children).finish()?;
        }
        Ok(())
    }
}

/// Whether a node has diagnostic errors and/or warnings in it or its children.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash)]
pub struct Diagnosis {
    pub errors: bool,
    pub warnings: bool,
}

impl Diagnosis {
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
            .map(SyntaxNode::diagnosis)
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
    pub span: DiagSpan,
    /// The main diagnostic message.
    pub message: EcoString,
    /// Additional hints to the user indicating how this issue could be avoided
    /// or worked around.
    pub hints: EcoVec<Spanned<EcoString, DiagSpan>>,
}

/// An error node in the untyped syntax tree.
#[derive(Clone, Eq, PartialEq, Hash)]
struct ErrorNode {
    /// The source text of the node.
    text: EcoString,
    /// The error message.
    message: EcoString,
    /// Additional hints to the user indicating how this error could be avoided
    /// or worked around.
    hints: EcoVec<(EcoString, Option<SubRange>)>,
}

impl ErrorNode {
    /// Create a new error node.
    fn new(message: EcoString, text: EcoString) -> Self {
        Self { text, message, hints: eco_vec![] }
    }

    /// Produce the syntax diagnostic for an error.
    fn diagnostic(&self, span: Span) -> SyntaxDiagnostic {
        SyntaxDiagnostic {
            is_error: true,
            span: span.into(),
            message: self.message.clone(),
            hints: build_diagnostic_hints(span, &self.hints),
        }
    }

    /// Write with debug formatting using the node's overall span.
    ///
    /// See [`WarningWrapper::debug_fmt_span`] for more explanation.
    fn debug_fmt_span(&self, f: &mut Formatter, span: Span) -> fmt::Result {
        if self.text.is_empty() && self.hints.is_empty() {
            write!(f, "Error: {:?}", self.message)
        } else {
            let mut out = f.debug_struct("Error:");
            out.field("text", &self.text);
            out.field("message", &self.message);
            for (hint, sub_range) in &self.hints {
                let field = if let Some(sr) = sub_range {
                    if let SpanKind::Range { id: _, range } = span.get() {
                        let new_range = sr.to_absolute(range.start);
                        &format!("hint @mapped({new_range:?})")
                    } else {
                        let selected = &self.text[sr.to_relative()];
                        &format!("hint @text({selected:?})")
                    }
                } else {
                    "hint"
                };
                out.field(field, hint);
            }
            out.finish()
        }
    }
}

/// A warning message wrapped around a node in the tree.
///
/// Warnings transparently wrap another node and do not have spans or text of
/// their own. This means their child cannot be directly found or mutated, only
/// affected _through_ the warning, usually via the [`SyntaxNode::node_ref`] and
/// [`SyntaxNode::inner_and_span_mut`] methods.
#[derive(Clone, Eq, PartialEq, Hash)]
struct WarningWrapper {
    /// The wrapped node data.
    child: Node,
    /// A relative sub-range for targeting text not grouped by an existing span.
    ///
    /// Warnings may need to target a range of text that isn't actually grouped
    /// by the syntax tree, this sub-range can select that text.
    sub_range: Option<SubRange>,
    /// The warning message.
    message: EcoString,
    /// Additional hints to the user indicating how this warning could be
    /// avoided or worked around.
    hints: EcoVec<(EcoString, Option<SubRange>)>,
}

impl WarningWrapper {
    /// Wrap an existing syntax node in a warning node.
    fn new(child: Node, sub_range: Option<SubRange>, message: EcoString) -> Self {
        Self { child, sub_range, message, hints: eco_vec![] }
    }

    /// Produce the syntax diagnostic for a warning.
    fn diagnostic(&self, span: Span) -> SyntaxDiagnostic {
        SyntaxDiagnostic {
            is_error: false,
            span: DiagSpan::from_span(span, self.sub_range),
            message: self.message.clone(),
            hints: build_diagnostic_hints(span, &self.hints),
        }
    }

    /// Write with debug formatting using the node's overall span.
    ///
    /// The overall span is needed to correctly format sub-ranges. If the syntax
    /// tree used range-mapping ([`SyntaxNode::synthesize_mapped`]), we can't
    /// know the actual text being warned/hinted at without accessing the source
    /// file. So instead of showing the text included by the sub-range, we fall
    /// back to showing the mapped indices of the sub-range.
    fn debug_fmt_span(&self, f: &mut Formatter, span: Span) -> fmt::Result {
        let full_text = LazyCell::new(|| {
            let data = self.child.clone();
            let temp_node = SyntaxNode { data, span: Span::detached() };
            temp_node.full_text()
        });
        let debug_field = |field, message, sub_range: Option<SubRange>| {
            // Inner closure has `move`, so need to explicitly capture by ref.
            let full_text = &full_text;
            debug(move |f| {
                if let Some(sr) = sub_range {
                    if let SpanKind::Range { id: _, range } = span.get() {
                        let new_range = sr.to_absolute(range.start);
                        write!(f, "{field} @mapped({new_range:?}): {message:?}")
                    } else {
                        let selected = &full_text[sr.to_relative()];
                        write!(f, "{field} @text({selected:?}): {message:?}")
                    }
                } else {
                    write!(f, "{field}: {message:?}")
                }
            })
        };

        write!(f, "Warning: ")?;
        // Use `debug_set` instead of `debug_struct` so we don't have to add a
        // field name when outputting the child.
        let mut out = f.debug_set();
        out.entry(&debug_field("message", &self.message, self.sub_range));
        for (hint, sub_range) in &self.hints {
            out.entry(&debug_field("hint", hint, *sub_range));
        }
        out.entry(&debug(move |f| self.child.debug_fmt_span(f, span)));
        out.finish()
    }
}

/// Map a vector of hints with optional sub-ranges to one with optional
/// diagnostic spans derived from a parent span.
fn build_diagnostic_hints(
    parent_span: Span,
    hints: &EcoVec<(EcoString, Option<SubRange>)>,
) -> EcoVec<Spanned<EcoString, DiagSpan>> {
    hints
        .iter()
        .map(|(message, sub_range)| {
            let msg = message.clone();
            match *sub_range {
                Some(sr) => Spanned::new(msg, DiagSpan::from_span(parent_span, Some(sr))),
                None => Spanned::detached(msg),
            }
        })
        .collect()
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
    pub fn find(&self, span: Span) -> Option<Self> {
        match span.get() {
            SpanKind::Detached => None,
            SpanKind::Number { id: _, num } => self.find_number(num),
            SpanKind::Range { id: _, range } => self.find_range(range.start, range.end),
        }
    }

    /// Find the descendant whose span number matches the given number.
    ///
    /// This relies on the ordering guarantees of numbered spans:
    /// - The number of a parent is smaller than the numbers of all its children
    /// - The numbers of sibling nodes always increase from left to right
    pub(crate) fn find_number(&self, target: SpanNumber) -> Option<Self> {
        let number = self.span().number();
        if number == target.0 {
            return Some(self.clone());
        }

        // The parent of a subtree has a smaller span number than all of its
        // descendants. Therefore, we can bail out early if the target span's
        // number is smaller than our number.
        if self.node.is_inner() && number < target.0 {
            // Use `self.children()`, not `inner.children()` to preserve being
            // in a `LinkedNode`.
            let mut children = self.children().peekable();
            while let Some(child) = children.next() {
                // Every node in this child's subtree has a smaller span number than
                // the next sibling. Therefore we only need to recurse if the next
                // sibling's span number is larger than the target span's number.
                if children.peek().is_none_or(|next| next.span().number() > target.0)
                    && let Some(found) = child.find_number(target)
                {
                    return Some(found);
                }
            }
        }

        None
    }

    /// Find the descendant whose byte range matches the given range exactly.
    pub(crate) fn find_range(&self, start: usize, end: usize) -> Option<Self> {
        if start == self.offset && end == self.offset + self.len() {
            return Some(self.clone());
        }
        for child in self.children() {
            // Descend into the single child which fully covers the range.
            if child.offset <= start && end <= child.offset + child.len() {
                return child.find_range(start, end);
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
#[derive(Debug, Clone, Copy)]
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

    /// Test the debug output of a [`SyntaxNode`].
    #[test]
    fn test_debug() {
        // A standard syntax tree:
        assert_eq!(
            format!("{:#?}", crate::parse("= Head <label>")),
            "\
Markup: 14 [
    Heading: 6 [
        HeadingMarker: \"=\",
        SpaceNoNewline: \" \",
        Markup: 4 [
            Text: \"Head\",
        ],
    ],
    SpaceNoNewline: \" \",
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

    /// Test the debug output of a [`SyntaxNode`] containing errors and warnings
    /// that use sub-ranges.
    #[test]
    fn test_debug_sub_range() {
        // An example warning for text at a sub-range:
        let mut root = crate::parse("= =head");
        let heading_body = &mut root.children_mut()[0];
        heading_body.warn_at(0..3, "equal space equal!");
        heading_body.hint("try equal equal space?");
        assert_eq!(
            format!("{root:#?}"),
            "\
Markup: 7 [
    Warning: {
        message @text(\"= =\"): \"equal space equal!\",
        hint: \"try equal equal space?\",
        Heading: 7 [
            HeadingMarker: \"=\",
            SpaceNoNewline: \" \",
            Markup: 5 [
                Text: \"=head\",
            ],
        ],
    },
]"
        );

        // An example for hints at sub-ranges:
        let mut root = crate::parse("<unclosed");
        let node = &mut root.children_mut()[0];
        // Hint on the "unclosed label" error:
        node.hint_at(0..1, "greater");
        node.hint_at(3..8, "open!");
        // Adding a warning with hints around the error:
        node.warn_at(3..9, "opened?");
        node.hint_at(0..9, "full text"); // no special treatment
        assert_eq!(
            format!("{root:#?}"),
            "\
Markup: 9 [
    Warning: {
        message @text(\"closed\"): \"opened?\",
        hint @text(\"<unclosed\"): \"full text\",
        Error: {
            text: \"<unclosed\",
            message: \"unclosed label\",
            hint @text(\"<\"): \"greater\",
            hint @text(\"close\"): \"open!\",
        },
    },
]"
        );
    }

    /// Test the debug output of a [`SyntaxNode`] containing errors and warnings
    /// when synthesizing spans and range mappings.
    #[test]
    fn test_debug_synthesized_ranges() {
        // An example for hints at sub-ranges:
        let mut root = crate::parse("<unclosed");
        let node = &mut root.children_mut()[0];
        // Hint on the "unclosed label" error:
        node.hint_at(0..1, "greater");
        node.hint_at(3..8, "open!");
        // Adding a warning with hints around the error:
        node.warn_at(3..9, "opened?");
        node.hint_at(0..9, "full text");

        // Synthesize sub-ranges with a range mapping:
        let mut synth_mapped = root.clone();
        let range_mapper = RangeMapper::new([10..13, 23..29]).unwrap();
        assert_eq!(synth_mapped.len(), range_mapper.total_len());

        let id = crate::FileId::new_for_test("synth_mapped");
        synth_mapped.synthesize_mapped(id, &range_mapper).unwrap();
        assert_eq!(
            format!("{synth_mapped:#?}"),
            "\
Markup: 9 [
    Warning: {
        message @mapped(23..29): \"opened?\",
        hint @mapped(10..29): \"full text\",
        Error: {
            text: \"<unclosed\",
            message: \"unclosed label\",
            hint @mapped(10..11): \"greater\",
            hint @mapped(23..28): \"open!\",
        },
    },
]"
        );

        // Synthesize sub-ranges with a detached span. The `message` and `hint`
        // output no longer includes the related text, but we add a new hint
        // reading: "from index `a` to `b`".
        let mut synth_detached = root.clone();
        synth_detached.synthesize(Span::detached());
        assert_eq!(
            format!("{synth_detached:#?}"),
            "\
Markup: 9 [
    Warning: {
        message: \"opened?\",
        hint: \"full text\",
        hint: \"from index `3` to `9`\",
        Error: {
            text: \"<unclosed\",
            message: \"unclosed label\",
            hint: \"greater\",
            hint: \"open!\",
            hint: \"from index `0` to `9`\",
        },
    },
]"
        );

        // Synthesize an error that surrounds empty text. This has a slightly
        // different hint than when the text is not empty.
        let mut root = crate::parse("#");
        root.synthesize(Span::detached());
        assert_eq!(
            format!("{root:#?}"),
            "\
Markup: 1 [
    Hash: \"#\",
    Error: {
        text: \"\",
        message: \"expected expression\",
        hint: \"at index `1`\",
    },
]"
        );
    }

    /// Return a vector of the span numbers from an in-order traversal of the
    /// tree.
    fn span_numbers(root: &SyntaxNode) -> Vec<u64> {
        let mut vec = vec![];
        root.traverse(|node| {
            vec.push(node.span().number());
            node.children()
        });
        vec
    }

    /// Basic test for span numbering.
    #[test]
    #[expect(clippy::zero_prefixed_literal)]
    fn test_numberize_basic() {
        let id = FileId::new_for_test("numberize_basic");

        // Simplest actual example.
        let mut root = crate::parse("A");
        assert_eq!(format!("{root:?}"), "Markup: 1 [Text: \"A\"]");
        let range_numberings = [
            (2..(2 + 02), [2, 3]), // +0, +1
            (2..(2 + 07), [2, 3]),
            (2..(2 + 08), [3, 5]), // +1, +2
            (2..(2 + 11), [3, 5]),
            (2..(2 + 12), [3, 6]),  // +1, +3
            (2..(2 + 16), [4, 8]),  // +2, +4
            (2..(2 + 20), [4, 9]),  // +2, +5
            (2..(2 + 24), [5, 11]), // +3, +6
            (Span::FULL, {
                let Range { start, end } = Span::FULL;
                let desc = root.descendants() as u64;
                let stride = (end - start) / (2 * desc);
                [0, 1].map(|i| start + stride / 2 + i * stride)
            }),
        ];
        for (range, numbers) in range_numberings {
            root.numberize(id, range).unwrap();
            assert_eq!(span_numbers(&root), numbers);
        }

        // Two extra nodes.
        let mut root = crate::parse(" A ");
        assert_eq!(
            format!("{root:?}"),
            "Markup: 3 [SpaceNoNewline: \" \", Text: \"A\", SpaceNoNewline: \" \"]",
        );
        // This has a simple pattern since it has no nesting.
        let range_numberings = [
            (2..(2 + 04), [2, 3, 4, 5]), // +0, iter by 1
            (2..(2 + 05), [2, 3, 4, 5]),
            (2..(2 + 08), [2, 3, 4, 5]), // +0, iter by 1 (since 8/16==0, 8/8==1)
            (2..(2 + 15), [2, 3, 4, 5]),
            (2..(2 + 16), [3, 5, 7, 9]), // +1, iter by 2 (since 16/16==1, 16/8==2)
            (2..(2 + 23), [3, 5, 7, 9]),
            (2..(2 + 24), [3, 6, 9, 12]), // +1, iter by 3 (since 24/16==1, 24/8==3)
            (2..(2 + 32), [4, 8, 12, 16]), // +2, iter by 4 (etc.)
            (2..(2 + 40), [4, 9, 14, 19]), // +2, iter by 5
            (2..(2 + 48), [5, 11, 17, 23]), // +3, iter by 6
            (Span::FULL, {
                let Range { start, end } = Span::FULL;
                let desc = root.descendants() as u64;
                let stride = (end - start) / (2 * desc);
                [0, 1, 2, 3].map(|i| start + stride / 2 + i * stride)
            }),
        ];
        for (range, numbering) in range_numberings {
            root.numberize(id, range).unwrap();
            assert_eq!(span_numbers(&root), numbering);
        }

        // More nesting and an empty inner node.
        let mut root = crate::parse("1.");
        assert_eq!(
            format!("{root:?}"),
            "Markup: 2 [EnumItem: 2 [EnumMarker: \"1.\", Markup: 0]]",
        );
        // This pattern is harder. The first node always matches the previous
        // pattern, but the `EnumItem` node only counts 3 descendants, so starts
        // a different stride for its children.
        let range_numberings = [
            (2..(2 + 04), [2, 3, 4, 5]), // Start simple
            (2..(2 + 05), [2, 3, 4, 5]),
            (2..(2 + 08), [2, 3, 4, 5]),
            (2..(2 + 15), [2, 3, 4, 5]), // +1, +1, iter by 1 (15/8==1)
            (2..(2 + 16), [3, 4, 5, 6]), // +2, +1, iter by 1 (16/8==2)
            (2..(2 + 23), [3, 4, 5, 6]),
            (2..(2 + 24), [3, 5, 6, 7]), // +2, +2, iter by 1 (24/8==3)
            (2..(2 + 31), [3, 5, 6, 7]),
            (2..(2 + 32), [4, 7, 9, 10]), // +3, +3, iter by 1 (32/8==4)
            (2..(2 + 40), [4, 8, 10, 11]),
            (2..(2 + 47), [4, 8, 10, 11]),
            (2..(2 + 48), [5, 9, 12, 14]),
            (Span::FULL, {
                let Range { start, end } = Span::FULL;
                let desc = root.descendants() as u64;
                let stride = (end - start) / (2 * desc);
                let markup_1 = start + stride / 2;
                let enum_item = markup_1 + stride * 3 / 4; // stride/4 + stride/2
                let enum_marker = enum_item + stride / 2;
                let markup_2 = enum_marker + stride * 3 / 8; // stride/4 + stride/8
                [markup_1, enum_item, enum_marker, markup_2]
            }),
        ];
        for (range, numbering) in range_numberings {
            root.numberize(id, range).unwrap();
            assert_eq!(span_numbers(&root), numbering);
        }
    }

    impl SyntaxNode {
        /// Helper to call [`SyntaxNode::replace_children`] and
        /// [`SyntaxNode::renumber_children`] more easily.
        fn replace_and_renumber(
            &mut self,
            range: Range<usize>,
            replacement: &[SyntaxNode],
        ) -> NumberingResult {
            let replaced_range = self.replace_children(range, replacement.to_vec());
            self.renumber_children(replaced_range)
        }
    }

    /// Test span numbering when replacing a single child.
    #[test]
    #[expect(clippy::zero_prefixed_literal)]
    fn test_numberize_replace() {
        let id = FileId::new_for_test("numberize_replace");

        let orig_root = crate::parse("A");
        assert_eq!(format!("{orig_root:?}"), "Markup: 1 [Text: \"A\"]");

        // Test replacing a single child with one or two nodes.
        let one_node = &[SyntaxNode::leaf(SyntaxKind::Text, "B")];
        let two_nodes = &[
            SyntaxNode::leaf(SyntaxKind::Text, "C"),
            SyntaxNode::leaf(SyntaxKind::Text, "D"),
        ];
        let range_numbering_replacements = [
            // (1) The range of numbers, (2) the initial numbering for that
            // range, (3) the numbering when one node replaces "A", (4) the
            // numbering when two nodes replace "A".
            (2..(2 + 02), [2, 3], Some([2, 3]), None),
            (2..(2 + 03), [2, 3], Some([2, 3]), Some([2, 3, 4])),
            (2..(2 + 04), [2, 3], Some([2, 3]), Some([2, 3, 4])),
            (2..(2 + 07), [2, 3], Some([2, 4]), Some([2, 3, 4])), // init/one differ
            (2..(2 + 08), [3, 5], Some([3, 5]), Some([3, 4, 5])),
            (2..(2 + 10), [3, 5], Some([3, 6]), Some([3, 5, 7])), // init/one differ
            (2..(2 + 16), [4, 8], Some([4, 8]), Some([4, 6, 9])),
            (2..(2 + 19), [4, 8], Some([4, 9]), Some([4, 7, 11])), // init/one differ
        ];
        for (range, num_init, replace_one, replace_two) in range_numbering_replacements {
            let mut root = orig_root.clone();
            root.numberize(id, range).unwrap();
            assert_eq!(span_numbers(&root), num_init);

            // Replace with a single node.
            let num_result = root.replace_and_renumber(0..1, one_node);
            if let Some(expected) = replace_one {
                assert!(num_result.is_ok(), "numbering should succeed");
                assert_eq!(span_numbers(&root), expected);
            } else {
                assert!(num_result.is_err(), "numbering should fail");
            }

            // Replace with two nodes.
            let num_result = root.replace_and_renumber(0..1, two_nodes);
            if let Some(expected) = replace_two {
                assert!(num_result.is_ok(), "numbering should succeed");
                assert_eq!(span_numbers(&root), expected);
            } else {
                assert!(num_result.is_err(), "numbering should fail");
            }
        }
    }

    /// Test how many nodes we can insert at the front/back of an inner node
    /// until we change any of the spans.
    #[test]
    #[expect(clippy::zero_prefixed_literal)]
    fn test_numberize_insertion() {
        let id = FileId::new_for_test("numberize_insertion");
        let one_node = &[SyntaxNode::leaf(SyntaxKind::Text, "X")];

        let orig_root = crate::parse("A");
        assert_eq!(format!("{orig_root:?}"), "Markup: 1 [Text: \"A\"]");

        let back_insertion = [
            // (1) The range of numbers, (2) the initial numbering for
            // that range, (3) the numbers inserted at the _back_ until one of
            // the spans changes.
            (2..(2 + 02), [2, 3], &[] as &[u64]),
            (2..(2 + 03), [2, 3], &[4]),
            (2..(2 + 04), [2, 3], &[4, 5]),
            (2..(2 + 07), [2, 3], &[5, 6, 7, 8]),
            (2..(2 + 08), [3, 5], &[7, 8, 9]),
            (2..(2 + 09), [3, 5], &[7, 8, 9, 10]),
            (2..(2 + 10), [3, 5], &[7, 9, 10, 11]),
            (2..(2 + 12), [3, 6], &[8, 10, 11, 12, 13]),
            (2..(2 + 13), [3, 6], &[9, 11, 12, 13, 14]),
            (2..(2 + 14), [3, 6], &[9, 11, 13, 14, 15]),
            (2..(2 + 15), [3, 6], &[9, 11, 13, 14, 15, 16]),
            (2..(2 + 16), [4, 8], &[11, 13, 15, 16, 17]),
        ];
        for (range, nums_init, inserted_nums) in back_insertion {
            let mut expected = nums_init.to_vec();

            let mut back = orig_root.clone();
            back.numberize(id, range).unwrap();
            assert_eq!(span_numbers(&back), expected);

            expected.extend_from_slice(inserted_nums);

            // Insert nodes in the back one at a time until the span number of
            // one of nodes changes.
            let mut prev = back.clone();
            let mut n = 1;
            while back.replace_and_renumber(n..n, one_node).is_ok()
                && prev
                    .children()
                    .zip(back.children().take(n))
                    .all(|(a, b)| a.span() == b.span())
            {
                n += 1;
                prev = back.clone();
            }
            assert_eq!(span_numbers(&prev), expected);
        }

        let front_insertion = [
            // (1) The range of numbers, (2) the initial numbering for that
            // range, (3) the numbers inserted at the _front_ until one of the
            // spans changes.
            (2..(2 + 02), [2, 3], &[] as &[u64]),
            (2..(2 + 07), [2, 3], &[]),
            (2..(2 + 08), [3, 5], &[4]),
            (2..(2 + 09), [3, 5], &[4]),
            (2..(2 + 10), [3, 5], &[4]),
            (2..(2 + 11), [3, 5], &[4]),
            (2..(2 + 12), [3, 6], &[4]),
            (2..(2 + 16), [4, 8], &[5]),
            (2..(2 + 20), [4, 9], &[5, 6]),
            // To understand these patterns, recall that the last span number
            // will be the first node inserted (since we insert one at a time).
            // For example, we insert 7 between 5 and 11 because we split the
            // range of 5 numbers (6,7,8,9,10) in half: `5 / 2 == 2` and we
            // insert `5 + 2 == 7`. We then insert 6 because it is the only
            // number between 5 and 7, and we stop there since another insertion
            // would be forced to modify one of the existing numbers.
            (2..(2 + 24), [5, 11], &[6, 7]),
            (2..(2 + 28), [5, 12], &[6, 7]),
            (2..(2 + 32), [6, 14], &[7, 8]),
            (2..(2 + 40), [7, 17], &[8, 10]),
            (2..(2 + 48), [8, 20], &[9, 11]),
            (2..(2 + 56), [9, 23], &[10, 13]),
            (2..(2 + 80), [12, 32], &[13, 14, 17]),
            (2..(2 + 88), [13, 35], &[14, 15, 19]),
            (2..(2 + 144), [20, 56], &[21, 23, 29]),
            (2..(2 + 152), [21, 59], &[22, 24, 31]),
            (2..(2 + 160), [22, 62], &[23, 25, 32]),
        ];
        for (range, num_init, inserted_nums) in front_insertion {
            let mut expected = num_init.to_vec();

            let mut front = orig_root.clone();
            front.numberize(id, range).unwrap();
            assert_eq!(span_numbers(&front), expected);

            expected.splice(1..1, inserted_nums.iter().copied());

            // Insert nodes in the front one at a time until the span number of
            // one of nodes changes.
            let mut prev = front.clone();
            while front.replace_and_renumber(0..0, one_node).is_ok()
                && prev
                    .children()
                    .zip(front.children().skip(1))
                    .all(|(a, b)| a.span() == b.span())
            {
                prev = front.clone();
            }
            assert_eq!(span_numbers(&prev), expected);
        }
    }

    #[test]
    fn test_linked_node() {
        let test_id = FileId::new_for_test("test_linked_node");
        let source = Source::new(test_id, "#set text(12pt, red)");

        // Find "text" with Before.
        let node = LinkedNode::new(source.root()).leaf_at(7, Side::Before).unwrap();
        assert_eq!(node.offset(), 5);
        assert_eq!(node.leaf_text(), "text");

        // Find "text" with After.
        let node = LinkedNode::new(source.root()).leaf_at(7, Side::After).unwrap();
        assert_eq!(node.offset(), 5);
        assert_eq!(node.leaf_text(), "text");

        // Go back to "#set". Skips the space.
        let prev = node.prev_sibling().unwrap();
        assert_eq!(prev.offset(), 1);
        assert_eq!(prev.leaf_text(), "set");
    }

    #[test]
    fn test_linked_node_non_trivia_leaf() {
        let test_id = FileId::new_for_test("test_linked_node_non_trivia_leaf");
        let source = Source::new(test_id, "#set fun(12pt, red)");
        let leaf = LinkedNode::new(source.root()).leaf_at(6, Side::Before).unwrap();
        let prev = leaf.prev_leaf().unwrap();
        assert_eq!(leaf.leaf_text(), "fun");
        assert_eq!(prev.leaf_text(), "set");

        // Check position 9 with Before.
        let source = Source::new(test_id, "#let x = 10");
        let leaf = LinkedNode::new(source.root()).leaf_at(9, Side::Before).unwrap();
        let prev = leaf.prev_leaf().unwrap();
        let next = leaf.next_leaf().unwrap();
        assert_eq!(prev.leaf_text(), "=");
        assert_eq!(leaf.leaf_text(), " ");
        assert_eq!(next.leaf_text(), "10");

        // Check position 9 with After.
        let source = Source::new(test_id, "#let x = 10");
        let leaf = LinkedNode::new(source.root()).leaf_at(9, Side::After).unwrap();
        let prev = leaf.prev_leaf().unwrap();
        assert!(leaf.next_leaf().is_none());
        assert_eq!(prev.leaf_text(), "=");
        assert_eq!(leaf.leaf_text(), "10");
    }
}
