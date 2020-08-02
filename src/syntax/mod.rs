//! Syntax trees, parsing and tokenization.

#[cfg(test)]
#[macro_use]
mod test;

/// Basic types used around the syntax side.
pub mod prelude {
    pub use super::expr::*;
    pub use super::tree::{SyntaxTree, SyntaxNode, DynamicNode};
    pub use super::span::{SpanVec, Span, Spanned};
    pub use super::value::*;
}

pub mod decoration;
pub mod expr;
pub mod tree;
pub mod parsing;
pub mod span;
pub mod scope;
pub mod tokens;
pub mod value;
