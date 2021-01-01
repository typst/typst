//! Layout nodes.

use std::any::Any;
use std::fmt::{self, Debug, Formatter};

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
    pub fn dynamic<T>(inner: T) -> Self
    where
        T: Layout + Debug + Clone + PartialEq + 'static,
    {
        Self::Dyn(Dynamic::new(inner))
    }
}

impl Layout for LayoutNode {
    fn layout(&self, ctx: &mut LayoutContext, areas: &Areas) -> Layouted {
        match self {
            Self::Spacing(spacing) => spacing.layout(ctx, areas),
            Self::Text(text) => text.layout(ctx, areas),
            Self::Dyn(dynamic) => dynamic.layout(ctx, areas),
        }
    }
}

impl Debug for LayoutNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Spacing(spacing) => spacing.fmt(f),
            Self::Text(text) => text.fmt(f),
            Self::Dyn(dynamic) => dynamic.fmt(f),
        }
    }
}

/// A wrapper around a dynamic layouting node.
pub struct Dynamic(Box<dyn Bounds>);

impl Dynamic {
    /// Create a new instance from any node that satisifies the required bounds.
    pub fn new<T>(inner: T) -> Self
    where
        T: Layout + Debug + Clone + PartialEq + 'static,
    {
        Self(Box::new(inner))
    }
}

impl Layout for Dynamic {
    fn layout(&self, ctx: &mut LayoutContext, areas: &Areas) -> Layouted {
        self.0.layout(ctx, areas)
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

trait Bounds: Layout + Debug + 'static {
    fn as_any(&self) -> &dyn Any;
    fn dyn_eq(&self, other: &dyn Bounds) -> bool;
    fn dyn_clone(&self) -> Box<dyn Bounds>;
}

impl<T> Bounds for T
where
    T: Layout + Debug + PartialEq + Clone + 'static,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn dyn_eq(&self, other: &dyn Bounds) -> bool {
        if let Some(other) = other.as_any().downcast_ref::<Self>() {
            self == other
        } else {
            false
        }
    }

    fn dyn_clone(&self) -> Box<dyn Bounds> {
        Box::new(self.clone())
    }
}
