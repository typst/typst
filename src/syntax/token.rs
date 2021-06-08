use crate::color::RgbaColor;
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
    /// A single hashtag: `#`.
    Hashtag,
    /// A tilde: `~`.
    Tilde,
    /// Two hyphens: `--`.
    HyphHyph,
    /// Three hyphens: `---`.
    HyphHyphHyph,
    /// A backslash followed by nothing or whitespace: `\`.
    Backslash,
    /// A comma: `,`.
    Comma,
    /// A semicolon: `;`.
    Semicolon,
    /// A colon: `:`.
    Colon,
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
    /// The none literal: `none`.
    None,
    /// The `let` keyword.
    Let,
    /// The `if` keyword.
    If,
    /// The `else` keyword.
    Else,
    /// The `for` keyword.
    For,
    /// The `in` keyword.
    In,
    /// The `while` keyword.
    While,
    /// The `break` keyword.
    Break,
    /// The `continue` keyword.
    Continue,
    /// The `return` keyword.
    Return,
    /// The `import` keyword.
    Import,
    /// The `include` keyword.
    Include,
    /// The `using` keyword.
    Using,
    /// One or more whitespace characters.
    ///
    /// The contained `usize` denotes the number of newlines that were contained
    /// in the whitespace.
    Space(usize),
    /// A consecutive non-markup string.
    Text(&'s str),
    /// A slash and the letter "u" followed by a hexadecimal unicode entity
    /// enclosed in curly braces: `\u{1F5FA}`.
    UnicodeEscape(UnicodeEscapeToken<'s>),
    /// An arbitrary number of backticks followed by inner contents, terminated
    /// with the same number of backticks: `` `...` ``.
    Raw(RawToken<'s>),
    /// One or two dollar signs followed by inner contents, terminated with the
    /// same number of dollar signs.
    Math(MathToken<'s>),
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
    /// A color value: `#20d82a`.
    Color(RgbaColor),
    /// A quoted string: `"..."`.
    Str(StrToken<'s>),
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

/// A quoted string token: `"..."`.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct StrToken<'s> {
    /// The string inside the quotes.
    ///
    /// _Note_: If the string contains escape sequences these are not yet
    /// applied to be able to just store a string slice here instead of
    /// a `String`. The resolving is done later in the parser.
    pub string: &'s str,
    /// Whether the closing quote was present.
    pub terminated: bool,
}

/// A raw block token: `` `...` ``.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct RawToken<'s> {
    /// The raw text between the backticks.
    pub text: &'s str,
    /// The number of opening backticks.
    pub backticks: usize,
    /// Whether all closing backticks were present.
    pub terminated: bool,
}

/// A math formula token: `$2pi + x$` or `$[f'(x) = x^2]$`.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct MathToken<'s> {
    /// The formula between the dollars.
    pub formula: &'s str,
    /// Whether the formula is display-level, that is, it is surrounded by
    /// `$[..]`.
    pub display: bool,
    /// Whether the closing dollars were present.
    pub terminated: bool,
}

/// A unicode escape sequence token: `\u{1F5FA}`.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct UnicodeEscapeToken<'s> {
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
            Self::Hashtag => "hashtag",
            Self::Tilde => "tilde",
            Self::HyphHyph => "en dash",
            Self::HyphHyphHyph => "em dash",
            Self::Backslash => "backslash",
            Self::Comma => "comma",
            Self::Semicolon => "semicolon",
            Self::Colon => "colon",
            Self::Plus => "plus",
            Self::Hyph => "minus",
            Self::Slash => "slash",
            Self::Eq => "assignment operator",
            Self::EqEq => "equality operator",
            Self::BangEq => "inequality operator",
            Self::Lt => "less-than operator",
            Self::LtEq => "less-than or equal operator",
            Self::Gt => "greater-than operator",
            Self::GtEq => "greater-than or equal operator",
            Self::PlusEq => "add-assign operator",
            Self::HyphEq => "subtract-assign operator",
            Self::StarEq => "multiply-assign operator",
            Self::SlashEq => "divide-assign operator",
            Self::Dots => "dots",
            Self::Arrow => "arrow",
            Self::Not => "operator `not`",
            Self::And => "operator `and`",
            Self::Or => "operator `or`",
            Self::None => "`none`",
            Self::Let => "keyword `let`",
            Self::If => "keyword `if`",
            Self::Else => "keyword `else`",
            Self::For => "keyword `for`",
            Self::In => "keyword `in`",
            Self::While => "keyword `while`",
            Self::Break => "keyword `break`",
            Self::Continue => "keyword `continue`",
            Self::Return => "keyword `return`",
            Self::Import => "keyword `import`",
            Self::Include => "keyword `include`",
            Self::Using => "keyword `using`",
            Self::Space(_) => "space",
            Self::Text(_) => "text",
            Self::UnicodeEscape(_) => "unicode escape sequence",
            Self::Raw(_) => "raw block",
            Self::Math(_) => "math formula",
            Self::Ident(_) => "identifier",
            Self::Bool(_) => "boolean",
            Self::Int(_) => "integer",
            Self::Float(_) => "float",
            Self::Length(_, _) => "length",
            Self::Angle(_, _) => "angle",
            Self::Percent(_) => "percentage",
            Self::Color(_) => "color",
            Self::Str(_) => "string",
            Self::LineComment(_) => "line comment",
            Self::BlockComment(_) => "block comment",
            Self::Invalid("*/") => "end of block comment",
            Self::Invalid(_) => "invalid token",
        }
    }
}
