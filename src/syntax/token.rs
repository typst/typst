use crate::util::EcoString;

/// A quoted string token: `"..."`.
#[derive(Debug, Clone, PartialEq)]
pub struct StrToken {
    /// The string inside the quotes.
    ///
    /// _Note_: If the string contains escape sequences these are not yet
    /// applied to be able to just store a string slice here instead of
    /// a `String`. The resolving is done later in the parser.
    pub string: EcoString,
    /// Whether the closing quote was present.
    pub terminated: bool,
}

/// A raw block token: `` `...` ``.
#[derive(Debug, Clone, PartialEq)]
pub struct RawToken {
    /// The raw text in the block.
    pub text: EcoString,
    /// The programming language of the raw text.
    pub lang: Option<EcoString>,
    /// The number of opening backticks.
    pub backticks: u8,
    /// Whether all closing backticks were present.
    pub terminated: bool,
    /// Whether to display this as a block.
    pub block: bool,
}

/// A math formula token: `$2pi + x$` or `$[f'(x) = x^2]$`.
#[derive(Debug, Clone, PartialEq)]
pub struct MathToken {
    /// The formula between the dollars.
    pub formula: EcoString,
    /// Whether the formula is display-level, that is, it is surrounded by
    /// `$[..]`.
    pub display: bool,
    /// Whether the closing dollars were present.
    pub terminated: bool,
}

/// A unicode escape sequence token: `\u{1F5FA}`.
#[derive(Debug, Clone, PartialEq)]
pub struct UnicodeEscapeToken {
    /// The escape sequence between the braces.
    pub sequence: EcoString,
    /// The resulting unicode character.
    pub character: Option<char>,
    /// Whether the closing brace was present.
    pub terminated: bool,
}

/// A unit-bound number token: `1.2em`.
#[derive(Debug, Clone, PartialEq)]
pub struct UnitToken {
    /// The number part.
    pub number: std::ops::Range<usize>,
    /// The unit part.
    pub unit: std::ops::Range<usize>,
}
