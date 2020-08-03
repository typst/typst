//! Syntax trees, parsing and tokenization.

#[cfg(test)]
#[macro_use]
mod test;

pub mod decoration;
pub mod expr;
pub mod parsing;
pub mod scope;
pub mod span;
pub mod tokens;
pub mod tree;
pub mod value;

/// Basic types used around the syntax side.
pub mod prelude {
    pub use super::expr::*;
    pub use super::span::{Span, SpanVec, Spanned};
    pub use super::tree::{DynamicNode, SyntaxNode, SyntaxTree};
    pub use super::value::*;
}
