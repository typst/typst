//! Syntax types.

mod expr;
mod ident;
mod markup;
mod pretty;
mod span;
mod token;

use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::mem;
use std::rc::Rc;

pub use expr::*;
pub use ident::*;
pub use markup::*;
pub use pretty::*;
pub use span::*;
pub use token::*;

use crate::geom::{AngularUnit, LengthUnit};
use crate::source::SourceId;
use crate::util::EcoString;

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
    /// Two dots: `..`.
    Dots,
    /// An equals sign followed by a greater-than sign: `=>`.
    Arrow,
    /// The `not` operator.
    Not,
    /// The `and` operator.
    And,
    /// The `or` operator.
    Or,
    /// The `with` operator.
    With,
    /// The `with` expression: `with (1)`.
    WithExpr,
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
    /// One or more whitespace characters.
    Space(usize),
    /// A consecutive non-markup string.
    Text(EcoString),
    /// A slash and the letter "u" followed by a hexadecimal unicode entity
    /// enclosed in curly braces: `\u{1F5FA}`.
    UnicodeEscape(Rc<UnicodeEscapeToken>),
    /// An arbitrary number of backticks followed by inner contents, terminated
    /// with the same number of backticks: `` `...` ``.
    Raw(Rc<RawToken>),
    /// Dollar signs surrounding inner contents.
    Math(Rc<MathToken>),
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
    /// A length: `12pt`, `3cm`.
    Length(f64, LengthUnit),
    /// An angle: `90deg`.
    Angle(f64, AngularUnit),
    /// A percentage: `50%`.
    ///
    /// _Note_: `50%` is stored as `50.0` here, as in the corresponding
    /// [literal](super::Lit::Percent).
    Percentage(f64),
    /// A fraction unit: `3fr`.
    Fraction(f64),
    /// A quoted string: `"..."`.
    Str(StrToken),
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
    /// Template markup.
    Markup,
    /// A forced line break: `\`.
    Linebreak,
    /// A paragraph break: Two or more newlines.
    Parbreak,
    /// Strong text was enabled / disabled: `*`.
    Strong,
    /// Emphasized text was enabled / disabled: `_`.
    Emph,
    /// A non-breaking space: `~`.
    NonBreakingSpace,
    /// An en-dash: `--`.
    EnDash,
    /// An em-dash: `---`.
    EmDash,
    /// A section heading: `= Introduction`.
    Heading,
    /// A heading's level: `=`, `==`, `===`, etc.
    HeadingLevel(u8),
    /// An item in an unordered list: `- ...`.
    List,
    /// The bullet character of an item in an unordered list: `-`.
    ListBullet,
    /// An item in an enumeration (ordered list): `1. ...`.
    Enum,
    /// An array expression: `(1, "hi", 12cm)`.
    Array,
    /// A dictionary expression: `(thickness: 3pt, pattern: dashed)`.
    Dict,
    /// A named argument: `thickness: 3pt`.
    Named,
    /// A template expression: `[*Hi* there!]`.
    Template,
    /// A grouped expression: `(1 + 2)`.
    Group,
    /// A block expression: `{ let x = 1; x + 2 }`.
    Block,
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
    ParameterSink,
    /// A for loop expression: `for x in y { ... }`.
    ForExpr,
    /// A while loop expression: `while x { ... }`.
    WhileExpr,
    /// An if expression: `if x { ... }`.
    IfExpr,
    /// A let expression: `let x = 1`.
    LetExpr,
    /// A for loop's destructuring pattern: `x` or `x, y`.
    ForPattern,
    /// The import expression: `import x from "foo.typ"`.
    ImportExpr,
    /// Items to import: `a, b, c`.
    ImportItems,
    /// The include expression: `include "foo.typ"`.
    IncludeExpr,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ErrorPosition {
    /// At the start of the node.
    Start,
    /// Over the full width of the node.
    Full,
    /// At the end of the node.
    End,
}

impl Display for NodeKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.pad(self.as_str())
    }
}

impl NodeKind {
    pub fn is_parenthesis(&self) -> bool {
        match self {
            Self::LeftParen => true,
            Self::RightParen => true,
            _ => false,
        }
    }

    pub fn is_bracket(&self) -> bool {
        match self {
            Self::LeftBracket => true,
            Self::RightBracket => true,
            _ => false,
        }
    }

    pub fn is_brace(&self) -> bool {
        match self {
            Self::LeftBrace => true,
            Self::RightBrace => true,
            _ => false,
        }
    }

    pub fn is_error(&self) -> bool {
        matches!(self, NodeKind::Error(_, _))
    }

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
            Self::Dots => "dots",
            Self::Arrow => "arrow",
            Self::Not => "operator `not`",
            Self::And => "operator `and`",
            Self::Or => "operator `or`",
            Self::With => "operator `with`",
            Self::WithExpr => "`with` expression",
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
            Self::Space(_) => "space",
            Self::Math(_) => "math formula",
            Self::EnumNumbering(_) => "numbering",
            Self::Str(_) => "string",
            Self::LineComment => "line comment",
            Self::BlockComment => "block comment",
            Self::Markup => "markup",
            Self::Linebreak => "forced linebreak",
            Self::Parbreak => "paragraph break",
            Self::Strong => "strong",
            Self::Emph => "emphasis",
            Self::Text(_) => "text",
            Self::NonBreakingSpace => "non-breaking space",
            Self::EnDash => "en dash",
            Self::EmDash => "em dash",
            Self::UnicodeEscape(_) => "unicode escape sequence",
            Self::Raw(_) => "raw block",
            Self::Heading => "heading",
            Self::HeadingLevel(_) => "heading level",
            Self::List => "list",
            Self::ListBullet => "list bullet",
            Self::Enum => "enum",
            Self::Ident(_) => "identifier",
            Self::Bool(_) => "boolean",
            Self::Int(_) => "integer",
            Self::Float(_) => "float",
            Self::Length(_, _) => "length",
            Self::Angle(_, _) => "angle",
            Self::Percentage(_) => "percentage",
            Self::Fraction(_) => "`fr` value",
            Self::Array => "array",
            Self::Dict => "dictionary",
            Self::Named => "named argument",
            Self::Template => "template",
            Self::Group => "group",
            Self::Block => "block",
            Self::Unary => "unary expression",
            Self::Binary => "binary expression",
            Self::Call => "call",
            Self::CallArgs => "call arguments",
            Self::Closure => "closure",
            Self::ClosureParams => "closure parameters",
            Self::ParameterSink => "parameter sink",
            Self::ForExpr => "for-loop expression",
            Self::WhileExpr => "while-loop expression",
            Self::IfExpr => "if expression",
            Self::LetExpr => "let expression",
            Self::ForPattern => "for-loop destructuring pattern",
            Self::ImportExpr => "import expression",
            Self::ImportItems => "import items",
            Self::IncludeExpr => "include expression",
            Self::Unknown(src) => match src.as_str() {
                "*/" => "end of block comment",
                _ => "invalid token",
            },
            Self::Error(_, _) => "parse error",
        }
    }
}

/// A syntactical node.
#[derive(Clone, PartialEq)]
pub struct GreenNode {
    /// Node metadata.
    data: GreenData,
    /// This node's children, losslessly make up this node.
    children: Vec<Green>,
}

/// Data shared between [`GreenNode`]s and [`GreenToken`]s.
#[derive(Clone, PartialEq)]
pub struct GreenData {
    /// What kind of node this is (each kind would have its own struct in a
    /// strongly typed AST).
    kind: NodeKind,
    /// The byte length of the node in the source.
    len: usize,
    /// Whether this node or any of its children are erroneous.
    erroneous: bool,
}

impl GreenData {
    pub fn new(kind: NodeKind, len: usize) -> Self {
        Self { len, erroneous: kind.is_error(), kind }
    }

    pub fn kind(&self) -> &NodeKind {
        &self.kind
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn erroneous(&self) -> bool {
        self.erroneous
    }
}

impl From<GreenData> for Green {
    fn from(token: GreenData) -> Self {
        Self::Token(token)
    }
}

/// Children of a [`GreenNode`].
#[derive(Clone, PartialEq)]
pub enum Green {
    /// A terminal owned token.
    Token(GreenData),
    /// A non-terminal node in an Rc.
    Node(Rc<GreenNode>),
}

impl Green {
    fn data(&self) -> &GreenData {
        match self {
            Green::Token(t) => &t,
            Green::Node(n) => &n.data,
        }
    }

    pub fn kind(&self) -> &NodeKind {
        self.data().kind()
    }

    pub fn len(&self) -> usize {
        self.data().len()
    }

    pub fn erroneous(&self) -> bool {
        self.data().erroneous()
    }

    pub fn children(&self) -> &[Green] {
        match self {
            Green::Token(_) => &[],
            Green::Node(n) => &n.children(),
        }
    }
}

impl GreenNode {
    pub fn new(kind: NodeKind, len: usize) -> Self {
        Self {
            data: GreenData::new(kind, len),
            children: Vec::new(),
        }
    }

    pub fn with_children(kind: NodeKind, len: usize, children: Vec<Green>) -> Self {
        let mut meta = GreenData::new(kind, len);
        meta.erroneous |= children.iter().any(|c| c.erroneous());
        Self { data: meta, children }
    }

    pub fn with_child(kind: NodeKind, len: usize, child: impl Into<Green>) -> Self {
        Self::with_children(kind, len, vec![child.into()])
    }

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

#[derive(Copy, Clone, PartialEq)]
pub struct RedRef<'a> {
    id: SourceId,
    offset: usize,
    green: &'a Green,
}

impl<'a> RedRef<'a> {
    pub fn own(self) -> RedNode {
        RedNode {
            id: self.id,
            offset: self.offset,
            green: self.green.clone(),
        }
    }

    pub fn kind(&self) -> &NodeKind {
        self.green.kind()
    }

    pub fn span(&self) -> Span {
        Span::new(self.id, self.offset, self.offset + self.green.len())
    }

    pub fn cast<T>(self) -> Option<T>
    where
        T: TypedNode,
    {
        T::cast_from(self)
    }

    pub fn erroneous(&self) -> bool {
        self.green.erroneous()
    }

    pub fn children(self) -> impl Iterator<Item = RedRef<'a>> + Clone {
        let children = match &self.green {
            Green::Node(node) => node.children(),
            Green::Token(_) => &[],
        };

        let mut offset = self.offset;
        children.iter().map(move |green| {
            let child_offset = offset;
            offset += green.len();
            RedRef { id: self.id, offset: child_offset, green }
        })
    }

    pub(crate) fn typed_child(&self, kind: &NodeKind) -> Option<RedRef> {
        self.children()
            .find(|x| mem::discriminant(x.kind()) == mem::discriminant(kind))
    }

    pub(crate) fn cast_first_child<T: TypedNode>(&self) -> Option<T> {
        self.children().find_map(RedRef::cast)
    }

    pub(crate) fn cast_last_child<T: TypedNode>(&self) -> Option<T> {
        self.children().filter_map(RedRef::cast).last()
    }
}

#[derive(Clone, PartialEq)]
pub struct RedNode {
    id: SourceId,
    offset: usize,
    green: Green,
}

impl RedNode {
    pub fn new_root(root: Rc<GreenNode>, id: SourceId) -> Self {
        Self { id, offset: 0, green: root.into() }
    }

    pub fn span(&self) -> Span {
        self.as_ref().span()
    }

    pub fn len(&self) -> usize {
        self.green.len()
    }

    pub fn kind(&self) -> &NodeKind {
        self.green.kind()
    }

    pub fn children<'a>(&'a self) -> impl Iterator<Item = RedRef<'a>> + Clone {
        self.as_ref().children()
    }

    pub fn errors(&self) -> Vec<(Span, EcoString)> {
        if !self.green.erroneous() {
            return vec![];
        }

        match self.kind() {
            NodeKind::Error(pos, msg) => {
                let span = match pos {
                    ErrorPosition::Start => self.span().at_start(),
                    ErrorPosition::Full => self.span(),
                    ErrorPosition::End => self.span().at_end(),
                };

                vec![(span, msg.clone())]
            }
            _ => self
                .as_ref()
                .children()
                .filter(|red| red.green.erroneous())
                .flat_map(|red| red.own().errors())
                .collect(),
        }
    }

    pub fn as_ref<'a>(&'a self) -> RedRef<'a> {
        RedRef {
            id: self.id,
            offset: self.offset,
            green: &self.green,
        }
    }

    pub(crate) fn typed_child(&self, kind: &NodeKind) -> Option<RedNode> {
        self.as_ref().typed_child(kind).map(RedRef::own)
    }

    pub(crate) fn cast_first_child<T: TypedNode>(&self) -> Option<T> {
        self.as_ref().cast_first_child()
    }

    pub(crate) fn cast_last_child<T: TypedNode>(&self) -> Option<T> {
        self.as_ref().cast_last_child()
    }
}

impl Debug for RedNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}: {:?}", self.kind(), self.span())?;
        let children = self.as_ref().children().collect::<Vec<_>>();
        if !children.is_empty() {
            f.write_str(" ")?;
            f.debug_list()
                .entries(children.into_iter().map(RedRef::own))
                .finish()?;
        }
        Ok(())
    }
}

pub trait TypedNode: Sized {
    /// Performs the conversion.
    fn cast_from(value: RedRef) -> Option<Self>;
}

#[macro_export]
macro_rules! node {
    ($(#[$attr:meta])* $name:ident) => {
        node!{$(#[$attr])* $name => $name}
    };
    ($(#[$attr:meta])* $variant:ident => $name:ident) => {
        #[derive(Debug, Clone, PartialEq)]
        #[repr(transparent)]
        $(#[$attr])*
        pub struct $name(RedNode);

        impl TypedNode for $name {
            fn cast_from(node: RedRef) -> Option<Self> {
                if node.kind() != &NodeKind::$variant {
                    return None;
                }

                Some(Self(node.own()))
            }
        }

        impl $name {
            pub fn span(&self) -> Span {
                self.0.span()
            }

            pub fn underlying(&self) -> RedRef {
                self.0.as_ref()
            }
        }
    };
}
