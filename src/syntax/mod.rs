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
use crate::diag::Error;
use crate::parse::TokenMode;
use crate::source::SourceId;
use crate::util::EcoString;

/// An inner or leaf node in the untyped green tree.
#[derive(Clone, PartialEq, Hash)]
pub enum Green {
    /// A reference-counted inner node.
    Node(Arc<GreenNode>),
    /// A terminal, owned token.
    Token(GreenData),
}

impl Green {
    /// Returns the metadata of the node.
    fn data(&self) -> &GreenData {
        match self {
            Green::Node(n) => &n.data,
            Green::Token(t) => t,
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

    /// Whether the node or its children contain an error.
    pub fn erroneous(&self) -> bool {
        match self {
            Self::Node(node) => node.erroneous,
            Self::Token(data) => data.kind.is_error(),
        }
    }

    /// The node's children.
    pub fn children(&self) -> &[Green] {
        match self {
            Green::Node(n) => n.children(),
            Green::Token(_) => &[],
        }
    }

    /// Whether the node is a leaf node in the green tree.
    pub fn is_leaf(&self) -> bool {
        match self {
            Green::Node(n) => n.children().is_empty(),
            Green::Token(_) => true,
        }
    }

    /// Change the type of the node.
    pub fn convert(&mut self, kind: NodeKind) {
        match self {
            Self::Node(node) => {
                let node = Arc::make_mut(node);
                node.erroneous |= kind.is_error();
                node.data.kind = kind;
            }
            Self::Token(data) => data.kind = kind,
        }
    }
}

impl Default for Green {
    fn default() -> Self {
        Self::Token(GreenData::new(NodeKind::None, 0))
    }
}

impl Debug for Green {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Node(node) => node.fmt(f),
            Self::Token(token) => token.fmt(f),
        }
    }
}

/// An inner node in the untyped green tree.
#[derive(Clone, PartialEq, Hash)]
pub struct GreenNode {
    /// Node metadata.
    data: GreenData,
    /// This node's children, losslessly make up this node.
    children: Vec<Green>,
    /// Whether this node or any of its children are erroneous.
    erroneous: bool,
}

impl GreenNode {
    /// Creates a new node with the given kind and a single child.
    pub fn with_child(kind: NodeKind, child: impl Into<Green>) -> Self {
        Self::with_children(kind, vec![child.into()])
    }

    /// Creates a new node with the given kind and children.
    pub fn with_children(kind: NodeKind, children: Vec<Green>) -> Self {
        let mut erroneous = kind.is_error();
        let len = children
            .iter()
            .inspect(|c| erroneous |= c.erroneous())
            .map(Green::len)
            .sum();

        Self {
            data: GreenData::new(kind, len),
            children,
            erroneous,
        }
    }

    /// The node's children.
    pub fn children(&self) -> &[Green] {
        &self.children
    }

    /// The node's metadata.
    fn data(&self) -> &GreenData {
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

    /// The node's children, mutably.
    pub(crate) fn children_mut(&mut self) -> &mut [Green] {
        &mut self.children
    }

    /// Replaces a range of children with some replacement.
    pub(crate) fn replace_children(
        &mut self,
        range: Range<usize>,
        replacement: Vec<Green>,
    ) {
        let superseded = &self.children[range.clone()];
        let superseded_len: usize = superseded.iter().map(Green::len).sum();
        let replacement_len: usize = replacement.iter().map(Green::len).sum();

        // If we're erroneous, but not due to the superseded range, then we will
        // still be erroneous after the replacement.
        let still_erroneous = self.erroneous && !superseded.iter().any(Green::erroneous);

        self.children.splice(range, replacement);
        self.data.len = self.data.len + replacement_len - superseded_len;
        self.erroneous = still_erroneous || self.children.iter().any(Green::erroneous);
    }

    /// Update the length of this node given the old and new length of
    /// replaced children.
    pub(crate) fn update_parent(&mut self, new_len: usize, old_len: usize) {
        self.data.len = self.data.len() + new_len - old_len;
        self.erroneous = self.children.iter().any(Green::erroneous);
    }
}

impl From<GreenNode> for Green {
    fn from(node: GreenNode) -> Self {
        Arc::new(node).into()
    }
}

impl From<Arc<GreenNode>> for Green {
    fn from(node: Arc<GreenNode>) -> Self {
        Self::Node(node)
    }
}

impl Debug for GreenNode {
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
#[derive(Clone, PartialEq, Hash)]
pub struct GreenData {
    /// What kind of node this is (each kind would have its own struct in a
    /// strongly typed AST).
    kind: NodeKind,
    /// The byte length of the node in the source.
    len: usize,
}

impl GreenData {
    /// Create new node metadata.
    pub fn new(kind: NodeKind, len: usize) -> Self {
        Self { len, kind }
    }

    /// The type of the node.
    pub fn kind(&self) -> &NodeKind {
        &self.kind
    }

    /// The length of the node.
    pub fn len(&self) -> usize {
        self.len
    }
}

impl From<GreenData> for Green {
    fn from(token: GreenData) -> Self {
        Self::Token(token)
    }
}

impl Debug for GreenData {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.len)
    }
}

/// A owned wrapper for a green node with span information.
///
/// Owned variant of [`RedRef`]. Can be [cast](Self::cast) to an AST node.
#[derive(Clone, PartialEq, Hash)]
pub struct RedNode {
    id: SourceId,
    offset: usize,
    green: Green,
}

impl RedNode {
    /// Create a new red node from a root [`GreenNode`].
    pub fn from_root(root: Arc<GreenNode>, id: SourceId) -> Self {
        Self { id, offset: 0, green: root.into() }
    }

    /// Convert to a borrowed representation.
    pub fn as_ref(&self) -> RedRef<'_> {
        RedRef {
            id: self.id,
            offset: self.offset,
            green: &self.green,
        }
    }

    /// The type of the node.
    pub fn kind(&self) -> &NodeKind {
        self.as_ref().kind()
    }

    /// The length of the node.
    pub fn len(&self) -> usize {
        self.as_ref().len()
    }

    /// The span of the node.
    pub fn span(&self) -> Span {
        self.as_ref().span()
    }

    /// The error messages for this node and its descendants.
    pub fn errors(&self) -> Vec<Error> {
        self.as_ref().errors()
    }

    /// Convert the node to a typed AST node.
    pub fn cast<T>(self) -> Option<T>
    where
        T: TypedNode,
    {
        self.as_ref().cast()
    }

    /// The children of the node.
    pub fn children(&self) -> Children<'_> {
        self.as_ref().children()
    }

    /// Get the first child that can cast to some AST type.
    pub fn cast_first_child<T: TypedNode>(&self) -> Option<T> {
        self.as_ref().cast_first_child()
    }

    /// Get the last child that can cast to some AST type.
    pub fn cast_last_child<T: TypedNode>(&self) -> Option<T> {
        self.as_ref().cast_last_child()
    }
}

impl Debug for RedNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.as_ref().fmt(f)
    }
}

/// A borrowed wrapper for a [`GreenNode`] with span information.
///
/// Borrowed variant of [`RedNode`]. Can be [cast](Self::cast) to an AST node.
#[derive(Copy, Clone, PartialEq, Hash)]
pub struct RedRef<'a> {
    id: SourceId,
    offset: usize,
    green: &'a Green,
}

impl<'a> RedRef<'a> {
    /// Convert to an owned representation.
    pub fn own(self) -> RedNode {
        RedNode {
            id: self.id,
            offset: self.offset,
            green: self.green.clone(),
        }
    }

    /// The type of the node.
    pub fn kind(self) -> &'a NodeKind {
        self.green.kind()
    }

    /// The length of the node.
    pub fn len(self) -> usize {
        self.green.len()
    }

    /// The span of the node.
    pub fn span(self) -> Span {
        Span::new(self.id, self.offset, self.offset + self.green.len())
    }

    /// Whether the node is a leaf node.
    pub fn is_leaf(self) -> bool {
        self.green.is_leaf()
    }

    /// The error messages for this node and its descendants.
    pub fn errors(self) -> Vec<Error> {
        if !self.green.erroneous() {
            return vec![];
        }

        match self.kind() {
            NodeKind::Error(pos, msg) => {
                let span = match pos {
                    ErrorPos::Start => self.span().at_start(),
                    ErrorPos::Full => self.span(),
                    ErrorPos::End => self.span().at_end(),
                };

                vec![Error::new(span, msg.to_string())]
            }
            _ => self
                .children()
                .filter(|red| red.green.erroneous())
                .flat_map(|red| red.errors())
                .collect(),
        }
    }

    /// Returns all leaf descendants of this node (may include itself).
    pub fn leafs(self) -> Vec<Self> {
        if self.is_leaf() {
            vec![self]
        } else {
            self.children().flat_map(Self::leafs).collect()
        }
    }

    /// Convert the node to a typed AST node.
    pub fn cast<T>(self) -> Option<T>
    where
        T: TypedNode,
    {
        T::from_red(self)
    }

    /// The node's children.
    pub fn children(self) -> Children<'a> {
        let children = match &self.green {
            Green::Node(node) => node.children(),
            Green::Token(_) => &[],
        };

        Children {
            id: self.id,
            iter: children.iter(),
            front: self.offset,
            back: self.offset + self.len(),
        }
    }

    /// Get the first child that can cast to some AST type.
    pub fn cast_first_child<T: TypedNode>(self) -> Option<T> {
        self.children().find_map(RedRef::cast)
    }

    /// Get the last child that can cast to some AST type.
    pub fn cast_last_child<T: TypedNode>(self) -> Option<T> {
        self.children().rev().find_map(RedRef::cast)
    }
}

impl Debug for RedRef<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}: {:?}", self.kind(), self.span())?;
        let mut children = self.children().peekable();
        if children.peek().is_some() {
            f.write_str(" ")?;
            f.debug_list().entries(children.map(RedRef::own)).finish()?;
        }
        Ok(())
    }
}

/// An iterator over the children of a red node.
pub struct Children<'a> {
    id: SourceId,
    iter: std::slice::Iter<'a, Green>,
    front: usize,
    back: usize,
}

impl<'a> Iterator for Children<'a> {
    type Item = RedRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|green| {
            let offset = self.front;
            self.front += green.len();
            RedRef { id: self.id, offset, green }
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl DoubleEndedIterator for Children<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back().map(|green| {
            self.back -= green.len();
            RedRef { id: self.id, offset: self.back, green }
        })
    }
}

impl ExactSizeIterator for Children<'_> {}

/// All syntactical building blocks that can be part of a Typst document.
///
/// Can be emitted as a token by the tokenizer or as part of a green node by
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
    /// A forced line break. If soft (`\`, `true`), the preceding line can still
    /// be justified, if hard (`\+`, `false`) not.
    Linebreak(bool),
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
    /// A smart quote: `'` (`false`) or `"` (true).
    Quote(bool),
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
    /// Spreaded arguments or a parameter sink: `..x`.
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

/// Where in a node an error should be annotated.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ErrorPos {
    /// At the start of the node.
    Start,
    /// Over the full width of the node.
    Full,
    /// At the end of the node.
    End,
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

    /// Which mode this node can appear in, in both if `None`.
    pub fn only_in_mode(&self) -> Option<TokenMode> {
        match self {
            Self::Markup(_)
            | Self::Linebreak(_)
            | Self::Text(_)
            | Self::NonBreakingSpace
            | Self::EnDash
            | Self::EmDash
            | Self::Ellipsis
            | Self::Quote(_)
            | Self::Escape(_)
            | Self::Strong
            | Self::Emph
            | Self::Heading
            | Self::Enum
            | Self::EnumNumbering(_)
            | Self::List
            | Self::Raw(_)
            | Self::Math(_) => Some(TokenMode::Markup),
            Self::ContentBlock
            | Self::Space(_)
            | Self::Ident(_)
            | Self::CodeBlock
            | Self::LetExpr
            | Self::SetExpr
            | Self::ShowExpr
            | Self::WrapExpr
            | Self::IfExpr
            | Self::WhileExpr
            | Self::ForExpr
            | Self::ImportExpr
            | Self::FuncCall
            | Self::IncludeExpr
            | Self::LineComment
            | Self::BlockComment
            | Self::Error(_, _)
            | Self::Minus
            | Self::Eq => None,
            _ => Some(TokenMode::Code),
        }
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
            Self::Linebreak(false) => "hard linebreak",
            Self::Linebreak(true) => "soft linebreak",
            Self::Text(_) => "text",
            Self::NonBreakingSpace => "non-breaking space",
            Self::Shy => "soft hyphen",
            Self::EnDash => "en dash",
            Self::EmDash => "em dash",
            Self::Ellipsis => "ellipsis",
            Self::Quote(false) => "single quote",
            Self::Quote(true) => "double quote",
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
            Self::Named => "named argument",
            Self::UnaryExpr => "unary expression",
            Self::BinaryExpr => "binary expression",
            Self::FieldAccess => "field access",
            Self::FuncCall => "function call",
            Self::MethodCall => "method call",
            Self::CallArgs => "call arguments",
            Self::Spread => "parameter sink",
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
            Self::Linebreak(s) => s.hash(state),
            Self::Text(s) => s.hash(state),
            Self::NonBreakingSpace => {}
            Self::Shy => {}
            Self::EnDash => {}
            Self::EmDash => {}
            Self::Ellipsis => {}
            Self::Quote(d) => d.hash(state),
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
