use std::fmt::{self, Debug, Formatter, Write};
use std::hash::Hash;
use std::sync::Arc;

use comemo::{Prehashed, Tracked};

use super::{Content, NodeId, Selector, StyleChain, Value};
use crate::diag::SourceResult;
use crate::World;

/// A node that can be realized given some styles.
pub trait Show: 'static {
    /// Unguard nested content against recursive show rules.
    fn unguard(&self, sel: Selector) -> ShowNode;

    /// Access a field on this node.
    fn field(&self, name: &str) -> Option<Value>;

    /// The base recipe for this node that is executed if there is no
    /// user-defined show rule.
    fn realize(
        &self,
        world: Tracked<dyn World>,
        styles: StyleChain,
    ) -> SourceResult<Content>;

    /// Finalize this node given the realization of a base or user recipe. Use
    /// this for effects that should work even in the face of a user-defined
    /// show rule, for example:
    /// - Application of general settable properties
    ///
    /// Defaults to just the realized content.
    #[allow(unused_variables)]
    fn finalize(
        &self,
        world: Tracked<dyn World>,
        styles: StyleChain,
        realized: Content,
    ) -> SourceResult<Content> {
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

    /// The id of this node.
    pub fn id(&self) -> NodeId {
        (**self.0).node_id()
    }
}

impl Show for ShowNode {
    fn unguard(&self, sel: Selector) -> ShowNode {
        self.0.unguard(sel)
    }

    fn field(&self, name: &str) -> Option<Value> {
        self.0.field(name)
    }

    fn realize(
        &self,
        world: Tracked<dyn World>,
        styles: StyleChain,
    ) -> SourceResult<Content> {
        self.0.realize(world, styles)
    }

    fn finalize(
        &self,
        world: Tracked<dyn World>,
        styles: StyleChain,
        realized: Content,
    ) -> SourceResult<Content> {
        self.0.finalize(world, styles, realized)
    }

    fn pack(self) -> ShowNode {
        self
    }
}

impl Debug for ShowNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("Show(")?;
        self.0.fmt(f)?;
        f.write_char(')')
    }
}

impl PartialEq for ShowNode {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

trait Bounds: Show + Debug + Sync + Send + 'static {
    fn node_id(&self) -> NodeId;
}

impl<T> Bounds for T
where
    T: Show + Debug + Hash + Sync + Send + 'static,
{
    fn node_id(&self) -> NodeId {
        NodeId::of::<Self>()
    }
}
