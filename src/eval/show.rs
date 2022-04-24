use std::any::{Any, TypeId};
use std::fmt::{self, Debug, Formatter};
use std::hash::Hash;
use std::sync::Arc;

use super::{Content, Dict, StyleChain};
use crate::diag::TypResult;
use crate::util::Prehashed;
use crate::Context;

/// A node that can be realized given some styles.
pub trait Show: 'static {
    /// Encode this node into a dictionary.
    fn encode(&self) -> Dict;

    /// The base recipe for this node that is executed if there is no
    /// user-defined show rule.
    fn realize(&self, ctx: &mut Context, styles: StyleChain) -> TypResult<Content>;

    /// Finalize this node given the realization of a base or user recipe. Use
    /// this for effects that should work even in the face of a user-defined
    /// show rule, for example:
    /// - Application of general settable properties
    /// - Attaching things like semantics to a heading
    ///
    /// Defaults to just the realized content.
    #[allow(unused_variables)]
    fn finalize(
        &self,
        ctx: &mut Context,
        styles: StyleChain,
        realized: Content,
    ) -> TypResult<Content> {
        Ok(realized)
    }

    /// Convert to a packed show node.
    fn pack(self) -> ShowNode
    where
        Self: Debug + Hash + Sized + Sync + Send + 'static,
    {
        ShowNode::new(self)
    }
}

/// A type-erased showable node with a precomputed hash.
#[derive(Clone, Hash)]
pub struct ShowNode(Arc<Prehashed<dyn Bounds>>);

impl ShowNode {
    /// Pack any showable node.
    pub fn new<T>(node: T) -> Self
    where
        T: Show + Debug + Hash + Sync + Send + 'static,
    {
        Self(Arc::new(Prehashed::new(node)))
    }

    /// The type id of this node.
    pub fn id(&self) -> TypeId {
        self.0.as_any().type_id()
    }
}

impl Show for ShowNode {
    fn encode(&self) -> Dict {
        self.0.encode()
    }

    fn realize(&self, ctx: &mut Context, styles: StyleChain) -> TypResult<Content> {
        self.0.realize(ctx, styles)
    }

    fn finalize(
        &self,
        ctx: &mut Context,
        styles: StyleChain,
        realized: Content,
    ) -> TypResult<Content> {
        self.0.finalize(ctx, styles, realized)
    }

    fn pack(self) -> ShowNode {
        self
    }
}

impl Debug for ShowNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl PartialEq for ShowNode {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

trait Bounds: Show + Debug + Sync + Send + 'static {
    fn as_any(&self) -> &dyn Any;
}

impl<T> Bounds for T
where
    T: Show + Debug + Hash + Sync + Send + 'static,
{
    fn as_any(&self) -> &dyn Any {
        self
    }
}
