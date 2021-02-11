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

/// The abstract syntax tree.
pub type Tree = Vec<Node>;
