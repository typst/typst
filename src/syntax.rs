//! Tokenized and syntax tree representations of source code.


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
#[derive(Debug, Clone, PartialEq)]
pub struct SyntaxTree<'s> {
    /// The children.
    pub nodes: Vec<Node<'s>>,
}

impl<'s> SyntaxTree<'s> {
    /// Create an empty syntax tree.
    #[inline]
    pub fn new() -> SyntaxTree<'s> {
        SyntaxTree { nodes: vec![] }
    }
}

/// A node in the abstract syntax tree.
#[derive(Debug, Clone, PartialEq)]
pub enum Node<'s> {
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
    Word(&'s str),
    /// A function invocation.
    Func(Function<'s>),
}

/// A node representing a function invocation.
#[derive(Debug, Clone, PartialEq)]
pub struct Function<'s> {
    /// The name of the function.
    pub name: &'s str,
    /// Some syntax tree if the function had a body (second set of brackets),
    /// otherwise nothing.
    pub body: Option<SyntaxTree<'s>>,
}
