//! Syntax types.

mod expr;
mod ident;
mod lit;
mod node;
mod span;
mod token;

pub use expr::*;
pub use ident::*;
pub use lit::*;
pub use node::*;
pub use span::*;
pub use token::*;

/// A collection of nodes which form a tree together with the nodes' children.
pub type SynTree = SpanVec<SynNode>;
