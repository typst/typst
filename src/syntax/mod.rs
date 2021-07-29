//! Syntax types.

mod expr;
mod ident;
mod node;
mod span;
mod token;
pub mod visit;

pub use expr::*;
pub use ident::*;
pub use node::*;
pub use span::*;
pub use token::*;

use crate::util::EcoString;

/// The abstract syntax tree.
///
/// This type can represent a full parsed document.
pub type SyntaxTree = Vec<SyntaxNode>;
