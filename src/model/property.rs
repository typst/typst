use std::any::Any;
use std::fmt::{self, Debug, Formatter};
use std::hash::Hash;
use std::sync::Arc;

use comemo::Prehashed;

use super::{Interruption, NodeId, StyleChain};
use crate::library::layout::PageNode;
use crate::library::structure::{DescNode, EnumNode, ListNode};
use crate::library::text::ParNode;
use crate::util::ReadableTypeId;

/// A style property originating from a set rule or constructor.
#[derive(Clone, Hash)]
pub struct Property {
    /// The id of the property's [key](Key).
    key: KeyId,
    /// The id of the node the property belongs to.
    node: NodeId,
    /// Whether the property should only affect the first node down the
    /// hierarchy. Used by constructors.
    scoped: bool,
    /// The property's value.
    value: Arc<Prehashed<dyn Bounds>>,
    /// The name of the property.
    #[cfg(debug_assertions)]
    name: &'static str,
}

impl Property {
    /// Create a new property from a key-value pair.
    pub fn new<'a, K: Key<'a>>(_: K, value: K::Value) -> Self {
        Self {
            key: KeyId::of::<K>(),
            node: K::node(),
            value: Arc::new(Prehashed::new(value)),
            scoped: false,
            #[cfg(debug_assertions)]
            name: K::NAME,
        }
    }

    /// Whether this property has the given key.
    pub fn is<'a, K: Key<'a>>(&self) -> bool {
        self.key == KeyId::of::<K>()
    }

    /// Whether this property belongs to the node `T`.
    pub fn is_of<T: 'static>(&self) -> bool {
        self.node == NodeId::of::<T>()
    }

    /// Access the property's value if it is of the given key.
    pub fn downcast<'a, K: Key<'a>>(&'a self) -> Option<&'a K::Value> {
        if self.key == KeyId::of::<K>() {
            (**self.value).as_any().downcast_ref()
        } else {
            None
        }
    }

    /// The node this property is for.
    pub fn node(&self) -> NodeId {
        self.node
    }

    /// Whether the property is scoped.
    pub fn scoped(&self) -> bool {
        self.scoped
    }

    /// Make the property scoped.
    pub fn make_scoped(&mut self) {
        self.scoped = true;
    }

    /// What kind of structure the property interrupts.
    pub fn interruption(&self) -> Option<Interruption> {
        if self.is_of::<PageNode>() {
            Some(Interruption::Page)
        } else if self.is_of::<ParNode>() {
            Some(Interruption::Par)
        } else if self.is_of::<ListNode>()
            || self.is_of::<EnumNode>()
            || self.is_of::<DescNode>()
        {
            Some(Interruption::List)
        } else {
            None
        }
    }
}

impl Debug for Property {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        #[cfg(debug_assertions)]
        write!(f, "{} = ", self.name)?;
        write!(f, "{:?}", self.value)?;
        if self.scoped {
            write!(f, " [scoped]")?;
        }
        Ok(())
    }
}

impl PartialEq for Property {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
            && self.value.eq(&other.value)
            && self.scoped == other.scoped
    }
}

trait Bounds: Debug + Sync + Send + 'static {
    fn as_any(&self) -> &dyn Any;
}

impl<T> Bounds for T
where
    T: Debug + Sync + Send + 'static,
{
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A style property key.
///
/// This trait is not intended to be implemented manually, but rather through
/// the `#[node]` proc-macro.
pub trait Key<'a>: Copy + 'static {
    /// The unfolded type which this property is stored as in a style map.
    type Value: Debug + Clone + Hash + Sync + Send + 'static;

    /// The folded type of value that is returned when reading this property
    /// from a style chain.
    type Output;

    /// The name of the property, used for debug printing.
    const NAME: &'static str;

    /// The id of the node the key belongs to.
    fn node() -> NodeId;

    /// Compute an output value from a sequence of values belonging to this key,
    /// folding if necessary.
    fn get(
        chain: StyleChain<'a>,
        values: impl Iterator<Item = &'a Self::Value>,
    ) -> Self::Output;
}

/// A unique identifier for a property key.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
struct KeyId(ReadableTypeId);

impl KeyId {
    /// The id of the given key.
    pub fn of<'a, T: Key<'a>>() -> Self {
        Self(ReadableTypeId::of::<T>())
    }
}

impl Debug for KeyId {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// A scoped property barrier.
///
/// Barriers interact with [scoped](super::StyleMap::scoped) styles: A scoped
/// style can still be read through a single barrier (the one of the node it
/// _should_ apply to), but a second barrier will make it invisible.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct Barrier(NodeId);

impl Barrier {
    /// Create a new barrier for the given node.
    pub fn new(node: NodeId) -> Self {
        Self(node)
    }

    /// Whether this barrier is for the node `T`.
    pub fn is_for(&self, node: NodeId) -> bool {
        self.0 == node
    }
}

impl Debug for Barrier {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Barrier for {:?}", self.0)
    }
}
