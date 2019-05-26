//! Tokenized and syntax tree representations of source code.

use std::collections::HashMap;
use crate::func::Function;


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
    /// A colon (`:`) indicating the beginning of function arguments.
    ///
    /// If a colon occurs outside of a function header, it will be tokenized as a
    /// [Word](Token::Word).
    Colon,
    /// An equals (`=`) sign assigning a function argument a value.
    ///
    /// Outside of functions headers, same as with [Colon](Token::Colon).
    Equals,
    /// Two underscores, indicating text in italics.
    DoubleUnderscore,
    /// Two stars, indicating bold text.
    DoubleStar,
    /// A dollar sign, indicating mathematical content.
    Dollar,
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
    /// Indicates that math mode was enabled / disabled.
    ToggleMath,
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
pub enum Expression {}
