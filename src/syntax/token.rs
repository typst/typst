//! Token definition.

use crate::length::Length;

/// A minimal semantic entity of source code.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Token<'s> {
    /// One or more whitespace characters.
    ///
    /// The contained `usize` denotes the number of newlines that were contained
    /// in the whitespace.
    Space(usize),
    /// A consecutive non-markup string.
    Text(&'s str),

    /// A line comment with inner string contents `//<str>\n`.
    LineComment(&'s str),
    /// A block comment with inner string contents `/*<str>*/`.
    ///
    /// The comment can contain nested block comments.
    BlockComment(&'s str),

    /// A star: `*`.
    Star,
    /// An underscore: `_`.
    Underscore,
    /// A backslash followed by whitespace: `\`.
    Backslash,
    /// A hashtag indicating a section heading: `#`.
    Hashtag,
    /// A non-breaking space: `~`.
    NonBreakingSpace,
    /// A raw block: `` `...` ``.
    Raw(TokenRaw<'s>),
    /// A unicode escape sequence: `\u{1F5FA}`.
    UnicodeEscape(TokenUnicodeEscape<'s>),

    /// A left bracket: `[`.
    LeftBracket,
    /// A right bracket: `]`.
    RightBracket,
    /// A left brace: `{`.
    LeftBrace,
    /// A right brace: `}`.
    RightBrace,
    /// A left parenthesis: `(`.
    LeftParen,
    /// A right parenthesis: `)`.
    RightParen,

    /// A colon: `:`.
    Colon,
    /// A comma: `,`.
    Comma,
    /// An equals sign: `=`.
    Equals,
    /// A double forward chevron: `>>`.
    Chain,
    /// A plus: `+`.
    Plus,
    /// A hyphen: `-`.
    Hyphen,
    /// A slash: `/`.
    Slash,

    /// An identifier: `center`.
    Ident(&'s str),
    /// A boolean: `true`, `false`.
    Bool(bool),
    /// An integer: `120`.
    Int(i64),
    /// A floating-point number: `1.2`, `10e-4`.
    Float(f64),
    /// A length: `12pt`, `3cm`.
    Length(Length),
    /// A percentage: `50%`.
    ///
    /// Note: `50%` is represented as `50.0` here, as in the corresponding
    /// [literal].
    ///
    /// [literal]: ../ast/enum.Lit.html#variant.Percent
    Percent(f64),
    /// A hex value: `#20d82a`.
    Hex(&'s str),
    /// A quoted string: `"..."`.
    Str(TokenStr<'s>),

    /// Things that are not valid in the context they appeared in.
    Invalid(&'s str),
}

/// A quoted string: `"..."`.
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

/// A unicode escape sequence: `\u{1F5FA}`.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct TokenUnicodeEscape<'s> {
    /// The escape sequence between two braces.
    pub sequence: &'s str,
    /// Whether the closing brace was present.
    pub terminated: bool,
}

/// A raw block: `` `...` ``.
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
    /// The natural-language name of this token for use in error messages.
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
            Self::NonBreakingSpace => "non-breaking space",
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
            Self::Int(_) => "integer",
            Self::Float(_) => "float",
            Self::Length(_) => "length",
            Self::Percent(_) => "percentage",
            Self::Hex(_) => "hex value",
            Self::Str { .. } => "string",

            Self::Invalid("*/") => "end of block comment",
            Self::Invalid(_) => "invalid token",
        }
    }
}
