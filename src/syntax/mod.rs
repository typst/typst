//! Tokenization and parsing of source code.

use std::any::Any;
use std::fmt::{self, Debug, Display, Formatter};
use std::future::Future;
use std::pin::Pin;
use serde::Serialize;

use crate::error::Error;
use crate::func::{Commands, Command};
use crate::layout::{Layouted, LayoutContext};
use crate::size::Size;

pub_use_mod!(expr);
pub_use_mod!(func);
pub_use_mod!(tokens);
pub_use_mod!(parsing);
pub_use_mod!(span);

/// Common syntax types.
pub mod prelude {
    pub use super::*;
}


#[async_trait::async_trait(?Send)]
pub trait Model: Debug + ModelBounds {
    async fn layout<'a>(
        &'a self,
        ctx: LayoutContext<'_, '_>
    ) -> Layouted<Commands<'a>>;
}

pub type DynFuture<'a, T> = Pin<Box<dyn Future<Output=T> + 'a>>;

impl dyn Model {
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

pub trait ModelBounds {
    fn as_any(&self) -> &dyn Any;
    fn bound_eq(&self, other: &dyn Model) -> bool;
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

/// A tree representation of source code.
#[derive(Debug, Clone, PartialEq)]
pub struct SyntaxModel {
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

#[async_trait::async_trait(?Send)]
impl Model for SyntaxModel {
    async fn layout<'a>(&'a self, _: LayoutContext<'_, '_>) -> Layouted<Commands<'a>> {
        Layouted {
            output: vec![Command::LayoutSyntaxModel(self)],
            errors: vec![],
        }
    }
}

/// A node in the syntax tree.
#[derive(Debug, Clone)]
pub enum Node {
    /// A number of whitespace characters containing less than two newlines.
    Space,
    /// Whitespace characters with more than two newlines.
    Newline,
    /// Plain text.
    Text(String),
    /// Italics enabled / disabled.
    ToggleItalic,
    /// Bolder enabled / disabled.
    ToggleBolder,
    /// Monospace enabled / disabled.
    ToggleMonospace,
    /// A submodel.
    Model(Box<dyn Model>),
}

impl PartialEq for Node {
    fn eq(&self, other: &Node) -> bool {
        use Node::*;
        match (self, other) {
            (Space, Space) => true,
            (Newline, Newline) => true,
            (Text(a), Text(b)) => a == b,
            (ToggleItalic, ToggleItalic) => true,
            (ToggleBolder, ToggleBolder) => true,
            (ToggleMonospace, ToggleMonospace) => true,
            (Model(a), Model(b)) => a == b,
            _ => false,
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Decoration {
    ValidFuncName,
    InvalidFuncName,
    ArgumentKey,
}
