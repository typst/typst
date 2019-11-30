//! Tokenization and parsing of source code.

use std::fmt::{self, Display, Formatter};

use crate::func::Function;
use crate::size::Size;

mod tokens;
#[macro_use]
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
    /// _Header only_.
    Colon,
    /// An equals (`=`) sign assigning a function argument a value (Header only).
    Equals,
    /// A comma (`,`) separating two function arguments (Header only).
    Comma,
    /// Quoted text as a string value (Header only).
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
    /// [BlockComment](Token::BlockComment) would be returned).
    StarSlash,
    /// Any consecutive string which does not contain markup.
    Text(&'s str),
}

/// A tree representation of source code.
#[derive(Debug, PartialEq)]
pub struct SyntaxTree {
    pub nodes: Vec<Spanned<Node>>,
}

impl SyntaxTree {
    /// Create an empty syntax tree.
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
    pub header: Spanned<FuncHeader>,
    pub body: Spanned<Box<dyn Function>>,
}

/// Contains header information of a function invocation.
#[derive(Debug, Clone, PartialEq)]
pub struct FuncHeader {
    pub name: Spanned<String>,
    pub args: FuncArgs,
}

/// The arguments passed to a function.
#[derive(Debug, Clone, PartialEq)]
pub struct FuncArgs {
    pub positional: Vec<Spanned<Expression>>,
    pub keyword: Vec<Spanned<(Spanned<String>, Spanned<Expression>)>>
}

impl FuncArgs {
    /// Create an empty collection of arguments.
    fn new() -> FuncArgs {
        FuncArgs {
            positional: vec![],
            keyword: vec![],
        }
    }
}

/// One argument passed to a function.
#[derive(Debug, Clone, PartialEq)]
pub enum FuncArg {
    Positional(Spanned<Expression>),
    Keyword(Spanned<(Spanned<String>, Spanned<Expression>)>),
}

/// An argument or return value.
#[derive(Clone, PartialEq)]
pub enum Expression {
    Ident(String),
    Str(String),
    Num(f64),
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
            Num(n) => write!(f, "{}", n),
            Size(s) => write!(f, "{}", s),
            Bool(b) => write!(f, "{}", b),
        }
    }
}

debug_display!(Expression);

/// Annotates a value with the part of the source code it corresponds to.
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Spanned<T> {
    pub val: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(val: T, span: Span) -> Spanned<T> {
        Spanned { val, span }
    }

    pub fn value(self) -> T {
        self.val
    }

    pub fn span_map<F, U>(self, f: F) -> Spanned<U> where F: FnOnce(T) -> U {
        Spanned::new(f(self.val), self.span)
    }
}

impl<T> Display for Spanned<T> where T: std::fmt::Debug {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "({:?}:{})", self.val, self.span)
    }
}

debug_display!(Spanned; T where T: std::fmt::Debug);

/// Describes a slice of source code.
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Span {
        Span { start, end }
    }

    pub fn merge(a: Span, b: Span) -> Span {
        Span {
            start: a.start.min(b.start),
            end: a.end.max(b.end),
        }
    }

    pub fn at(index: usize) -> Span {
        Span { start: index, end: index + 1 }
    }

    pub fn pair(&self) -> (usize, usize) {
        (self.start, self.end)
    }

    pub fn expand(&mut self, other: Span) {
        *self = Span::merge(*self, other)
    }
}

impl Display for Span {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "[{}, {}]", self.start, self.end)
    }
}

debug_display!(Span);
