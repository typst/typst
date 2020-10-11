//! Layout nodes.

use std::any::Any;
use std::fmt::{self, Debug, Formatter};
use std::ops::Deref;

use super::*;

/// A self-contained, styled layout node.
#[derive(Clone, PartialEq)]
pub enum LayoutNode {
    /// A spacing node.
    Spacing(Spacing),
    /// A text node.
    Text(Text),
    /// A dynamic that can implement custom layouting behaviour.
    Dyn(Dynamic),
}

impl LayoutNode {
    /// Create a new dynamic node.
    pub fn dynamic<T: DynNode>(inner: T) -> Self {
        Self::Dyn(Dynamic::new(inner))
    }
}

#[async_trait(?Send)]
impl Layout for LayoutNode {
    async fn layout(&self, ctx: &mut LayoutContext, areas: &Areas) -> Vec<Layouted> {
        match self {
            Self::Spacing(spacing) => spacing.layout(ctx, areas).await,
            Self::Text(text) => text.layout(ctx, areas).await,
            Self::Dyn(boxed) => boxed.layout(ctx, areas).await,
        }
    }
}

impl Debug for LayoutNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Spacing(spacing) => spacing.fmt(f),
            Self::Text(text) => text.fmt(f),
            Self::Dyn(boxed) => boxed.fmt(f),
        }
    }
}

/// A wrapper around a boxed dynamic node.
///
/// _Note_: This is needed because the compiler can't `derive(PartialEq)` for
///         [`LayoutNode`] when directly putting the boxed node in there, see
///         the [Rust Issue].
///
/// [`LayoutNode`]: enum.LayoutNode.html
/// [Rust Issue]: https://github.com/rust-lang/rust/issues/31740
pub struct Dynamic(pub Box<dyn DynNode>);

impl Dynamic {
    /// Wrap a type implementing `DynNode`.
    pub fn new<T: DynNode>(inner: T) -> Self {
        Self(Box::new(inner))
    }
}

impl Deref for Dynamic {
    type Target = dyn DynNode;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

impl Debug for Dynamic {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<Dynamic> for LayoutNode {
    fn from(dynamic: Dynamic) -> Self {
        Self::Dyn(dynamic)
    }
}

impl Clone for Dynamic {
    fn clone(&self) -> Self {
        Self(self.0.dyn_clone())
    }
}

impl PartialEq for Dynamic {
    fn eq(&self, other: &Self) -> bool {
        self.0.dyn_eq(other.0.as_ref())
    }
}

/// A dynamic node, which can implement custom layouting behaviour.
///
/// This trait just combines the requirements for types to qualify as dynamic
/// nodes. The interesting part happens in the inherited trait [`Layout`].
///
/// The trait itself also contains three helper methods to make `Box<dyn
/// DynNode>` able to implement `Clone` and `PartialEq`. However, these are
/// automatically provided by a blanket impl as long as the type in question
/// implements[`Layout`],  `Debug`, `PartialEq`, `Clone` and is `'static`.
///
/// [`Layout`]: ../trait.Layout.html
pub trait DynNode: Debug + Layout + 'static {
    /// Convert into a `dyn Any` to enable downcasting.
    fn as_any(&self) -> &dyn Any;

    /// Check for equality with another trait object.
    fn dyn_eq(&self, other: &dyn DynNode) -> bool;

    /// Clone into a trait object.
    fn dyn_clone(&self) -> Box<dyn DynNode>;
}

impl<T> DynNode for T
where
    T: Debug + Layout + PartialEq + Clone + 'static,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn dyn_eq(&self, other: &dyn DynNode) -> bool {
        if let Some(other) = other.as_any().downcast_ref::<Self>() {
            self == other
        } else {
            false
        }
    }

    fn dyn_clone(&self) -> Box<dyn DynNode> {
        Box::new(self.clone())
    }
}
