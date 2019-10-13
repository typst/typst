//! Tokenized and syntax tree representations of source code.

use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};

use crate::func::Function;
use crate::size::Size;


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
    /// A colon (`:`) indicating the beginning of function arguments (Function header only).
    ///
    /// If a colon occurs outside of a function header, it will be tokenized as a
    /// [Word](Token::Word).
    Colon,
    /// An equals (`=`) sign assigning a function argument a value (Function header only).
    Equals,
    /// A comma (`,`) separating two function arguments (Function header only).
    Comma,
    /// Quoted text as a string value (Function header only).
    Quoted(&'s str),
    /// An underscore, indicating text in italics.
    Underscore,
    /// A star, indicating bold text.
    Star,
    /// A backtick, indicating monospace text.
    Backtick,
    /// A line comment.
    LineComment(&'s str),
    /// A block comment.
    BlockComment(&'s str),
    /// A star followed by a slash unexpectedly ending a block comment (the comment was not started
    /// before, otherwise a [BlockComment](Token::BlockComment) would be returned).
    StarSlash,
    /// Everything else is just text.
    Text(&'s str),
}

/// A tree representation of the source.
#[derive(Debug, PartialEq)]
pub struct SyntaxTree {
    /// The children.
    pub nodes: Vec<Node>,
}

impl SyntaxTree {
    /// Create an empty syntax tree.
    #[inline]
    pub fn new() -> SyntaxTree {
        SyntaxTree { nodes: vec![] }
    }
}

/// A node in the abstract syntax tree.
#[derive(Debug, PartialEq)]
pub enum Node {
    /// Whitespace between other nodes.
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

/// A function invocation consisting of header and body.
#[derive(Debug)]
pub struct FuncCall {
    pub header: FuncHeader,
    pub body: Box<dyn Function>,
}

impl PartialEq for FuncCall {
    fn eq(&self, other: &FuncCall) -> bool {
        (self.header == other.header) && (&self.body == &other.body)
    }
}

/// Contains header information of a function invocation.
#[derive(Debug, Clone, PartialEq)]
pub struct FuncHeader {
    pub name: String,
    pub args: Vec<Expression>,
    pub kwargs: HashMap<String, Expression>
}

/// A value expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Ident(String),
    Str(String),
    Number(f64),
    Size(Size),
    Bool(bool),
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
