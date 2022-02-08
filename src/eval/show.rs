use std::any::{Any, TypeId};
use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use super::{StyleChain, Template};
use crate::util::Prehashed;

/// A node that can be realized given some styles.
pub trait Show {
    /// Realize the template in the given styles.
    fn show(&self, styles: StyleChain) -> Template;

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
    fn show(&self, styles: StyleChain) -> Template {
        self.0.show(styles)
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
        // We cast to thin pointers for comparison because we don't want to
        // compare vtables (which can be different across codegen units).
        std::ptr::eq(
            Arc::as_ptr(&self.0) as *const (),
            Arc::as_ptr(&other.0) as *const (),
        )
    }
}

trait Bounds: Show + Debug + Sync + Send + 'static {
    fn as_any(&self) -> &dyn Any;
    fn hash64(&self) -> u64;
}

impl<T> Bounds for T
where
    T: Show + Debug + Hash + Sync + Send + 'static,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn hash64(&self) -> u64 {
        // Also hash the TypeId since nodes with different types but
        // equal data should be different.
        let mut state = fxhash::FxHasher64::default();
        self.type_id().hash(&mut state);
        self.hash(&mut state);
        state.finish()
    }
}

impl Hash for dyn Bounds {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.hash64());
    }
}
