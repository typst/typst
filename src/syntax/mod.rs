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

use self::ast::{MathNode, RawNode, TypedNode, Unit};
use crate::diag::{Error, ErrorPos};
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
    /// Returns the metadata of the node.
    pub fn data(&self) -> &NodeData {
        match self {
            SyntaxNode::Inner(inner) => &inner.data,
            SyntaxNode::Leaf(leaf) => leaf,
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
    pub fn count(&self) -> usize {
        match self {
            SyntaxNode::Inner(inner) => inner.count(),
            SyntaxNode::Leaf(_) => 1,
        }
    }

    /// The span of the node.
    pub fn span(&self) -> Span {
        self.data().span()
    }

    /// If the span points into this node, convert it to a byte range.
    pub fn range(&self, span: Span, offset: usize) -> Option<Range<usize>> {
        match self {
            SyntaxNode::Inner(inner) => inner.range(span, offset),
            SyntaxNode::Leaf(leaf) => leaf.range(span, offset),
        }
    }

    /// The node's children.
    pub fn children(&self) -> std::slice::Iter<'_, SyntaxNode> {
        match self {
            SyntaxNode::Inner(inner) => inner.children(),
            SyntaxNode::Leaf(_) => [].iter(),
        }
    }

    /// Returns all leaf descendants of this node (may include itself).
    ///
    /// This method is slow and only intended for testing.
    pub fn leafs(&self) -> Vec<Self> {
        if match self {
            SyntaxNode::Inner(inner) => inner.children().len() == 0,
            SyntaxNode::Leaf(_) => true,
        } {
            vec![self.clone()]
        } else {
            self.children().flat_map(Self::leafs).collect()
        }
    }

    /// Whether the node or its children contain an error.
    pub fn erroneous(&self) -> bool {
        match self {
            Self::Inner(node) => node.erroneous,
            Self::Leaf(data) => data.kind.is_error(),
        }
    }

    /// The error messages for this node and its descendants.
    pub fn errors(&self) -> Vec<Error> {
        if !self.erroneous() {
            return vec![];
        }

        match self.kind() {
            &NodeKind::Error(pos, ref message) => {
                vec![Error { pos, ..Error::new(self.span(), message) }]
            }
            _ => self
                .children()
                .filter(|node| node.erroneous())
                .flat_map(|node| node.errors())
                .collect(),
        }
    }

    /// Convert the node to a typed AST node.
    pub fn cast<T>(&self) -> Option<T>
    where
        T: TypedNode,
    {
        T::from_untyped(self)
    }

    /// Get the first child that can cast to some AST type.
    pub fn cast_first_child<T: TypedNode>(&self) -> Option<T> {
        self.children().find_map(Self::cast)
    }

    /// Get the last child that can cast to some AST type.
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

    /// Assign spans to each node.
    pub fn number(&mut self, id: SourceId, from: u64) {
        match self {
            SyntaxNode::Inner(inner) => Arc::make_mut(inner).number(id, from),
            SyntaxNode::Leaf(leaf) => leaf.number(id, from),
        }
    }

    /// Set a synthetic node id for the node and all its descendants.
    pub fn synthesize(&mut self, span: Span) {
        match self {
            SyntaxNode::Inner(inner) => Arc::make_mut(inner).synthesize(span),
            SyntaxNode::Leaf(leaf) => leaf.synthesize(span),
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
#[derive(Clone, PartialEq, Hash)]
pub struct InnerNode {
    /// Node metadata.
    data: NodeData,
    /// The number of nodes in the whole subtree, including this node.
    count: usize,
    /// Whether this node or any of its children are erroneous.
    erroneous: bool,
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
        let mut count = 1;
        let mut erroneous = kind.is_error();

        for child in &children {
            len += child.len();
            count += child.count();
            erroneous |= child.erroneous();
        }

        Self {
            data: NodeData::new(kind, len),
            count,
            erroneous,
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
    pub fn count(&self) -> usize {
        self.count
    }

    /// If the span points into this node, convert it to a byte range.
    pub fn range(&self, span: Span, mut offset: usize) -> Option<Range<usize>> {
        if let Some(range) = self.data.range(span, offset) {
            return Some(range);
        }

        for child in &self.children {
            if let Some(range) = child.range(span, offset) {
                return Some(range);
            }

            offset += child.len();
        }

        None
    }

    /// The node's children.
    pub fn children(&self) -> std::slice::Iter<'_, SyntaxNode> {
        self.children.iter()
    }

    /// Assign spans to each node.
    pub fn number(&mut self, id: SourceId, mut from: u64) {
        self.data.number(id, from);
        from += 1;

        for child in &mut self.children {
            child.number(id, from);
            from += child.count() as u64;
        }
    }

    /// Set a synthetic node id for the node and all its descendants.
    pub fn synthesize(&mut self, span: Span) {
        self.data.synthesize(span);
        for child in &mut self.children {
            child.synthesize(span);
        }
    }

    /// The node's children, mutably.
    pub(crate) fn children_mut(&mut self) -> &mut [SyntaxNode] {
        &mut self.children
    }

    /// Replaces a range of children with some replacement.
    pub(crate) fn replace_children(
        &mut self,
        range: Range<usize>,
        replacement: Vec<SyntaxNode>,
    ) {
        let superseded = &self.children[range.clone()];

        // Compute the new byte length.
        self.data.len = self.data.len
            + replacement.iter().map(SyntaxNode::len).sum::<usize>()
            - superseded.iter().map(SyntaxNode::len).sum::<usize>();

        // Compute the new descendant count.
        self.count = self.count
            + replacement.iter().map(SyntaxNode::count).sum::<usize>()
            - superseded.iter().map(SyntaxNode::count).sum::<usize>();

        // Determine whether we're still erroneous after the replacement. That's
        // the case if
        // - any of the new nodes is erroneous,
        // - or if we were erroneous before due to a non-superseded node.
        self.erroneous = replacement.iter().any(SyntaxNode::erroneous)
            || (self.erroneous
                && (self.children[.. range.start].iter().any(SyntaxNode::erroneous)
                    || self.children[range.end ..].iter().any(SyntaxNode::erroneous)));

        // Perform the replacement.
        self.children.splice(range, replacement);
    }

    /// Update the length of this node given the old and new length of
    /// replaced children.
    pub(crate) fn update_parent(
        &mut self,
        prev_len: usize,
        new_len: usize,
        prev_count: usize,
        new_count: usize,
    ) {
        self.data.len = self.data.len + new_len - prev_len;
        self.count = self.count + new_count - prev_count;
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

    /// If the span points into this node, convert it to a byte range.
    pub fn range(&self, span: Span, offset: usize) -> Option<Range<usize>> {
        (self.span == span).then(|| offset .. offset + self.len())
    }

    /// Assign spans to each node.
    pub fn number(&mut self, id: SourceId, from: u64) {
        self.span = Span::new(id, from);
    }

    /// Set a synthetic span for the node.
    pub fn synthesize(&mut self, span: Span) {
        self.span = span;
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
    /// A left curly brace: `{`.
    LeftBrace,
    /// A right curly brace: `}`.
    RightBrace,
    /// A left square bracket: `[`.
    LeftBracket,
    /// A right square bracket: `]`.
    RightBracket,
    /// A left round parenthesis: `(`.
    LeftParen,
    /// A right round parenthesis: `)`.
    RightParen,
    /// An asterisk: `*`.
    Star,
    /// An underscore: `_`.
    Underscore,
    /// A comma: `,`.
    Comma,
    /// A semicolon: `;`.
    Semicolon,
    /// A colon: `:`.
    Colon,
    /// A plus: `+`.
    Plus,
    /// A hyphen: `-`.
    Minus,
    /// A slash: `/`.
    Slash,
    /// A dot: `.`.
    Dot,
    /// A single equals sign: `=`.
    Eq,
    /// Two equals signs: `==`.
    EqEq,
    /// An exclamation mark followed by an equals sign: `!=`.
    ExclEq,
    /// A less-than sign: `<`.
    Lt,
    /// A less-than sign followed by an equals sign: `<=`.
    LtEq,
    /// A greater-than sign: `>`.
    Gt,
    /// A greater-than sign followed by an equals sign: `>=`.
    GtEq,
    /// A plus followed by an equals sign: `+=`.
    PlusEq,
    /// A hyphen followed by an equals sign: `-=`.
    HyphEq,
    /// An asterisk followed by an equals sign: `*=`.
    StarEq,
    /// A slash followed by an equals sign: `/=`.
    SlashEq,
    /// The `not` operator.
    Not,
    /// The `and` operator.
    And,
    /// The `or` operator.
    Or,
    /// Two dots: `..`.
    Dots,
    /// An equals sign followed by a greater-than sign: `=>`.
    Arrow,
    /// The none literal: `none`.
    None,
    /// The auto literal: `auto`.
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
    /// Markup of which all lines must start in some column.
    ///
    /// Notably, the number does not determine in which column the markup
    /// started, but to the right of which column all markup elements must be,
    /// so it is zero except for headings and lists.
    Markup(usize),
    /// One or more whitespace characters.
    Space(usize),
    /// A consecutive non-markup string.
    Text(EcoString),
    /// A forced line break: `\` or `\+` if justified.
    Linebreak { justified: bool },
    /// A non-breaking space: `~`.
    NonBreakingSpace,
    /// A soft hyphen: `-?`.
    Shy,
    /// An en-dash: `--`.
    EnDash,
    /// An em-dash: `---`.
    EmDash,
    /// An ellipsis: `...`.
    Ellipsis,
    /// A smart quote: `'` or `"`.
    Quote { double: bool },
    /// A slash and the letter "u" followed by a hexadecimal unicode entity
    /// enclosed in curly braces: `\u{1F5FA}`.
    Escape(char),
    /// Strong content: `*Strong*`.
    Strong,
    /// Emphasized content: `_Emphasized_`.
    Emph,
    /// An arbitrary number of backticks followed by inner contents, terminated
    /// with the same number of backticks: `` `...` ``.
    Raw(Arc<RawNode>),
    /// Dollar signs surrounding inner contents.
    Math(Arc<MathNode>),
    /// A section heading: `= Introduction`.
    Heading,
    /// An item in an unordered list: `- ...`.
    List,
    /// An item in an enumeration (ordered list): `1. ...`.
    Enum,
    /// A numbering: `23.`.
    ///
    /// Can also exist without the number: `.`.
    EnumNumbering(Option<usize>),
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
    /// A keyed pair: `"spaced key": true`.
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
    /// A line comment, two slashes followed by inner contents, terminated with
    /// a newline: `//<str>\n`.
    LineComment,
    /// A block comment, a slash and a star followed by inner contents,
    /// terminated with a star and a slash: `/*<str>*/`.
    ///
    /// The comment can contain nested block comments.
    BlockComment,
    /// Tokens that appear in the wrong place.
    Error(ErrorPos, EcoString),
    /// Unknown character sequences.
    Unknown(EcoString),
}

impl NodeKind {
    /// Whether this is some kind of brace.
    pub fn is_brace(&self) -> bool {
        matches!(self, Self::LeftBrace | Self::RightBrace)
    }

    /// Whether this is some kind of bracket.
    pub fn is_bracket(&self) -> bool {
        matches!(self, Self::LeftBracket | Self::RightBracket)
    }

    /// Whether this is some kind of parenthesis.
    pub fn is_paren(&self) -> bool {
        matches!(self, Self::LeftParen | Self::RightParen)
    }

    /// Whether this is a space.
    pub fn is_space(&self) -> bool {
        matches!(self, Self::Space(_))
    }

    /// Whether this is trivia.
    pub fn is_trivia(&self) -> bool {
        self.is_space() || matches!(self, Self::LineComment | Self::BlockComment)
    }

    /// Whether this is some kind of error.
    pub fn is_error(&self) -> bool {
        matches!(self, NodeKind::Error(_, _) | NodeKind::Unknown(_))
    }

    /// Whether this node is `at_start` given the previous value of the property.
    pub fn is_at_start(&self, prev: bool) -> bool {
        match self {
            Self::Space(1 ..) => true,
            Self::Space(_) | Self::LineComment | Self::BlockComment => prev,
            _ => false,
        }
    }

    /// Whether this node has to appear at the start of a line.
    pub fn only_at_start(&self) -> bool {
        match self {
            Self::Heading | Self::Enum | Self::List => true,
            Self::Text(t) => t == "-" || t.ends_with('.'),
            _ => false,
        }
    }

    /// Whether this is a node that only appears in markup.
    pub fn only_in_markup(&self) -> bool {
        matches!(
            self,
            Self::Text(_)
                | Self::Linebreak { .. }
                | Self::NonBreakingSpace
                | Self::Shy
                | Self::EnDash
                | Self::EmDash
                | Self::Ellipsis
                | Self::Quote { .. }
                | Self::Escape(_)
                | Self::Strong
                | Self::Emph
                | Self::Raw(_)
                | Self::Math(_)
                | Self::Heading
                | Self::List
                | Self::Enum
                | Self::EnumNumbering(_)
        )
    }

    /// A human-readable name for the kind.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LeftBrace => "opening brace",
            Self::RightBrace => "closing brace",
            Self::LeftBracket => "opening bracket",
            Self::RightBracket => "closing bracket",
            Self::LeftParen => "opening paren",
            Self::RightParen => "closing paren",
            Self::Star => "star",
            Self::Underscore => "underscore",
            Self::Comma => "comma",
            Self::Semicolon => "semicolon",
            Self::Colon => "colon",
            Self::Plus => "plus",
            Self::Minus => "minus",
            Self::Slash => "slash",
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
            Self::Not => "operator `not`",
            Self::And => "operator `and`",
            Self::Or => "operator `or`",
            Self::Dots => "dots",
            Self::Arrow => "arrow",
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
            Self::As => "keyword `as`",
            Self::While => "keyword `while`",
            Self::Break => "keyword `break`",
            Self::Continue => "keyword `continue`",
            Self::Return => "keyword `return`",
            Self::Import => "keyword `import`",
            Self::Include => "keyword `include`",
            Self::From => "keyword `from`",
            Self::Markup(_) => "markup",
            Self::Space(2 ..) => "paragraph break",
            Self::Space(_) => "space",
            Self::Linebreak { justified: false } => "linebreak",
            Self::Linebreak { justified: true } => "justified linebreak",
            Self::Text(_) => "text",
            Self::NonBreakingSpace => "non-breaking space",
            Self::Shy => "soft hyphen",
            Self::EnDash => "en dash",
            Self::EmDash => "em dash",
            Self::Ellipsis => "ellipsis",
            Self::Quote { double: false } => "single quote",
            Self::Quote { double: true } => "double quote",
            Self::Escape(_) => "escape sequence",
            Self::Strong => "strong content",
            Self::Emph => "emphasized content",
            Self::Raw(_) => "raw block",
            Self::Math(_) => "math formula",
            Self::List => "list item",
            Self::Heading => "heading",
            Self::Enum => "enumeration item",
            Self::EnumNumbering(_) => "enumeration item numbering",
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
            Self::LineComment => "line comment",
            Self::BlockComment => "block comment",
            Self::Error(_, _) => "parse error",
            Self::Unknown(src) => match src.as_str() {
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
            Self::LeftBrace => {}
            Self::RightBrace => {}
            Self::LeftBracket => {}
            Self::RightBracket => {}
            Self::LeftParen => {}
            Self::RightParen => {}
            Self::Star => {}
            Self::Underscore => {}
            Self::Comma => {}
            Self::Semicolon => {}
            Self::Colon => {}
            Self::Plus => {}
            Self::Minus => {}
            Self::Slash => {}
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
            Self::Not => {}
            Self::And => {}
            Self::Or => {}
            Self::Dots => {}
            Self::Arrow => {}
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
            Self::As => {}
            Self::While => {}
            Self::Break => {}
            Self::Continue => {}
            Self::Return => {}
            Self::Import => {}
            Self::Include => {}
            Self::From => {}
            Self::Markup(c) => c.hash(state),
            Self::Space(n) => n.hash(state),
            Self::Linebreak { justified } => justified.hash(state),
            Self::Text(s) => s.hash(state),
            Self::NonBreakingSpace => {}
            Self::Shy => {}
            Self::EnDash => {}
            Self::EmDash => {}
            Self::Ellipsis => {}
            Self::Quote { double } => double.hash(state),
            Self::Escape(c) => c.hash(state),
            Self::Strong => {}
            Self::Emph => {}
            Self::Raw(raw) => raw.hash(state),
            Self::Math(math) => math.hash(state),
            Self::List => {}
            Self::Heading => {}
            Self::Enum => {}
            Self::EnumNumbering(num) => num.hash(state),
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
            Self::LineComment => {}
            Self::BlockComment => {}
            Self::Error(pos, msg) => (pos, msg).hash(state),
            Self::Unknown(src) => src.hash(state),
        }
    }
}
