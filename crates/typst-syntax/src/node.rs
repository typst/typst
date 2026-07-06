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
    pub fn warn(&mut self, message: impl Into<EcoString>) {
        let kind = self.kind();
        let child = std::mem::replace(&mut self.data, Node::Leaf(EcoString::new(), kind));
        let warn = Arc::new(WarningWrapper::new(child, None, message.into()));
        self.data = Node::Warning(warn, kind);
    }

    /// Add a warning around this node at a particular sub-range of the node's
    /// text. Panics if the range is empty or exceeds the length of the wrapped
    /// text.
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
                    update_sub_range(offset, &mut warn.sub_range);
                    for (_hint, sub_range) in warn.hints.make_mut() {
                        update_sub_range(offset, sub_range);
                    }
                    if let Some(hint) = add_hint(offset, len) {
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
                        if a.message == b.message && a.hints == b.hints =>
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

    /// Replaces a range of children with a replacement.
    ///
    /// May have mutated the children if it returns `Err(_)`.
    pub(super) fn replace_children(
        &mut self,
        range: Range<usize>,
        replacement: Vec<SyntaxNode>,
    ) -> NumberingResult {
        if let Some((inner, span)) = self.inner_and_span_mut() {
            inner.replace_children(span, range, replacement)
        } else {
            Ok(())
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
        if let Some((inner, _)) = self.inner_and_span_mut() {
            inner.update_parent(prev_len, new_len, prev_descendants, new_descendants);
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
        self.data.fmt(f)
    }
}

impl Debug for Node {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Node::Leaf(text, kind) => write!(f, "{kind:?}: {text:?}"),
            Node::Inner(inner, kind) => inner.debug_fmt(f, *kind),
            Node::Error(err, _) => err.fmt(f),
            Node::Warning(warn, _) => warn.fmt(f),
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
            stride = space / self.descendants as u64;
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

    /// Replaces a range of children with a replacement.
    ///
    /// May have mutated the children if it returns `Err(_)`.
    fn replace_children(
        &mut self,
        span: &mut Span,
        mut range: Range<usize>,
        replacement: Vec<SyntaxNode>,
    ) -> NumberingResult {
        let Some(id) = span.id() else { return Err(Unnumberable) };
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

        // Update our diagnosis after the replacement.
        // - If we had no errors/warnings before, we can just use the replaced
        //   diagnosis
        // - Or, if our replacement has errors _and_ warnings, we can use that
        // - Otherwise, we need to update based on all of the children _outside_
        //   the replaced range in case we replaced the erroneous children
        let replaced_diagnosis = Diagnosis::any(replacement);
        if !self.diagnosis.either() || replaced_diagnosis.both() {
            self.diagnosis = replaced_diagnosis;
        } else {
            self.diagnosis = replaced_diagnosis.or(Diagnosis::or(
                Diagnosis::any(&self.children[..range.start]),
                Diagnosis::any(&self.children[range.end..]),
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
        self.diagnosis = Diagnosis::any(&self.children);
    }

    /// Format the inner node with its `SyntaxKind` for debugging.
    fn debug_fmt(&self, f: &mut Formatter, kind: SyntaxKind) -> fmt::Result {
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
}

impl Debug for ErrorNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if self.text.is_empty() && self.hints.is_empty() {
            write!(f, "Error: {:?}", self.message)
        } else {
            let mut out = f.debug_struct("Error:");
            out.field("text", &self.text);
            out.field("message", &self.message);
            for (hint, sub_range) in &self.hints {
                let field = if let Some(sub_range) = sub_range {
                    let selected = &self.text[sub_range.to_relative()];
                    &format!("hint @({selected:?})")
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
}

impl Debug for WarningWrapper {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
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
                    let selected = &full_text[sr.to_relative()];
                    write!(f, "{field} @({selected:?}): {message:?}")
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
        out.entry(&self.child);
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
        message @(\"= =\"): \"equal space equal!\",
        hint: \"try equal equal space?\",
        Heading: 7 [
            HeadingMarker: \"=\",
            Space: \" \",
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
        message @(\"closed\"): \"opened?\",
        hint @(\"<unclosed\"): \"full text\",
        Error: {
            text: \"<unclosed\",
            message: \"unclosed label\",
            hint @(\"<\"): \"greater\",
            hint @(\"close\"): \"open!\",
        },
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
        let source = Source::detached("#set fun(12pt, red)");
        let leaf = LinkedNode::new(source.root()).leaf_at(6, Side::Before).unwrap();
        let prev = leaf.prev_leaf().unwrap();
        assert_eq!(leaf.leaf_text(), "fun");
        assert_eq!(prev.leaf_text(), "set");

        // Check position 9 with Before.
        let source = Source::detached("#let x = 10");
        let leaf = LinkedNode::new(source.root()).leaf_at(9, Side::Before).unwrap();
        let prev = leaf.prev_leaf().unwrap();
        let next = leaf.next_leaf().unwrap();
        assert_eq!(prev.leaf_text(), "=");
        assert_eq!(leaf.leaf_text(), " ");
        assert_eq!(next.leaf_text(), "10");

        // Check position 9 with After.
        let source = Source::detached("#let x = 10");
        let leaf = LinkedNode::new(source.root()).leaf_at(9, Side::After).unwrap();
        let prev = leaf.prev_leaf().unwrap();
        assert!(leaf.next_leaf().is_none());
        assert_eq!(prev.leaf_text(), "=");
        assert_eq!(leaf.leaf_text(), "10");
    }
}
