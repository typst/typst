use crate::util::EcoString;

/// A quoted string token: `"..."`.
#[derive(Debug, Clone, PartialEq)]
#[repr(transparent)]
pub struct StrToken {
    /// The string inside the quotes.
    pub string: EcoString,
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
}

/// A unicode escape sequence token: `\u{1F5FA}`.
#[derive(Debug, Clone, PartialEq)]
#[repr(transparent)]
pub struct UnicodeEscapeToken {
    /// The resulting unicode character.
    pub character: char,
}
