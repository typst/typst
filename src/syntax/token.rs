//! Token definition.

use crate::length::Length;

/// A minimal semantic entity of source code.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Token<'s> {
    /// One or more whitespace characters. The contained `usize` denotes the
    /// number of newlines that were contained in the whitespace.
    Space(usize),
    /// A consecutive non-markup string.
    Text(&'s str),

    /// A line comment with inner string contents `//<str>\n`.
    LineComment(&'s str),
    /// A block comment with inner string contents `/*<str>*/`. The comment
    /// can contain nested block comments.
    BlockComment(&'s str),

    /// A star. It can appear in a function header where it signifies the
    /// multiplication of expressions or the body where it modifies the styling.
    Star,
    /// An underscore in body-text.
    Underscore,
    /// A backslash followed by whitespace in text.
    Backslash,
    /// A hashtag indicating a section heading.
    Hashtag,
    /// A raw block.
    Raw(TokenRaw<'s>),
    /// A unicode escape sequence.
    UnicodeEscape(TokenUnicodeEscape<'s>),

    /// A left bracket starting a function invocation or body: `[`.
    LeftBracket,
    /// A right bracket ending a function invocation or body: `]`.
    RightBracket,
    /// A left brace indicating the start of content: `{`.
    LeftBrace,
    /// A right brace indicating the end of content: `}`.
    RightBrace,
    /// A left parenthesis in a function header: `(`.
    LeftParen,
    /// A right parenthesis in a function header: `)`.
    RightParen,

    /// A colon in a function header: `:`.
    Colon,
    /// A comma in a function header: `,`.
    Comma,
    /// An equals sign in a function header: `=`.
    Equals,
    /// A double forward chevron in a function header: `>>`.
    Chain,
    /// A plus in a function header, signifying the addition of expressions.
    Plus,
    /// A hyphen in a function header, signifying the subtraction of
    /// expressions.
    Hyphen,
    /// A slash in a function header, signifying the division of expressions.
    Slash,

    /// An identifier in a function header: `center`.
    Ident(&'s str),
    /// A boolean in a function header: `true | false`.
    Bool(bool),
    /// A number in a function header: `3.14`.
    Number(f64),
    /// A length in a function header: `12pt`.
    Length(Length),
    /// A hex value in a function header: `#20d82a`.
    Hex(&'s str),
    /// A quoted string in a function header: `"..."`.
    Str(TokenStr<'s>),

    /// Things that are not valid in the context they appeared in.
    Invalid(&'s str),
}

/// A quoted string in a function header: `"..."`.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct TokenStr<'s> {
    /// The string inside the quotes.
    ///
    /// _Note_: If the string contains escape sequences these are not yet
    /// applied to be able to just store a string slice here instead of
    /// a `String`. The resolving is done later in the parser.
    pub string: &'s str,
    /// Whether the closing quote was present.
    pub terminated: bool,
}

/// A unicode escape sequence.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct TokenUnicodeEscape<'s> {
    /// The escape sequence between two braces.
    pub sequence: &'s str,
    /// Whether the closing brace was present.
    pub terminated: bool,
}

/// A raw block.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct TokenRaw<'s> {
    /// The raw text between the backticks.
    pub text: &'s str,
    /// The number of opening backticks.
    pub backticks: usize,
    /// Whether all closing backticks were present.
    pub terminated: bool,
}

impl<'s> Token<'s> {
    /// The natural-language name for this token for use in error messages.
    pub fn name(self) -> &'static str {
        match self {
            Self::Space(_) => "space",
            Self::Text(_) => "text",

            Self::LineComment(_) => "line comment",
            Self::BlockComment(_) => "block comment",

            Self::Star => "star",
            Self::Underscore => "underscore",
            Self::Backslash => "backslash",
            Self::Hashtag => "hashtag",
            Self::Raw { .. } => "raw block",
            Self::UnicodeEscape { .. } => "unicode escape sequence",

            Self::LeftBracket => "opening bracket",
            Self::RightBracket => "closing bracket",
            Self::LeftBrace => "opening brace",
            Self::RightBrace => "closing brace",
            Self::LeftParen => "opening paren",
            Self::RightParen => "closing paren",

            Self::Colon => "colon",
            Self::Comma => "comma",
            Self::Equals => "equals sign",
            Self::Chain => "function chaining operator",
            Self::Plus => "plus sign",
            Self::Hyphen => "minus sign",
            Self::Slash => "slash",

            Self::Ident(_) => "identifier",
            Self::Bool(_) => "bool",
            Self::Number(_) => "number",
            Self::Length(_) => "length",
            Self::Hex(_) => "hex value",
            Self::Str { .. } => "string",

            Self::Invalid("*/") => "end of block comment",
            Self::Invalid(_) => "invalid token",
        }
    }
}
