//! The syntax tree.

use std::any::Any;
use std::fmt::Debug;

use crate::layout::Layout;
use super::span::SpanVec;

/// A list of nodes which forms a tree together with the nodes' children.
pub type SyntaxTree = SpanVec<SyntaxNode>;

/// A syntax node, which encompasses a single logical entity of parsed source
/// code.
#[derive(Debug, Clone)]
pub enum SyntaxNode {
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
    /// A subtree, typically a function invocation.
    Dyn(Box<dyn DynamicNode>),
}

impl PartialEq for SyntaxNode {
    fn eq(&self, other: &SyntaxNode) -> bool {
        use SyntaxNode::*;
        match (self, other) {
            (Space, Space) => true,
            (Parbreak, Parbreak) => true,
            (Linebreak, Linebreak) => true,
            (Text(a), Text(b)) => a == b,
            (Raw(a), Raw(b)) => a == b,
            (ToggleItalic, ToggleItalic) => true,
            (ToggleBolder, ToggleBolder) => true,
            (Dyn(a), Dyn(b)) => a == b,
            _ => false,
        }
    }
}

/// Dynamic syntax nodes.
///
/// *Note*: This is automatically implemented for all types which are
/// `Debug + Clone + PartialEq`, `Layout` and `'static`.
pub trait DynamicNode: Debug + Layout {
    /// Convert into a `dyn Any`.
    fn as_any(&self) -> &dyn Any;

    /// Check for equality with another dynamic node.
    fn dyn_eq(&self, other: &dyn DynamicNode) -> bool;

    /// Clone into a boxed node trait object.
    fn box_clone(&self) -> Box<dyn DynamicNode>;
}

impl dyn DynamicNode {
    /// Downcast this dynamic node to a concrete node.
    pub fn downcast<N>(&self) -> Option<&N> where N: DynamicNode + 'static {
        self.as_any().downcast_ref::<N>()
    }
}

impl PartialEq for dyn DynamicNode {
    fn eq(&self, other: &dyn DynamicNode) -> bool {
        self.dyn_eq(other)
    }
}

impl Clone for Box<dyn DynamicNode> {
    fn clone(&self) -> Self {
        self.box_clone()
    }
}

impl<T> DynamicNode for T where T: Debug + PartialEq + Clone + Layout + 'static {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn dyn_eq(&self, other: &dyn DynamicNode) -> bool {
        match other.as_any().downcast_ref::<Self>() {
            Some(other) => self == other,
            None => false,
        }
    }

    fn box_clone(&self) -> Box<dyn DynamicNode> {
        Box::new(self.clone())
    }
}
