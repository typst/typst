//! Abstract syntax tree definition.

mod expr;
mod lit;
mod node;

pub use expr::*;
pub use lit::*;
pub use node::*;

use super::{Ident, SpanVec, Spanned};

/// A collection of nodes which form a tree together with the nodes' children.
pub type SynTree = SpanVec<SynNode>;
