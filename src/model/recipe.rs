use std::fmt::{self, Debug, Formatter};

use super::NodeId;
use crate::eval::{Func, Node};
use crate::syntax::Span;

/// A show rule recipe.
#[derive(Clone, PartialEq, Hash)]
pub struct Recipe {
    /// The affected node.
    pub node: NodeId,
    /// The function that defines the recipe.
    pub func: Func,
    /// The span to report all erros with.
    pub span: Span,
}

impl Recipe {
    /// Create a new recipe for the node `T`.
    pub fn new<T: Node>(func: Func, span: Span) -> Self {
        Self { node: NodeId::of::<T>(), func, span }
    }
}

impl Debug for Recipe {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Recipe for {:?} from {:?}", self.node, self.span)
    }
}
