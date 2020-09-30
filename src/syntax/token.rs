//! Tokenization.

use crate::length::Length;

/// A minimal semantic entity of source code.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Token<'s> {
    /// One or more whitespace characters. The contained `usize` denotes the
    /// number of newlines that were contained in the whitespace.
    Space(usize),

    /// A line comment with inner string contents `//<str>\n`.
    LineComment(&'s str),
    /// A block comment with inner string contents `/*<str>*/`. The comment
    /// can contain nested block comments.
    BlockComment(&'s str),

    /// A left bracket starting a function invocation or body: `[`.
    LeftBracket,
    /// A right bracket ending a function invocation or body: `]`.
    RightBracket,
    /// A left parenthesis in a function header: `(`.
    LeftParen,
    /// A right parenthesis in a function header: `)`.
    RightParen,
    /// A left brace in a function header: `{`.
    LeftBrace,
    /// A right brace in a function header: `}`.
    RightBrace,
    /// A double forward chevron in a function header: `>>`.
    Chain,

    /// A colon in a function header: `:`.
    Colon,
    /// A comma in a function header: `,`.
    Comma,
    /// An equals sign in a function header: `=`.
    Equals,

    /// An identifier in a function header: `center`.
    Ident(&'s str),
    /// A quoted string in a function header: `"..."`.
    Str {
        /// The string inside the quotes.
        ///
        /// _Note_: If the string contains escape sequences these are not yet
        /// applied to be able to just store a string slice here instead of
        /// a String. The escaping is done later in the parser.
        string: &'s str,
        /// Whether the closing quote was present.
        terminated: bool,
    },
    /// A boolean in a function header: `true | false`.
    Bool(bool),
    /// A number in a function header: `3.14`.
    Number(f64),
    /// A length in a function header: `12pt`.
    Length(Length),
    /// A hex value in a function header: `#20d82a`.
    Hex(&'s str),
    /// A plus in a function header, signifying the addition of expressions.
    Plus,
    /// A hyphen in a function header, signifying the subtraction of
    /// expressions.
    Hyphen,
    /// A slash in a function header, signifying the division of expressions.
    Slash,

    /// A star. It can appear in a function header where it signifies the
    /// multiplication of expressions or the body where it modifies the styling.
    Star,
    /// An underscore in body-text.
    Underscore,
    /// A backslash followed by whitespace in text.
    Backslash,

    /// A hashtag token in the body can indicate compute mode or headings.
    Hashtag,

    /// A unicode escape sequence.
    UnicodeEscape {
        /// The escape sequence between two braces.
        sequence: &'s str,
        /// Whether the closing brace was present.
        terminated: bool,
    },

    /// Raw block.
    Raw {
        /// The raw text between the backticks.
        raw: &'s str,
        /// The number of opening backticks.
        backticks: usize,
        /// Whether all closing backticks were present.
        terminated: bool,
    },

    /// Any other consecutive string.
    Text(&'s str),

    /// Things that are not valid in the context they appeared in.
    Invalid(&'s str),
}

impl<'s> Token<'s> {
    /// The natural-language name for this token for use in error messages.
    pub fn name(self) -> &'static str {
        match self {
            Self::Space(_) => "space",
            Self::LineComment(_) => "line comment",
            Self::BlockComment(_) => "block comment",
            Self::LeftBracket => "opening bracket",
            Self::RightBracket => "closing bracket",
            Self::LeftParen => "opening paren",
            Self::RightParen => "closing paren",
            Self::LeftBrace => "opening brace",
            Self::RightBrace => "closing brace",
            Self::Chain => "function chain operator",
            Self::Colon => "colon",
            Self::Comma => "comma",
            Self::Equals => "equals sign",
            Self::Ident(_) => "identifier",
            Self::Str { .. } => "string",
            Self::Bool(_) => "bool",
            Self::Number(_) => "number",
            Self::Length(_) => "length",
            Self::Hex(_) => "hex value",
            Self::Plus => "plus",
            Self::Hyphen => "minus",
            Self::Slash => "slash",
            Self::Star => "star",
            Self::Underscore => "underscore",
            Self::Backslash => "backslash",
            Self::Hashtag => "hashtag",
            Self::UnicodeEscape { .. } => "unicode escape sequence",
            Self::Raw { .. } => "raw block",
            Self::Text(_) => "text",
            Self::Invalid("*/") => "end of block comment",
            Self::Invalid(_) => "invalid token",
        }
    }
}
