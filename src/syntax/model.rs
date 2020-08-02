//! The syntax model.

use std::any::Any;
use std::fmt::Debug;
use async_trait::async_trait;

use crate::{Pass, Feedback};
use crate::layout::{LayoutContext, Commands, Command};
use super::span::{Spanned, SpanVec};

/// Represents a parsed piece of source that can be layouted and in the future
/// also be queried for information used for refactorings, autocomplete, etc.
#[async_trait(?Send)]
pub trait Model: Debug + ModelBounds {
    /// Layout the model into a sequence of commands processed by a
    /// [`ModelLayouter`](crate::layout::ModelLayouter).
    async fn layout<'a>(&'a self, ctx: LayoutContext<'_>) -> Pass<Commands<'a>>;
}

/// A tree representation of source code.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct SyntaxModel {
    /// The syntactical elements making up this model.
    pub nodes: SpanVec<Node>,
}

impl SyntaxModel {
    /// Create an empty syntax model.
    pub fn new() -> SyntaxModel {
        SyntaxModel { nodes: vec![] }
    }

    /// Add a node to the model.
    pub fn add(&mut self, node: Spanned<Node>) {
        self.nodes.push(node);
    }
}

#[async_trait(?Send)]
impl Model for SyntaxModel {
    async fn layout<'a>(&'a self, _: LayoutContext<'_>) -> Pass<Commands<'a>> {
        Pass::new(vec![Command::LayoutSyntaxModel(self)], Feedback::new())
    }
}

/// A node in the [syntax model](SyntaxModel).
#[derive(Debug, Clone)]
pub enum Node {
    /// Whitespace containing less than two newlines.
    Space,
    /// Whitespace with more than two newlines.
    Parbreak,
    /// A forced line break.
    Linebreak,
    /// Plain text.
    Text(String),
    /// Lines of raw text.
    Raw(Vec<String>),
    /// Italics were enabled / disabled.
    ToggleItalic,
    /// Bolder was enabled / disabled.
    ToggleBolder,
    /// A submodel, typically a function invocation.
    Model(Box<dyn Model>),
}

impl PartialEq for Node {
    fn eq(&self, other: &Node) -> bool {
        use Node::*;
        match (self, other) {
            (Space, Space) => true,
            (Parbreak, Parbreak) => true,
            (Linebreak, Linebreak) => true,
            (Text(a), Text(b)) => a == b,
            (Raw(a), Raw(b)) => a == b,
            (ToggleItalic, ToggleItalic) => true,
            (ToggleBolder, ToggleBolder) => true,
            (Model(a), Model(b)) => a == b,
            _ => false,
        }
    }
}

impl dyn Model {
    /// Downcast this model to a concrete type implementing [`Model`].
    pub fn downcast<T>(&self) -> Option<&T> where T: Model + 'static {
        self.as_any().downcast_ref::<T>()
    }
}

impl PartialEq for dyn Model {
    fn eq(&self, other: &dyn Model) -> bool {
        self.bound_eq(other)
    }
}

impl Clone for Box<dyn Model> {
    fn clone(&self) -> Self {
        self.bound_clone()
    }
}

/// This trait describes bounds necessary for types implementing [`Model`]. It is
/// automatically implemented for all types that are [`Model`], [`PartialEq`],
/// [`Clone`] and `'static`.
///
/// It is necessary to make models comparable and clonable.
pub trait ModelBounds {
    /// Convert into a `dyn Any`.
    fn as_any(&self) -> &dyn Any;

    /// Check for equality with another model.
    fn bound_eq(&self, other: &dyn Model) -> bool;

    /// Clone into a boxed model trait object.
    fn bound_clone(&self) -> Box<dyn Model>;
}

impl<T> ModelBounds for T where T: Model + PartialEq + Clone + 'static {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn bound_eq(&self, other: &dyn Model) -> bool {
        match other.as_any().downcast_ref::<Self>() {
            Some(other) => self == other,
            None => false,
        }
    }

    fn bound_clone(&self) -> Box<dyn Model> {
        Box::new(self.clone())
    }
}
