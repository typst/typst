use crate::geom::{AngularUnit, LengthUnit};

/// A minimal semantic entity of source code.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Token<'s> {
    /// A left square bracket: `[`.
    LeftBracket,
    /// A right square bracket: `]`.
    RightBracket,
    /// A left curly brace: `{`.
    LeftBrace,
    /// A right curly brace: `}`.
    RightBrace,
    /// A left round parenthesis: `(`.
    LeftParen,
    /// A right round parenthesis: `)`.
    RightParen,
    /// An asterisk: `*`.
    Star,
    /// An underscore: `_`.
    Underscore,
    /// A hashtag: `#`.
    Hash,
    /// A tilde: `~`.
    Tilde,
    /// A backslash followed by nothing or whitespace: `\`.
    Backslash,
    /// A comma: `,`.
    Comma,
    /// A colon: `:`.
    Colon,
    /// A pipe: `|`.
    Pipe,
    /// A plus: `+`.
    Plus,
    /// A hyphen: `-`.
    Hyph,
    /// A slash: `/`.
    Slash,
    /// A single equals sign: `=`.
    Eq,
    /// Two equals signs: `==`.
    EqEq,
    /// An exclamation mark followed by an equals sign: `!=`.
    BangEq,
    /// A less-than sign: `<`.
    Lt,
    /// A less-than sign followed by an equals sign: `<=`.
    LtEq,
    /// A greater-than sign: `>`.
    Gt,
    /// A greater-than sign followed by an equals sign: `>=`.
    GtEq,
    /// A plus followed by an equals sign: `+=`.
    PlusEq,
    /// A hyphen followed by an equals sign: `-=`.
    HyphEq,
    /// An asterisk followed by an equals sign: `*=`.
    StarEq,
    /// A slash followed by an equals sign: `/=`.
    SlashEq,
    /// A question mark: `?`.
    Question,
    /// Two dots: `..`.
    Dots,
    /// An equals sign followed by a greater-than sign: `=>`.
    Arrow,
    /// The `not` operator.
    Not,
    /// The `and` operator.
    And,
    /// The `or` operator.
    Or,
    /// The `let` / `#let` keyword.
    Let,
    /// The `if` / `#if` keyword.
    If,
    /// The `else` / `#else` keyword.
    Else,
    /// The `for` / `#for` keyword.
    For,
    /// The `in` / `#in` keyword.
    In,
    /// The `while` / `#while` keyword.
    While,
    /// The `break` / `#break` keyword.
    Break,
    /// The `continue` / `#continue` keyword.
    Continue,
    /// The `return` / `#return` keyword.
    Return,
    /// The none literal: `none`.
    None,
    /// One or more whitespace characters.
    ///
    /// The contained `usize` denotes the number of newlines that were contained
    /// in the whitespace.
    Space(usize),
    /// A consecutive non-markup string.
    Text(&'s str),
    /// An arbitrary number of backticks followed by inner contents, terminated
    /// with the same number of backticks: `` `...` ``.
    Raw(TokenRaw<'s>),
    /// One or two dollar signs followed by inner contents, terminated with the
    /// same number of dollar signs.
    Math(TokenMath<'s>),
    /// A slash and the letter "u" followed by a hexadecimal unicode entity
    /// enclosed in curly braces: `\u{1F5FA}`.
    UnicodeEscape(TokenUnicodeEscape<'s>),
    /// An identifier: `center`.
    Ident(&'s str),
    /// A boolean: `true`, `false`.
    Bool(bool),
    /// An integer: `120`.
    Int(i64),
    /// A floating-point number: `1.2`, `10e-4`.
    Float(f64),
    /// A length: `12pt`, `3cm`.
    Length(f64, LengthUnit),
    /// An angle: `90deg`.
    Angle(f64, AngularUnit),
    /// A percentage: `50%`.
    ///
    /// _Note_: `50%` is stored as `50.0` here, as in the corresponding
    /// [literal](super::Expr::Percent).
    Percent(f64),
    /// A hex value: `#20d82a`.
    Hex(&'s str),
    /// A quoted string: `"..."`.
    Str(TokenStr<'s>),
    /// Two slashes followed by inner contents, terminated with a newline:
    /// `//<str>\n`.
    LineComment(&'s str),
    /// A slash and a star followed by inner contents,  terminated with a star
    /// and a slash: `/*<str>*/`.
    ///
    /// The comment can contain nested block comments.
    BlockComment(&'s str),
    /// Things that are not valid tokens.
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

/// A math formula: `$2pi + x$`, `$$f'(x) = x^2$$`.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct TokenMath<'s> {
    /// The formula between the dollars.
    pub formula: &'s str,
    /// Whether the formula was surrounded by one dollar (true) or two dollars
    /// (false).
    pub inline: bool,
    /// Whether the closing dollars were present.
    pub terminated: bool,
}

/// A unicode escape sequence: `\u{1F5FA}`.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct TokenUnicodeEscape<'s> {
    /// The escape sequence between the braces.
    pub sequence: &'s str,
    /// Whether the closing brace was present.
    pub terminated: bool,
}

impl<'s> Token<'s> {
    /// The English name of this token for use in error messages.
    pub fn name(self) -> &'static str {
        match self {
            Self::LeftBracket => "opening bracket",
            Self::RightBracket => "closing bracket",
            Self::LeftBrace => "opening brace",
            Self::RightBrace => "closing brace",
            Self::LeftParen => "opening paren",
            Self::RightParen => "closing paren",
            Self::Star => "star",
            Self::Underscore => "underscore",
            Self::Hash => "hashtag",
            Self::Tilde => "tilde",
            Self::Backslash => "backslash",
            Self::Comma => "comma",
            Self::Colon => "colon",
            Self::Pipe => "pipe",
            Self::Plus => "plus",
            Self::Hyph => "minus",
            Self::Slash => "slash",
            Self::Eq => "assignment operator",
            Self::EqEq => "equality operator",
            Self::BangEq => "inequality operator",
            Self::Lt => "less than operator",
            Self::LtEq => "less than or equal operator",
            Self::Gt => "greater than operator",
            Self::GtEq => "greater than or equal operator",
            Self::PlusEq => "add-assign operator",
            Self::HyphEq => "subtract-assign operator",
            Self::StarEq => "multiply-assign operator",
            Self::SlashEq => "divide-assign operator",
            Self::Question => "question mark",
            Self::Dots => "dots",
            Self::Arrow => "arrow",
            Self::Not => "not operator",
            Self::And => "and operator",
            Self::Or => "or operator",
            Self::Let => "let keyword",
            Self::If => "if keyword",
            Self::Else => "else keyword",
            Self::For => "for keyword",
            Self::In => "in keyword",
            Self::While => "while keyword",
            Self::Break => "break keyword",
            Self::Continue => "continue keyword",
            Self::Return => "return keyword",
            Self::None => "none",
            Self::Space(_) => "space",
            Self::Text(_) => "text",
            Self::Raw(_) => "raw block",
            Self::Math(_) => "math formula",
            Self::UnicodeEscape(_) => "unicode escape sequence",
            Self::Ident(_) => "identifier",
            Self::Bool(_) => "boolean",
            Self::Int(_) => "integer",
            Self::Float(_) => "float",
            Self::Length(..) => "length",
            Self::Angle(..) => "angle",
            Self::Percent(_) => "percentage",
            Self::Hex(_) => "hex value",
            Self::Str(_) => "string",
            Self::LineComment(_) => "line comment",
            Self::BlockComment(_) => "block comment",
            Self::Invalid("*/") => "end of block comment",
            Self::Invalid(_) => "invalid token",
        }
    }
}
