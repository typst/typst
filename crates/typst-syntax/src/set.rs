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

/// Syntax kinds that can start a statement.
pub const STMT: SyntaxSet = SyntaxSet::new()
    .add(SyntaxKind::Let)
    .add(SyntaxKind::Set)
    .add(SyntaxKind::Show)
    .add(SyntaxKind::Import)
    .add(SyntaxKind::Include)
    .add(SyntaxKind::Return);

/// Syntax kinds that can start a markup expression.
pub const MARKUP_EXPR: SyntaxSet = SyntaxSet::new()
    .add(SyntaxKind::Space)
    .add(SyntaxKind::Parbreak)
    .add(SyntaxKind::LineComment)
    .add(SyntaxKind::BlockComment)
    .add(SyntaxKind::Text)
    .add(SyntaxKind::Linebreak)
    .add(SyntaxKind::Escape)
    .add(SyntaxKind::Shorthand)
    .add(SyntaxKind::SmartQuote)
    .add(SyntaxKind::RawDelim)
    .add(SyntaxKind::Link)
    .add(SyntaxKind::Label)
    .add(SyntaxKind::Hash)
    .add(SyntaxKind::Star)
    .add(SyntaxKind::Underscore)
    .add(SyntaxKind::HeadingMarker)
    .add(SyntaxKind::ListMarker)
    .add(SyntaxKind::EnumMarker)
    .add(SyntaxKind::TermMarker)
    .add(SyntaxKind::RefMarker)
    .add(SyntaxKind::Dollar)
    .add(SyntaxKind::LeftBracket)
    .add(SyntaxKind::RightBracket)
    .add(SyntaxKind::Colon);

/// Syntax kinds that can start a math expression.
pub const MATH_EXPR: SyntaxSet = SyntaxSet::new()
    .add(SyntaxKind::Hash)
    .add(SyntaxKind::MathIdent)
    .add(SyntaxKind::Text)
    .add(SyntaxKind::MathShorthand)
    .add(SyntaxKind::Linebreak)
    .add(SyntaxKind::MathAlignPoint)
    .add(SyntaxKind::Escape)
    .add(SyntaxKind::Str)
    .add(SyntaxKind::Root)
    .add(SyntaxKind::Prime);

/// Syntax kinds that can start a code expression.
pub const CODE_EXPR: SyntaxSet = CODE_PRIMARY.union(UNARY_OP);

/// Syntax kinds that can start an atomic code expression.
pub const ATOMIC_CODE_EXPR: SyntaxSet = ATOMIC_CODE_PRIMARY;

/// Syntax kinds that can start a code primary.
pub const CODE_PRIMARY: SyntaxSet = ATOMIC_CODE_PRIMARY.add(SyntaxKind::Underscore);

/// Syntax kinds that can start an atomic code primary.
pub const ATOMIC_CODE_PRIMARY: SyntaxSet = SyntaxSet::new()
    .add(SyntaxKind::Ident)
    .add(SyntaxKind::LeftBrace)
    .add(SyntaxKind::LeftBracket)
    .add(SyntaxKind::LeftParen)
    .add(SyntaxKind::Dollar)
    .add(SyntaxKind::Let)
    .add(SyntaxKind::Set)
    .add(SyntaxKind::Show)
    .add(SyntaxKind::Context)
    .add(SyntaxKind::If)
    .add(SyntaxKind::While)
    .add(SyntaxKind::For)
    .add(SyntaxKind::Import)
    .add(SyntaxKind::Include)
    .add(SyntaxKind::Break)
    .add(SyntaxKind::Continue)
    .add(SyntaxKind::Return)
    .add(SyntaxKind::None)
    .add(SyntaxKind::Auto)
    .add(SyntaxKind::Int)
    .add(SyntaxKind::Float)
    .add(SyntaxKind::Bool)
    .add(SyntaxKind::Numeric)
    .add(SyntaxKind::Str)
    .add(SyntaxKind::Label)
    .add(SyntaxKind::RawDelim);

/// Syntax kinds that are unary operators.
pub const UNARY_OP: SyntaxSet = SyntaxSet::new()
    .add(SyntaxKind::Plus)
    .add(SyntaxKind::Minus)
    .add(SyntaxKind::Not);

/// Syntax kinds that are binary operators.
pub const BINARY_OP: SyntaxSet = SyntaxSet::new()
    .add(SyntaxKind::Plus)
    .add(SyntaxKind::Minus)
    .add(SyntaxKind::Star)
    .add(SyntaxKind::Slash)
    .add(SyntaxKind::And)
    .add(SyntaxKind::Or)
    .add(SyntaxKind::EqEq)
    .add(SyntaxKind::ExclEq)
    .add(SyntaxKind::Lt)
    .add(SyntaxKind::LtEq)
    .add(SyntaxKind::Gt)
    .add(SyntaxKind::GtEq)
    .add(SyntaxKind::Eq)
    .add(SyntaxKind::In)
    .add(SyntaxKind::PlusEq)
    .add(SyntaxKind::HyphEq)
    .add(SyntaxKind::StarEq)
    .add(SyntaxKind::SlashEq);

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
