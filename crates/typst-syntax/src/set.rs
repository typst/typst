// Acknowledgement:
// Based on rust-analyzer's `TokenSet`.
// https://github.com/rust-lang/rust-analyzer/blob/master/crates/parser/src/token_set.rs

use crate::SyntaxKind;

/// A set of syntax kinds.
#[derive(Default, Copy, Clone)]
pub struct SyntaxSet(u128);

impl SyntaxSet {
    /// Create a new set from a slice of kinds.
    pub const fn new(slice: &[SyntaxKind]) -> Self {
        let mut bits = 0;
        let mut i = 0;
        while i < slice.len() {
            bits |= bit(slice[i]);
            i += 1;
        }
        Self(bits)
    }

    /// Insert a syntax kind into the set.
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Whether the set contains the given syntax kind.
    pub const fn contains(&self, kind: SyntaxKind) -> bool {
        (self.0 & bit(kind)) != 0
    }
}

const fn bit(kind: SyntaxKind) -> u128 {
    1 << (kind as usize)
}

/// Syntax kinds that can start a statement.
pub const STMT: SyntaxSet = SyntaxSet::new(&[
    SyntaxKind::Let,
    SyntaxKind::Set,
    SyntaxKind::Show,
    SyntaxKind::Import,
    SyntaxKind::Include,
    SyntaxKind::Return,
]);

/// Syntax kinds that can start a markup expression.
pub const MARKUP_EXPR: SyntaxSet = SyntaxSet::new(&[
    SyntaxKind::Space,
    SyntaxKind::Parbreak,
    SyntaxKind::LineComment,
    SyntaxKind::BlockComment,
    SyntaxKind::Text,
    SyntaxKind::Linebreak,
    SyntaxKind::Escape,
    SyntaxKind::Shorthand,
    SyntaxKind::SmartQuote,
    SyntaxKind::Raw,
    SyntaxKind::Link,
    SyntaxKind::Label,
    SyntaxKind::Hash,
    SyntaxKind::Star,
    SyntaxKind::Underscore,
    SyntaxKind::HeadingMarker,
    SyntaxKind::ListMarker,
    SyntaxKind::EnumMarker,
    SyntaxKind::TermMarker,
    SyntaxKind::RefMarker,
    SyntaxKind::Dollar,
    SyntaxKind::LeftBracket,
    SyntaxKind::RightBracket,
    SyntaxKind::Colon,
]);

/// Syntax kinds that can start a math expression.
pub const MATH_EXPR: SyntaxSet = SyntaxSet::new(&[
    SyntaxKind::Hash,
    SyntaxKind::MathIdent,
    SyntaxKind::Text,
    SyntaxKind::Shorthand,
    SyntaxKind::Linebreak,
    SyntaxKind::MathAlignPoint,
    SyntaxKind::Escape,
    SyntaxKind::Str,
    SyntaxKind::Root,
    SyntaxKind::Prime,
]);

/// Syntax kinds that can start a code expression.
pub const CODE_EXPR: SyntaxSet = CODE_PRIMARY.union(UNARY_OP);

/// Syntax kinds that can start an atomic code expression.
pub const ATOMIC_CODE_EXPR: SyntaxSet = ATOMIC_CODE_PRIMARY;

/// Syntax kinds that can start a code primary.
pub const CODE_PRIMARY: SyntaxSet =
    ATOMIC_CODE_PRIMARY.union(SyntaxSet::new(&[SyntaxKind::Underscore]));

/// Syntax kinds that can start an atomic code primary.
pub const ATOMIC_CODE_PRIMARY: SyntaxSet = SyntaxSet::new(&[
    SyntaxKind::Ident,
    SyntaxKind::LeftBrace,
    SyntaxKind::LeftBracket,
    SyntaxKind::LeftParen,
    SyntaxKind::Dollar,
    SyntaxKind::Let,
    SyntaxKind::Set,
    SyntaxKind::Show,
    SyntaxKind::If,
    SyntaxKind::While,
    SyntaxKind::For,
    SyntaxKind::Import,
    SyntaxKind::Include,
    SyntaxKind::Break,
    SyntaxKind::Continue,
    SyntaxKind::Return,
    SyntaxKind::None,
    SyntaxKind::Auto,
    SyntaxKind::Int,
    SyntaxKind::Float,
    SyntaxKind::Bool,
    SyntaxKind::Numeric,
    SyntaxKind::Str,
    SyntaxKind::Label,
    SyntaxKind::Raw,
]);

/// Syntax kinds that are unary operators.
pub const UNARY_OP: SyntaxSet =
    SyntaxSet::new(&[SyntaxKind::Plus, SyntaxKind::Minus, SyntaxKind::Not]);

/// Syntax kinds that are binary operators.
pub const BINARY_OP: SyntaxSet = SyntaxSet::new(&[
    SyntaxKind::Plus,
    SyntaxKind::Minus,
    SyntaxKind::Star,
    SyntaxKind::Slash,
    SyntaxKind::And,
    SyntaxKind::Or,
    SyntaxKind::EqEq,
    SyntaxKind::ExclEq,
    SyntaxKind::Lt,
    SyntaxKind::LtEq,
    SyntaxKind::Gt,
    SyntaxKind::GtEq,
    SyntaxKind::Eq,
    SyntaxKind::In,
    SyntaxKind::PlusEq,
    SyntaxKind::HyphEq,
    SyntaxKind::StarEq,
    SyntaxKind::SlashEq,
]);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_size() {
        assert!((SyntaxKind::Eof as usize) < 128);
    }

    #[test]
    fn test_set() {
        let set = SyntaxSet::new(&[SyntaxKind::And, SyntaxKind::Or]);
        assert!(set.contains(SyntaxKind::And));
        assert!(set.contains(SyntaxKind::Or));
        assert!(!set.contains(SyntaxKind::Not));
    }
}
