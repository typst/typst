use std::any::Any;
use std::fmt::{self, Debug, Formatter};

use super::*;

/// A self-contained layout node.
#[derive(Clone, PartialEq)]
pub enum Node {
    /// A text node.
    Text(TextNode),
    /// A spacing node.
    Spacing(SpacingNode),
    /// A dynamic node that can implement custom layouting behaviour.
    Any(AnyNode),
}

impl Layout for Node {
    fn layout(&self, ctx: &mut LayoutContext, areas: &Areas) -> Fragment {
        match self {
            Self::Spacing(spacing) => spacing.layout(ctx, areas),
            Self::Text(text) => text.layout(ctx, areas),
            Self::Any(any) => any.layout(ctx, areas),
        }
    }
}

impl Debug for Node {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Spacing(spacing) => spacing.fmt(f),
            Self::Text(text) => text.fmt(f),
            Self::Any(any) => any.fmt(f),
        }
    }
}

/// A wrapper around a dynamic layouting node.
pub struct AnyNode(Box<dyn Bounds>);

impl AnyNode {
    /// Create a new instance from any node that satisifies the required bounds.
    pub fn new<T>(any: T) -> Self
    where
        T: Layout + Debug + Clone + PartialEq + 'static,
    {
        Self(Box::new(any))
    }
}

impl Layout for AnyNode {
    fn layout(&self, ctx: &mut LayoutContext, areas: &Areas) -> Fragment {
        self.0.layout(ctx, areas)
    }
}

impl Clone for AnyNode {
    fn clone(&self) -> Self {
        Self(self.0.dyn_clone())
    }
}

impl PartialEq for AnyNode {
    fn eq(&self, other: &Self) -> bool {
        self.0.dyn_eq(other.0.as_ref())
    }
}

impl Debug for AnyNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<T> From<T> for Node
where
    T: Into<AnyNode>,
{
    fn from(t: T) -> Self {
        Self::Any(t.into())
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
