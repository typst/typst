//! Syntax types.

pub mod ast;
mod pretty;
mod span;

use std::fmt::{self, Debug, Display, Formatter};
use std::mem;
use std::rc::Rc;

pub use pretty::*;
pub use span::*;

use self::ast::TypedNode;
use crate::diag::Error;
use crate::geom::{AngularUnit, LengthUnit};
use crate::source::SourceId;
use crate::util::EcoString;

/// An inner of leaf node in the untyped green tree.
#[derive(Clone, PartialEq)]
pub enum Green {
    /// A reference-counted inner node.
    Node(Rc<GreenNode>),
    /// A terminal, owned token.
    Token(GreenData),
}

impl Green {
    /// Returns the metadata of the node.
    fn data(&self) -> &GreenData {
        match self {
            Green::Node(n) => &n.data,
            Green::Token(t) => &t,
        }
    }

    /// The type of the node.
    pub fn kind(&self) -> &NodeKind {
        self.data().kind()
    }

    /// Set the type of the node.
    pub fn set_kind(&mut self, kind: NodeKind) {
        match self {
            Self::Node(node) => Rc::make_mut(node).data.set_kind(kind),
            Self::Token(data) => data.set_kind(kind),
        }
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
            Green::Node(n) => &n.children(),
            Green::Token(_) => &[],
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
        write!(f, "{:?}: {}", self.kind(), self.len())?;
        if let Self::Node(n) = self {
            if !n.children.is_empty() {
                f.write_str(" ")?;
                f.debug_list().entries(&n.children).finish()?;
            }
        }
        Ok(())
    }
}

/// An inner node in the untyped green tree.
#[derive(Debug, Clone, PartialEq)]
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
}

impl From<GreenNode> for Green {
    fn from(node: GreenNode) -> Self {
        Rc::new(node).into()
    }
}

impl From<Rc<GreenNode>> for Green {
    fn from(node: Rc<GreenNode>) -> Self {
        Self::Node(node)
    }
}

/// Data shared between inner and leaf nodes.
#[derive(Debug, Clone, PartialEq)]
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

    /// Set the type of the node.
    pub fn set_kind(&mut self, kind: NodeKind) {
        self.kind = kind;
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

/// A borrowed wrapper for a [`GreenNode`] with span information.
///
/// Borrowed variant of [`RedNode`]. Can be [cast](Self::cast) to an AST node.
#[derive(Copy, Clone, PartialEq)]
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

    /// Whether the node or its children contain an error.
    pub fn erroneous(self) -> bool {
        self.green.erroneous()
    }

    /// The error messages for this node and its descendants.
    pub fn errors(self) -> Vec<Error> {
        if !self.erroneous() {
            return vec![];
        }

        match self.kind() {
            NodeKind::Error(pos, msg) => {
                let span = match pos {
                    ErrorPosition::Start => self.span().at_start(),
                    ErrorPosition::Full => self.span(),
                    ErrorPosition::End => self.span().at_end(),
                };

                vec![Error::new(span, msg.to_string())]
            }
            _ => self
                .children()
                .filter(|red| red.erroneous())
                .flat_map(|red| red.errors())
                .collect(),
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
    pub fn children(self) -> impl Iterator<Item = RedRef<'a>> {
        let children = match &self.green {
            Green::Node(node) => node.children(),
            Green::Token(_) => &[],
        };

        let mut cursor = self.offset;
        children.iter().map(move |green| {
            let offset = cursor;
            cursor += green.len();
            RedRef { id: self.id, offset, green }
        })
    }

    /// Get the first child of some type.
    pub(crate) fn typed_child(self, kind: &NodeKind) -> Option<RedRef<'a>> {
        self.children()
            .find(|x| mem::discriminant(x.kind()) == mem::discriminant(kind))
    }

    /// Get the first child that can cast to some AST type.
    pub(crate) fn cast_first_child<T: TypedNode>(self) -> Option<T> {
        self.children().find_map(RedRef::cast)
    }

    /// Get the last child that can cast to some AST type.
    pub(crate) fn cast_last_child<T: TypedNode>(self) -> Option<T> {
        self.children().filter_map(RedRef::cast).last()
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

/// A owned wrapper for a [`GreenNode`] with span information.
///
/// Owned variant of [`RedRef`]. Can be [cast](Self::cast) to an AST nodes.
#[derive(Clone, PartialEq)]
pub struct RedNode {
    id: SourceId,
    offset: usize,
    green: Green,
}

impl RedNode {
    /// Create a new root node from a [`GreenNode`].
    pub fn new_root(root: Rc<GreenNode>, id: SourceId) -> Self {
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
    pub fn children(&self) -> impl Iterator<Item = RedRef<'_>> {
        self.as_ref().children()
    }

    /// Get the first child of some type.
    pub(crate) fn typed_child(&self, kind: &NodeKind) -> Option<RedNode> {
        self.as_ref().typed_child(kind).map(RedRef::own)
    }

    /// Get the first child that can cast to some AST type.
    pub(crate) fn cast_first_child<T: TypedNode>(&self) -> Option<T> {
        self.as_ref().cast_first_child()
    }

    /// Get the last child that can cast to some AST type.
    pub(crate) fn cast_last_child<T: TypedNode>(&self) -> Option<T> {
        self.as_ref().cast_last_child()
    }
}

impl Debug for RedNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.as_ref().fmt(f)
    }
}

/// All syntactical building blocks that can be part of a Typst document.
///
/// Can be emitted as a token by the tokenizer or as part of a green node by
/// the parser.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    /// A left square bracket: `[`.
    LeftBracket,
    /// A right square bracket: `]`.
    RightBracket,
    /// A left curly brace: `{`.
    LeftBrace,
    /// A right curly brace: `}`.
    RightBrace,
    /// A left round parenthesis: `(`.
    LeftParen,
    /// A right round parenthesis: `)`.
    RightParen,
    /// An asterisk: `*`.
    Star,
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
    /// The `with` operator.
    With,
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
    /// Template markup.
    Markup,
    /// One or more whitespace characters.
    Space(usize),
    /// A forced line break: `\`.
    Linebreak,
    /// A paragraph break: Two or more newlines.
    Parbreak,
    /// A consecutive non-markup string.
    Text(EcoString),
    /// A non-breaking space: `~`.
    NonBreakingSpace,
    /// An en-dash: `--`.
    EnDash,
    /// An em-dash: `---`.
    EmDash,
    /// A slash and the letter "u" followed by a hexadecimal unicode entity
    /// enclosed in curly braces: `\u{1F5FA}`.
    UnicodeEscape(char),
    /// Strong text was enabled / disabled: `*`.
    Strong,
    /// Emphasized text was enabled / disabled: `_`.
    Emph,
    /// A section heading: `= Introduction`.
    Heading,
    /// An item in an enumeration (ordered list): `1. ...`.
    Enum,
    /// A numbering: `23.`.
    ///
    /// Can also exist without the number: `.`.
    EnumNumbering(Option<usize>),
    /// An item in an unordered list: `- ...`.
    List,
    /// The bullet character of an item in an unordered list: `-`.
    ListBullet,
    /// An arbitrary number of backticks followed by inner contents, terminated
    /// with the same number of backticks: `` `...` ``.
    Raw(Rc<RawData>),
    /// Dollar signs surrounding inner contents.
    Math(Rc<MathData>),
    /// An identifier: `center`.
    Ident(EcoString),
    /// A boolean: `true`, `false`.
    Bool(bool),
    /// An integer: `120`.
    Int(i64),
    /// A floating-point number: `1.2`, `10e-4`.
    Float(f64),
    /// A length: `12pt`, `3cm`.
    Length(f64, LengthUnit),
    /// An angle: `90deg`.
    Angle(f64, AngularUnit),
    /// A percentage: `50%`.
    ///
    /// _Note_: `50%` is stored as `50.0` here, as in the corresponding
    /// [literal](ast::Lit::Percent).
    Percentage(f64),
    /// A fraction unit: `3fr`.
    Fraction(f64),
    /// A quoted string: `"..."`.
    Str(EcoString),
    /// An array expression: `(1, "hi", 12cm)`.
    Array,
    /// A dictionary expression: `(thickness: 3pt, pattern: dashed)`.
    Dict,
    /// A named argument: `thickness: 3pt`.
    Named,
    /// A grouped expression: `(1 + 2)`.
    Group,
    /// A unary operation: `-x`.
    Unary,
    /// A binary operation: `a + b`.
    Binary,
    /// An invocation of a function: `f(x, y)`.
    Call,
    /// A function call's argument list: `(x, y)`.
    CallArgs,
    /// A closure expression: `(x, y) => z`.
    Closure,
    /// A closure's parameters: `(x, y)`.
    ClosureParams,
    /// A parameter sink: `..x`.
    Spread,
    /// A template expression: `[*Hi* there!]`.
    Template,
    /// A block expression: `{ let x = 1; x + 2 }`.
    Block,
    /// A for loop expression: `for x in y { ... }`.
    ForExpr,
    /// A while loop expression: `while x { ... }`.
    WhileExpr,
    /// An if expression: `if x { ... }`.
    IfExpr,
    /// A let expression: `let x = 1`.
    LetExpr,
    /// The `with` expression: `with (1)`.
    WithExpr,
    /// A for loop's destructuring pattern: `x` or `x, y`.
    ForPattern,
    /// The import expression: `import x from "foo.typ"`.
    ImportExpr,
    /// Items to import: `a, b, c`.
    ImportItems,
    /// The include expression: `include "foo.typ"`.
    IncludeExpr,
    /// Two slashes followed by inner contents, terminated with a newline:
    /// `//<str>\n`.
    LineComment,
    /// A slash and a star followed by inner contents,  terminated with a star
    /// and a slash: `/*<str>*/`.
    ///
    /// The comment can contain nested block comments.
    BlockComment,
    /// Tokens that appear in the wrong place.
    Error(ErrorPosition, EcoString),
    /// Unknown character sequences.
    Unknown(EcoString),
}

/// Payload of a raw block: `` `...` ``.
#[derive(Debug, Clone, PartialEq)]
pub struct RawData {
    /// The raw text in the block.
    pub text: EcoString,
    /// The programming language of the raw text.
    pub lang: Option<EcoString>,
    /// The number of opening backticks.
    pub backticks: u8,
    /// Whether to display this as a block.
    pub block: bool,
}

/// Payload of a math formula: `$2pi + x$` or `$[f'(x) = x^2]$`.
#[derive(Debug, Clone, PartialEq)]
pub struct MathData {
    /// The formula between the dollars.
    pub formula: EcoString,
    /// Whether the formula is display-level, that is, it is surrounded by
    /// `$[..]`.
    pub display: bool,
}

/// Where in a node an error should be annotated.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ErrorPosition {
    /// At the start of the node.
    Start,
    /// Over the full width of the node.
    Full,
    /// At the end of the node.
    End,
}

impl NodeKind {
    /// Whether this is some kind of parenthesis.
    pub fn is_paren(&self) -> bool {
        matches!(self, Self::LeftParen | Self::RightParen)
    }

    /// Whether this is some kind of bracket.
    pub fn is_bracket(&self) -> bool {
        matches!(self, Self::LeftBracket | Self::RightBracket)
    }

    /// Whether this is some kind of brace.
    pub fn is_brace(&self) -> bool {
        matches!(self, Self::LeftBrace | Self::RightBrace)
    }

    /// Whether this is some kind of error.
    pub fn is_error(&self) -> bool {
        matches!(self, NodeKind::Error(_, _) | NodeKind::Unknown(_))
    }

    /// A human-readable name for the kind.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LeftBracket => "opening bracket",
            Self::RightBracket => "closing bracket",
            Self::LeftBrace => "opening brace",
            Self::RightBrace => "closing brace",
            Self::LeftParen => "opening paren",
            Self::RightParen => "closing paren",
            Self::Star => "star",
            Self::Comma => "comma",
            Self::Semicolon => "semicolon",
            Self::Colon => "colon",
            Self::Plus => "plus",
            Self::Minus => "minus",
            Self::Slash => "slash",
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
            Self::With => "operator `with`",
            Self::Dots => "dots",
            Self::Arrow => "arrow",
            Self::None => "`none`",
            Self::Auto => "`auto`",
            Self::Let => "keyword `let`",
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
            Self::Markup => "markup",
            Self::Space(_) => "space",
            Self::Linebreak => "forced linebreak",
            Self::Parbreak => "paragraph break",
            Self::Text(_) => "text",
            Self::NonBreakingSpace => "non-breaking space",
            Self::EnDash => "en dash",
            Self::EmDash => "em dash",
            Self::UnicodeEscape(_) => "unicode escape sequence",
            Self::Strong => "strong",
            Self::Emph => "emphasis",
            Self::Heading => "heading",
            Self::Enum => "enumeration item",
            Self::EnumNumbering(_) => "enumeration item numbering",
            Self::List => "list item",
            Self::ListBullet => "list bullet",
            Self::Raw(_) => "raw block",
            Self::Math(_) => "math formula",
            Self::Ident(_) => "identifier",
            Self::Bool(_) => "boolean",
            Self::Int(_) => "integer",
            Self::Float(_) => "float",
            Self::Length(_, _) => "length",
            Self::Angle(_, _) => "angle",
            Self::Percentage(_) => "percentage",
            Self::Fraction(_) => "`fr` value",
            Self::Str(_) => "string",
            Self::Array => "array",
            Self::Dict => "dictionary",
            Self::Named => "named argument",
            Self::Group => "group",
            Self::Unary => "unary expression",
            Self::Binary => "binary expression",
            Self::Call => "call",
            Self::CallArgs => "call arguments",
            Self::Closure => "closure",
            Self::ClosureParams => "closure parameters",
            Self::Spread => "parameter sink",
            Self::Template => "template",
            Self::Block => "block",
            Self::ForExpr => "for-loop expression",
            Self::WhileExpr => "while-loop expression",
            Self::IfExpr => "`if` expression",
            Self::LetExpr => "`let` expression",
            Self::WithExpr => "`with` expression",
            Self::ForPattern => "for-loop destructuring pattern",
            Self::ImportExpr => "`import` expression",
            Self::ImportItems => "import items",
            Self::IncludeExpr => "`include` expression",
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
