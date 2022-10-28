use std::any::Any;
use std::fmt::{self, Debug, Formatter};
use std::hash::Hash;
use std::sync::Arc;

use comemo::Prehashed;

use super::{Interruption, NodeId, Smart, StyleChain};
use crate::geom::{Abs, Axes, Corners, Em, Length, Numeric, Rel, Sides};
use crate::library::layout::PageNode;
use crate::library::structure::{DescNode, EnumNode, ListNode};
use crate::library::text::{ParNode, TextNode};
use crate::util::ReadableTypeId;

/// A style property originating from a set rule or constructor.
#[derive(Clone, Hash)]
pub struct Property {
    /// The id of the property's [key](Key).
    pub key: KeyId,
    /// The id of the node the property belongs to.
    pub node: NodeId,
    /// Whether the property should only affect the first node down the
    /// hierarchy. Used by constructors.
    pub scoped: bool,
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

/// A unique identifier for a property key.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct KeyId(ReadableTypeId);

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

/// Style property keys.
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

/// A property that is resolved with other properties from the style chain.
pub trait Resolve {
    /// The type of the resolved output.
    type Output;

    /// Resolve the value using the style chain.
    fn resolve(self, styles: StyleChain) -> Self::Output;
}

impl Resolve for Em {
    type Output = Abs;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        if self.is_zero() {
            Abs::zero()
        } else {
            self.at(styles.get(TextNode::SIZE))
        }
    }
}

impl Resolve for Length {
    type Output = Abs;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        self.abs + self.em.resolve(styles)
    }
}

impl<T: Resolve> Resolve for Option<T> {
    type Output = Option<T::Output>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        self.map(|v| v.resolve(styles))
    }
}

impl<T: Resolve> Resolve for Smart<T> {
    type Output = Smart<T::Output>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        self.map(|v| v.resolve(styles))
    }
}

impl<T: Resolve> Resolve for Axes<T> {
    type Output = Axes<T::Output>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        self.map(|v| v.resolve(styles))
    }
}

impl<T: Resolve> Resolve for Sides<T> {
    type Output = Sides<T::Output>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        self.map(|v| v.resolve(styles))
    }
}

impl<T: Resolve> Resolve for Corners<T> {
    type Output = Corners<T::Output>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        self.map(|v| v.resolve(styles))
    }
}

impl<T> Resolve for Rel<T>
where
    T: Resolve + Numeric,
    <T as Resolve>::Output: Numeric,
{
    type Output = Rel<<T as Resolve>::Output>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        self.map(|abs| abs.resolve(styles))
    }
}

/// A property that is folded to determine its final value.
pub trait Fold {
    /// The type of the folded output.
    type Output;

    /// Fold this inner value with an outer folded value.
    fn fold(self, outer: Self::Output) -> Self::Output;
}

impl<T> Fold for Option<T>
where
    T: Fold,
    T::Output: Default,
{
    type Output = Option<T::Output>;

    fn fold(self, outer: Self::Output) -> Self::Output {
        self.map(|inner| inner.fold(outer.unwrap_or_default()))
    }
}

impl<T> Fold for Smart<T>
where
    T: Fold,
    T::Output: Default,
{
    type Output = Smart<T::Output>;

    fn fold(self, outer: Self::Output) -> Self::Output {
        self.map(|inner| inner.fold(outer.unwrap_or_default()))
    }
}

impl<T> Fold for Sides<T>
where
    T: Fold,
{
    type Output = Sides<T::Output>;

    fn fold(self, outer: Self::Output) -> Self::Output {
        self.zip(outer, |inner, outer| inner.fold(outer))
    }
}

impl Fold for Sides<Option<Rel<Abs>>> {
    type Output = Sides<Rel<Abs>>;

    fn fold(self, outer: Self::Output) -> Self::Output {
        self.zip(outer, |inner, outer| inner.unwrap_or(outer))
    }
}

impl Fold for Sides<Option<Smart<Rel<Length>>>> {
    type Output = Sides<Smart<Rel<Length>>>;

    fn fold(self, outer: Self::Output) -> Self::Output {
        self.zip(outer, |inner, outer| inner.unwrap_or(outer))
    }
}

impl<T> Fold for Corners<T>
where
    T: Fold,
{
    type Output = Corners<T::Output>;

    fn fold(self, outer: Self::Output) -> Self::Output {
        self.zip(outer, |inner, outer| inner.fold(outer))
    }
}

impl Fold for Corners<Option<Rel<Abs>>> {
    type Output = Corners<Rel<Abs>>;

    fn fold(self, outer: Self::Output) -> Self::Output {
        self.zip(outer, |inner, outer| inner.unwrap_or(outer))
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
