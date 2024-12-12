// Acknowledgement:
// Based on rust-analyzer's `TokenSet`.
// https://github.com/rust-lang/rust-analyzer/blob/master/crates/parser/src/token_set.rs

use crate::SyntaxKind;

/// A set of syntax kinds.
#[derive(Default, Copy, Clone)]
pub struct SyntaxSet(u128);

impl SyntaxSet {
    /// Create a new set from a slice of kinds.
    pub const fn new() -> Self {
        Self(0)
    }

    /// Insert a syntax kind into the set.
    ///
    /// You can only add kinds with discriminator < 128.
    pub const fn add(self, kind: SyntaxKind) -> Self {
        assert!((kind as u8) < BITS);
        Self(self.0 | bit(kind))
    }

    /// Combine two syntax sets.
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Whether the set contains the given syntax kind.
    pub const fn contains(&self, kind: SyntaxKind) -> bool {
        (kind as u8) < BITS && (self.0 & bit(kind)) != 0
    }
}

const BITS: u8 = 128;

const fn bit(kind: SyntaxKind) -> u128 {
    1 << (kind as usize)
}

/// Generate a compile-time constant `SyntaxSet` of the given kinds.
macro_rules! syntax_set {
    ($($kind:ident),* $(,)?) => {{
        const SET: crate::set::SyntaxSet = crate::set::SyntaxSet::new()
            $(.add(crate::SyntaxKind:: $kind))*;
        SET
    }}
}

// Export so other modules can import as: `use set::syntax_set`
pub(crate) use syntax_set;

/// Syntax kinds that can start a statement.
pub const STMT: SyntaxSet = syntax_set!(Let, Set, Show, Import, Include, Return);

/// Syntax kinds that can start a math expression.
pub const MATH_EXPR: SyntaxSet = syntax_set!(
    Hash,
    MathIdent,
    FieldAccess,
    Comma,
    Semicolon,
    RightParen,
    Text,
    MathShorthand,
    Linebreak,
    MathAlignPoint,
    Escape,
    Str,
    Root,
    Prime,
);

/// Syntax kinds that can start a code expression.
pub const CODE_EXPR: SyntaxSet = CODE_PRIMARY.union(UNARY_OP);

/// Syntax kinds that can start an atomic code expression.
pub const ATOMIC_CODE_EXPR: SyntaxSet = ATOMIC_CODE_PRIMARY;

/// Syntax kinds that can start a code primary.
pub const CODE_PRIMARY: SyntaxSet = ATOMIC_CODE_PRIMARY.add(SyntaxKind::Underscore);

/// Syntax kinds that can start an atomic code primary.
pub const ATOMIC_CODE_PRIMARY: SyntaxSet = syntax_set!(
    Ident,
    LeftBrace,
    LeftBracket,
    LeftParen,
    Dollar,
    Let,
    Set,
    Show,
    Context,
    If,
    While,
    For,
    Import,
    Include,
    Break,
    Continue,
    Return,
    None,
    Auto,
    Int,
    Float,
    Bool,
    Numeric,
    Str,
    Label,
    Raw,
);

/// Syntax kinds that are unary operators.
pub const UNARY_OP: SyntaxSet = syntax_set!(Plus, Minus, Not);

/// Syntax kinds that are binary operators.
pub const BINARY_OP: SyntaxSet = syntax_set!(
    Plus, Minus, Star, Slash, And, Or, EqEq, ExclEq, Lt, LtEq, Gt, GtEq, Eq, In, PlusEq,
    HyphEq, StarEq, SlashEq,
);

/// Syntax kinds that can start an argument in a function call.
pub const ARRAY_OR_DICT_ITEM: SyntaxSet = CODE_EXPR.add(SyntaxKind::Dots);

/// Syntax kinds that can start an argument in a function call.
pub const ARG: SyntaxSet = CODE_EXPR.add(SyntaxKind::Dots);

/// Syntax kinds that can start a parameter in a parameter list.
pub const PARAM: SyntaxSet = PATTERN.add(SyntaxKind::Dots);

/// Syntax kinds that can start a destructuring item.
pub const DESTRUCTURING_ITEM: SyntaxSet = PATTERN.add(SyntaxKind::Dots);

/// Syntax kinds that can start a pattern.
pub const PATTERN: SyntaxSet =
    PATTERN_LEAF.add(SyntaxKind::LeftParen).add(SyntaxKind::Underscore);

/// Syntax kinds that can start a pattern leaf.
pub const PATTERN_LEAF: SyntaxSet = ATOMIC_CODE_EXPR;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set() {
        let set = SyntaxSet::new().add(SyntaxKind::And).add(SyntaxKind::Or);
        assert!(set.contains(SyntaxKind::And));
        assert!(set.contains(SyntaxKind::Or));
        assert!(!set.contains(SyntaxKind::Not));
    }
}
