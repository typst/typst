//! Tokenized and syntax tree representations of source code.

use std::collections::HashMap;
use crate::func::Function;


/// A logical unit of the incoming text stream.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Token<'s> {
    /// One or more whitespace (non-newline) codepoints.
    Space,
    /// A line feed (either `\n` or `\r\n`).
    Newline,
    /// A left bracket: `[`.
    LeftBracket,
    /// A right bracket: `]`.
    RightBracket,
    /// A colon (`:`) indicating the beginning of function arguments.
    ///
    /// If a colon occurs outside of the function header, it will be
    /// tokenized as a [Word](Token::Word).
    Colon,
    /// Same as with [Colon](Token::Colon).
    Equals,
    /// Two underscores, indicating text in _italics_.
    DoubleUnderscore,
    /// Two stars, indicating **bold** text.
    DoubleStar,
    /// A dollar sign, indicating _mathematical_ content.
    Dollar,
    /// A hashtag starting a _comment_.
    Hashtag,
    /// Everything else just is a literal word.
    Word(&'s str),
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
    /// Indicates that italics were enabled/disabled.
    ToggleItalics,
    /// Indicates that boldface was enabled/disabled.
    ToggleBold,
    /// Indicates that math mode was enabled/disabled.
    ToggleMath,
    /// A literal word.
    Word(String),
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

/// A potentially unevaluated expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {}
