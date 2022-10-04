//! Syntax types.

pub mod ast;
mod highlight;
mod span;

use std::fmt::{self, Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::Range;
use std::sync::Arc;

pub use highlight::*;
pub use span::*;

use self::ast::{RawNode, TypedNode, Unit};
use crate::diag::SourceError;
use crate::source::SourceId;
use crate::util::EcoString;

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
            &NodeKind::Error(pos, ref message) => {
                vec![SourceError::new(self.span().with_pos(pos), message)]
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

/// All syntactical building blocks that can be part of a Typst document.
///
/// Can be emitted as a token by the tokenizer or as part of a syntax node by
/// the parser.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    /// A line comment, two slashes followed by inner contents, terminated with
    /// a newline: `//<str>\n`.
    LineComment,
    /// A block comment, a slash and a star followed by inner contents,
    /// terminated with a star and a slash: `/*<str>*/`.
    ///
    /// The comment can contain nested block comments.
    BlockComment,
    /// One or more whitespace characters. Single spaces are collapsed into text
    /// nodes if they would otherwise be surrounded by text nodes.
    ///
    /// Also stores how many newlines are contained.
    Space { newlines: usize },

    /// A left curly brace, starting a code block: `{`.
    LeftBrace,
    /// A right curly brace, terminating a code block: `}`.
    RightBrace,
    /// A left square bracket, starting a content block: `[`.
    LeftBracket,
    /// A right square bracket, terminating a content block: `]`.
    RightBracket,
    /// A left round parenthesis, starting a grouped expression, collection,
    /// argument or parameter list: `(`.
    LeftParen,
    /// A right round parenthesis, terminating a grouped expression, collection,
    /// argument or parameter list: `)`.
    RightParen,
    /// A comma separator in a sequence: `,`.
    Comma,
    /// A semicolon terminating an expression: `;`.
    Semicolon,
    /// A colon between name / key and value in a dictionary, argument or
    /// parameter list, or between the term and body of a description list
    /// term: `:`.
    Colon,
    /// The strong text toggle, multiplication operator, and wildcard import
    /// symbol: `*`.
    Star,
    /// Toggles emphasized text and indicates a subscript in a formula: `_`.
    Underscore,
    /// Starts and ends a math formula.
    Dollar,
    /// The non-breaking space: `~`.
    Tilde,
    /// The soft hyphen: `-?`.
    HyphQuest,
    /// The en-dash: `--`.
    Hyph2,
    /// The em-dash: `---`.
    Hyph3,
    /// The ellipsis: `...`.
    Dot3,
    /// A smart quote: `'` or `"`.
    Quote { double: bool },
    /// The unary plus and addition operator, and start of enum items: `+`.
    Plus,
    /// The unary negation and subtraction operator, and start of list
    /// items: `-`.
    Minus,
    /// The division operator, start of description list items, and fraction
    /// operator in a formula: `/`.
    Slash,
    /// The superscript operator: `^`.
    Hat,
    /// The math alignment operator: `&`.
    Amp,
    /// The field access and method call operator: `.`.
    Dot,
    /// The assignment operator: `=`.
    Eq,
    /// The equality operator: `==`.
    EqEq,
    /// The inequality operator: `!=`.
    ExclEq,
    /// The less-than operator: `<`.
    Lt,
    /// The less-than or equal operator: `<=`.
    LtEq,
    /// The greater-than operator: `>`.
    Gt,
    /// The greater-than or equal operator: `>=`.
    GtEq,
    /// The add-assign operator: `+=`.
    PlusEq,
    /// The subtract-assign operator: `-=`.
    HyphEq,
    /// The multiply-assign operator: `*=`.
    StarEq,
    /// The divide-assign operator: `/=`.
    SlashEq,
    /// The spread operator: `..`.
    Dots,
    /// An arrow between a closure's parameters and body: `=>`.
    Arrow,

    /// The `not` operator.
    Not,
    /// The `and` operator.
    And,
    /// The `or` operator.
    Or,
    /// The `none` literal.
    None,
    /// The `auto` literal.
    Auto,
    /// The `let` keyword.
    Let,
    /// The `set` keyword.
    Set,
    /// The `show` keyword.
    Show,
    /// The `wrap` keyword.
    Wrap,
    /// The `if` keyword.
    If,
    /// The `else` keyword.
    Else,
    /// The `for` keyword.
    For,
    /// The `in` keyword.
    In,
    /// The `while` keyword.
    While,
    /// The `break` keyword.
    Break,
    /// The `continue` keyword.
    Continue,
    /// The `return` keyword.
    Return,
    /// The `import` keyword.
    Import,
    /// The `include` keyword.
    Include,
    /// The `from` keyword.
    From,
    /// The `as` keyword.
    As,

    /// Markup of which all lines must have a minimal indentation.
    ///
    /// Notably, the number does not determine in which column the markup
    /// started, but to the right of which column all markup elements must be,
    /// so it is zero except for headings and lists.
    Markup { min_indent: usize },
    /// A forced line break in markup or math.
    Linebreak,
    /// Consecutive text without markup. While basic text with just single
    /// spaces is collapsed into a single node, certain symbols that could
    /// possibly be markup force text into multiple nodes.
    Text(EcoString),
    /// A slash and the letter "u" followed by a hexadecimal unicode entity
    /// enclosed in curly braces: `\u{1F5FA}`.
    Escape(char),
    /// Strong content: `*Strong*`.
    Strong,
    /// Emphasized content: `_Emphasized_`.
    Emph,
    /// A hyperlink.
    Link(EcoString),
    /// A raw block with optional syntax highlighting: `` `...` ``.
    Raw(Arc<RawNode>),
    /// A math formula: `$x$`, `$ x^2 $`.
    Math,
    /// A section heading: `= Introduction`.
    Heading,
    /// An item in an unordered list: `- ...`.
    List,
    /// An item in an enumeration (ordered list): `+ ...` or `1. ...`.
    Enum,
    /// An explicit enumeration numbering: `23.`.
    EnumNumbering(usize),
    /// An item in a description list: `/ Term: Details.
    Desc,
    /// A label: `<label>`.
    Label(EcoString),
    /// A reference: `@label`.
    Ref(EcoString),

    /// An atom in a math formula: `x`, `+`, `12`.
    Atom(EcoString),
    /// A base with an optional sub- and superscript in a formula: `a_1^2`.
    Script,
    /// A fraction: `x/2`.
    Frac,
    /// A math alignment indicator: `&`, `&&`.
    Align,

    /// An identifier: `center`.
    Ident(EcoString),
    /// A boolean: `true`, `false`.
    Bool(bool),
    /// An integer: `120`.
    Int(i64),
    /// A floating-point number: `1.2`, `10e-4`.
    Float(f64),
    /// A numeric value with a unit: `12pt`, `3cm`, `2em`, `90deg`, `50%`.
    Numeric(f64, Unit),
    /// A quoted string: `"..."`.
    Str(EcoString),
    /// A code block: `{ let x = 1; x + 2 }`.
    CodeBlock,
    /// A content block: `[*Hi* there!]`.
    ContentBlock,
    /// A grouped expression: `(1 + 2)`.
    GroupExpr,
    /// An array expression: `(1, "hi", 12cm)`.
    ArrayExpr,
    /// A dictionary expression: `(thickness: 3pt, pattern: dashed)`.
    DictExpr,
    /// A named pair: `thickness: 3pt`.
    Named,
    /// A keyed pair: `"spacy key": true`.
    Keyed,
    /// A unary operation: `-x`.
    UnaryExpr,
    /// A binary operation: `a + b`.
    BinaryExpr,
    /// A field access: `properties.age`.
    FieldAccess,
    /// An invocation of a function: `f(x, y)`.
    FuncCall,
    /// An invocation of a method: `array.push(v)`.
    MethodCall,
    /// A function call's argument list: `(x, y)`.
    CallArgs,
    /// Spreaded arguments or a argument sink: `..x`.
    Spread,
    /// A closure expression: `(x, y) => z`.
    ClosureExpr,
    /// A closure's parameters: `(x, y)`.
    ClosureParams,
    /// A let expression: `let x = 1`.
    LetExpr,
    /// A set expression: `set text(...)`.
    SetExpr,
    /// A show expression: `show node: heading as [*{nody.body}*]`.
    ShowExpr,
    /// A wrap expression: `wrap body in columns(2, body)`.
    WrapExpr,
    /// An if-else expression: `if x { y } else { z }`.
    IfExpr,
    /// A while loop expression: `while x { ... }`.
    WhileExpr,
    /// A for loop expression: `for x in y { ... }`.
    ForExpr,
    /// A for loop's destructuring pattern: `x` or `x, y`.
    ForPattern,
    /// An import expression: `import a, b, c from "utils.typ"`.
    ImportExpr,
    /// Items to import: `a, b, c`.
    ImportItems,
    /// An include expression: `include "chapter1.typ"`.
    IncludeExpr,
    /// A break expression: `break`.
    BreakExpr,
    /// A continue expression: `continue`.
    ContinueExpr,
    /// A return expression: `return x + 1`.
    ReturnExpr,

    /// Tokens that appear in the wrong place.
    Error(SpanPos, EcoString),
    /// Unknown character sequences.
    Unknown(EcoString),
}

impl NodeKind {
    /// Whether this is a kind of parenthesis.
    pub fn is_paren(&self) -> bool {
        matches!(self, Self::LeftParen | Self::RightParen)
    }

    /// Whether this is a space.
    pub fn is_space(&self) -> bool {
        matches!(self, Self::Space { .. })
    }

    /// Whether this is trivia.
    pub fn is_trivia(&self) -> bool {
        self.is_space() || matches!(self, Self::LineComment | Self::BlockComment)
    }

    /// Whether this is a kind of error.
    pub fn is_error(&self) -> bool {
        matches!(self, NodeKind::Error(_, _) | NodeKind::Unknown(_))
    }

    /// Whether `at_start` would still be true after this node given the
    /// previous value of the property.
    pub fn is_at_start(&self, prev: bool) -> bool {
        match self {
            Self::Space { newlines: (1 ..) } => true,
            Self::Space { .. } | Self::LineComment | Self::BlockComment => prev,
            _ => false,
        }
    }

    /// Whether changes _inside_ this node are safely encapsulated, so that only
    /// this node must be reparsed.
    pub fn is_bounded(&self) -> bool {
        match self {
            Self::CodeBlock
            | Self::ContentBlock
            | Self::Linebreak { .. }
            | Self::Tilde
            | Self::HyphQuest
            | Self::Hyph2
            | Self::Hyph3
            | Self::Dot3
            | Self::Quote { .. }
            | Self::BlockComment
            | Self::Space { .. }
            | Self::Escape(_) => true,
            _ => false,
        }
    }

    /// A human-readable name for the kind.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LineComment => "line comment",
            Self::BlockComment => "block comment",
            Self::Space { .. } => "space",

            Self::LeftBrace => "opening brace",
            Self::RightBrace => "closing brace",
            Self::LeftBracket => "opening bracket",
            Self::RightBracket => "closing bracket",
            Self::LeftParen => "opening paren",
            Self::RightParen => "closing paren",
            Self::Comma => "comma",
            Self::Semicolon => "semicolon",
            Self::Colon => "colon",
            Self::Star => "star",
            Self::Underscore => "underscore",
            Self::Dollar => "dollar sign",
            Self::Tilde => "non-breaking space",
            Self::HyphQuest => "soft hyphen",
            Self::Hyph2 => "en dash",
            Self::Hyph3 => "em dash",
            Self::Dot3 => "ellipsis",
            Self::Quote { double: false } => "single quote",
            Self::Quote { double: true } => "double quote",
            Self::Plus => "plus",
            Self::Minus => "minus",
            Self::Slash => "slash",
            Self::Hat => "hat",
            Self::Amp => "ampersand",
            Self::Dot => "dot",
            Self::Eq => "assignment operator",
            Self::EqEq => "equality operator",
            Self::ExclEq => "inequality operator",
            Self::Lt => "less-than operator",
            Self::LtEq => "less-than or equal operator",
            Self::Gt => "greater-than operator",
            Self::GtEq => "greater-than or equal operator",
            Self::PlusEq => "add-assign operator",
            Self::HyphEq => "subtract-assign operator",
            Self::StarEq => "multiply-assign operator",
            Self::SlashEq => "divide-assign operator",
            Self::Dots => "dots",
            Self::Arrow => "arrow",

            Self::Not => "operator `not`",
            Self::And => "operator `and`",
            Self::Or => "operator `or`",
            Self::None => "`none`",
            Self::Auto => "`auto`",
            Self::Let => "keyword `let`",
            Self::Set => "keyword `set`",
            Self::Show => "keyword `show`",
            Self::Wrap => "keyword `wrap`",
            Self::If => "keyword `if`",
            Self::Else => "keyword `else`",
            Self::For => "keyword `for`",
            Self::In => "keyword `in`",
            Self::While => "keyword `while`",
            Self::Break => "keyword `break`",
            Self::Continue => "keyword `continue`",
            Self::Return => "keyword `return`",
            Self::Import => "keyword `import`",
            Self::Include => "keyword `include`",
            Self::From => "keyword `from`",
            Self::As => "keyword `as`",

            Self::Markup { .. } => "markup",
            Self::Linebreak => "linebreak",
            Self::Text(_) => "text",
            Self::Escape(_) => "escape sequence",
            Self::Strong => "strong content",
            Self::Emph => "emphasized content",
            Self::Link(_) => "link",
            Self::Raw(_) => "raw block",
            Self::Math => "math formula",
            Self::Heading => "heading",
            Self::List => "list item",
            Self::Enum => "enumeration item",
            Self::EnumNumbering(_) => "enumeration item numbering",
            Self::Desc => "description list item",
            Self::Label(_) => "label",
            Self::Ref(_) => "reference",

            Self::Atom(_) => "math atom",
            Self::Script => "script",
            Self::Frac => "fraction",
            Self::Align => "alignment indicator",

            Self::Ident(_) => "identifier",
            Self::Bool(_) => "boolean",
            Self::Int(_) => "integer",
            Self::Float(_) => "float",
            Self::Numeric(_, _) => "numeric value",
            Self::Str(_) => "string",
            Self::CodeBlock => "code block",
            Self::ContentBlock => "content block",
            Self::GroupExpr => "group",
            Self::ArrayExpr => "array",
            Self::DictExpr => "dictionary",
            Self::Named => "named pair",
            Self::Keyed => "keyed pair",
            Self::UnaryExpr => "unary expression",
            Self::BinaryExpr => "binary expression",
            Self::FieldAccess => "field access",
            Self::FuncCall => "function call",
            Self::MethodCall => "method call",
            Self::CallArgs => "call arguments",
            Self::Spread => "spread",
            Self::ClosureExpr => "closure",
            Self::ClosureParams => "closure parameters",
            Self::LetExpr => "`let` expression",
            Self::SetExpr => "`set` expression",
            Self::ShowExpr => "`show` expression",
            Self::WrapExpr => "`wrap` expression",
            Self::IfExpr => "`if` expression",
            Self::WhileExpr => "while-loop expression",
            Self::ForExpr => "for-loop expression",
            Self::ForPattern => "for-loop destructuring pattern",
            Self::ImportExpr => "`import` expression",
            Self::ImportItems => "import items",
            Self::IncludeExpr => "`include` expression",
            Self::BreakExpr => "`break` expression",
            Self::ContinueExpr => "`continue` expression",
            Self::ReturnExpr => "`return` expression",

            Self::Error(_, _) => "parse error",
            Self::Unknown(text) => match text.as_str() {
                "*/" => "end of block comment",
                _ => "invalid token",
            },
        }
    }
}

impl Display for NodeKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.pad(self.as_str())
    }
}

impl Hash for NodeKind {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Self::LineComment => {}
            Self::BlockComment => {}
            Self::Space { newlines } => newlines.hash(state),

            Self::LeftBrace => {}
            Self::RightBrace => {}
            Self::LeftBracket => {}
            Self::RightBracket => {}
            Self::LeftParen => {}
            Self::RightParen => {}
            Self::Comma => {}
            Self::Semicolon => {}
            Self::Colon => {}
            Self::Star => {}
            Self::Underscore => {}
            Self::Dollar => {}
            Self::Tilde => {}
            Self::HyphQuest => {}
            Self::Hyph2 => {}
            Self::Hyph3 => {}
            Self::Dot3 => {}
            Self::Quote { double } => double.hash(state),
            Self::Plus => {}
            Self::Minus => {}
            Self::Slash => {}
            Self::Hat => {}
            Self::Amp => {}
            Self::Dot => {}
            Self::Eq => {}
            Self::EqEq => {}
            Self::ExclEq => {}
            Self::Lt => {}
            Self::LtEq => {}
            Self::Gt => {}
            Self::GtEq => {}
            Self::PlusEq => {}
            Self::HyphEq => {}
            Self::StarEq => {}
            Self::SlashEq => {}
            Self::Dots => {}
            Self::Arrow => {}

            Self::Not => {}
            Self::And => {}
            Self::Or => {}
            Self::None => {}
            Self::Auto => {}
            Self::Let => {}
            Self::Set => {}
            Self::Show => {}
            Self::Wrap => {}
            Self::If => {}
            Self::Else => {}
            Self::For => {}
            Self::In => {}
            Self::While => {}
            Self::Break => {}
            Self::Continue => {}
            Self::Return => {}
            Self::Import => {}
            Self::Include => {}
            Self::From => {}
            Self::As => {}

            Self::Markup { min_indent } => min_indent.hash(state),
            Self::Linebreak => {}
            Self::Text(s) => s.hash(state),
            Self::Escape(c) => c.hash(state),
            Self::Strong => {}
            Self::Emph => {}
            Self::Link(link) => link.hash(state),
            Self::Raw(raw) => raw.hash(state),
            Self::Math => {}
            Self::Heading => {}
            Self::List => {}
            Self::Enum => {}
            Self::EnumNumbering(num) => num.hash(state),
            Self::Desc => {}
            Self::Label(c) => c.hash(state),
            Self::Ref(c) => c.hash(state),

            Self::Atom(c) => c.hash(state),
            Self::Script => {}
            Self::Frac => {}
            Self::Align => {}

            Self::Ident(v) => v.hash(state),
            Self::Bool(v) => v.hash(state),
            Self::Int(v) => v.hash(state),
            Self::Float(v) => v.to_bits().hash(state),
            Self::Numeric(v, u) => (v.to_bits(), u).hash(state),
            Self::Str(v) => v.hash(state),
            Self::CodeBlock => {}
            Self::ContentBlock => {}
            Self::GroupExpr => {}
            Self::ArrayExpr => {}
            Self::DictExpr => {}
            Self::Named => {}
            Self::Keyed => {}
            Self::UnaryExpr => {}
            Self::BinaryExpr => {}
            Self::FieldAccess => {}
            Self::FuncCall => {}
            Self::MethodCall => {}
            Self::CallArgs => {}
            Self::Spread => {}
            Self::ClosureExpr => {}
            Self::ClosureParams => {}
            Self::LetExpr => {}
            Self::SetExpr => {}
            Self::ShowExpr => {}
            Self::WrapExpr => {}
            Self::IfExpr => {}
            Self::WhileExpr => {}
            Self::ForExpr => {}
            Self::ForPattern => {}
            Self::ImportExpr => {}
            Self::ImportItems => {}
            Self::IncludeExpr => {}
            Self::BreakExpr => {}
            Self::ContinueExpr => {}
            Self::ReturnExpr => {}

            Self::Error(pos, msg) => (pos, msg).hash(state),
            Self::Unknown(text) => text.hash(state),
        }
    }
}
