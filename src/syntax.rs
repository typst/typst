//! Tokenized and syntax tree representations of source code.

use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};
use std::ops::Deref;

use crate::func::Function;
use crate::utility::StrExt;


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

/// A complete function invocation consisting of header and body.
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
    pub name: Ident,
    pub args: Vec<Expression>,
    pub kwargs: HashMap<Ident, Expression>
}

/// A potentially unevaluated expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {}

/// An owned valid identifier.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Ident(String);

impl Ident {
    /// Create a new identifier if the string is a valid one.
    #[inline]
    pub fn new<S: Into<String>>(ident: S) -> Option<Ident> {
        let ident = ident.into();
        if ident.is_identifier() {
            Some(Ident(ident))
        } else {
            None
        }
    }

    /// Consume self and return the underlying string.
    #[inline]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Display for Ident {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Deref for Ident {
    type Target = str;

    #[inline]
    fn deref(&self) -> &str {
        &*self.0
    }
}
