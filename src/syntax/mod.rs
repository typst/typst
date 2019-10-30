//! Tokenization and parsing of source code.

use std::fmt::{self, Display, Formatter};

use crate::func::Function;
use crate::size::Size;

mod tokens;
mod parsing;

pub use tokens::{tokenize, Tokens};
pub use parsing::{parse, ParseContext, ParseError, ParseResult};

/// A logical unit of the incoming text stream.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Token<'s> {
    /// One or more whitespace (non-newline) codepoints.
    Space,
    /// A line feed (`\n`, `\r\n` and some more as defined by the Unicode standard).
    Newline,
    /// A left bracket: `[`.
    LeftBracket,
    /// A right bracket: `]`.
    RightBracket,
    /// A colon (`:`) indicating the beginning of function arguments (Function
    /// header only).
    ///
    /// If a colon occurs outside of a function header, it will be tokenized as
    /// [Text](Token::Text), just like the other tokens annotated with
    /// _Function header only_.
    Colon,
    /// An equals (`=`) sign assigning a function argument a value (Function header only).
    Equals,
    /// A comma (`,`) separating two function arguments (Function header only).
    Comma,
    /// Quoted text as a string value (Function header only).
    Quoted(&'s str),
    /// An underscore, indicating text in italics (Body only).
    Underscore,
    /// A star, indicating bold text (Body only).
    Star,
    /// A backtick, indicating monospace text (Body only).
    Backtick,
    /// A line comment.
    LineComment(&'s str),
    /// A block comment.
    BlockComment(&'s str),
    /// A star followed by a slash unexpectedly ending a block comment
    /// (the comment was not started before, otherwise a
    /// [BlockComment](Token::BlockComment would be returned).
    StarSlash,
    /// A unit of Plain text.
    Text(&'s str),
}

/// A tree representation of source code.
#[derive(Debug, PartialEq)]
pub struct SyntaxTree {
    pub nodes: Vec<Node>,
}

impl SyntaxTree {
    /// Create an empty syntax tree.
    #[inline]
    pub fn new() -> SyntaxTree {
        SyntaxTree { nodes: vec![] }
    }
}

/// A node in the syntax tree.
#[derive(Debug, PartialEq)]
pub enum Node {
    /// Whitespace.
    Space,
    /// A line feed.
    Newline,
    /// Indicates that italics were enabled / disabled.
    ToggleItalics,
    /// Indicates that boldface was enabled / disabled.
    ToggleBold,
    /// Indicates that monospace was enabled / disabled.
    ToggleMonospace,
    /// Literal text.
    Text(String),
    /// A function invocation.
    Func(FuncCall),
}

/// A function invocation, consisting of header and a dynamically parsed body.
#[derive(Debug)]
pub struct FuncCall {
    pub header: FuncHeader,
    pub body: Box<dyn Function>,
}

/// Contains header information of a function invocation.
#[derive(Debug, Clone, PartialEq)]
pub struct FuncHeader {
    pub name: String,
    pub args: Vec<Expression>,
    pub kwargs: Vec<(String, Expression)>,
}

/// An argument or return value.
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Ident(String),
    Str(String),
    Number(f64),
    Size(Size),
    Bool(bool),
}

impl PartialEq for FuncCall {
    fn eq(&self, other: &FuncCall) -> bool {
        (self.header == other.header) && (&self.body == &other.body)
    }
}

impl Display for Expression {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        use Expression::*;
        match self {
            Ident(s) => write!(f, "{}", s),
            Str(s) => write!(f, "{:?}", s),
            Number(n) => write!(f, "{}", n),
            Size(s) => write!(f, "{}", s),
            Bool(b) => write!(f, "{}", b),
        }
    }
}

pub struct Spanned<T> {
    pub val: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(val: T, span: Span) -> Spanned<T> {
        Spanned { val, span }
    }
}

pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Span {
        Span { start, end }
    }

    pub fn at(index: usize) -> Span {
        Span { start: index, end: index + 1 }
    }
}
