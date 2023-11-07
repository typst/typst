use std::any::Any;
use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};

/// A block storage for storing stylechain values either on the stack (if they
/// fit) or on the heap.
///
/// We're using a `Box` since values will either be contained in an `Arc` and
/// therefore already on the heap or they will be small enough that we can just
/// clone them.
pub struct Block(Box<dyn Blockable>);

impl Block {
    /// Creates a new block.
    pub fn new<T: Blockable>(value: T) -> Self {
        Self(Box::new(value))
    }

    /// Downcasts the block to the specified type.
    pub fn downcast<T: Blockable>(&self) -> Option<&T> {
        self.0.as_any().downcast_ref()
    }

    /// Downcasts mutably the block to the specified type.
    pub fn downcast_mut<T: Blockable>(&mut self) -> Option<&mut T> {
        self.0.as_any_mut().downcast_mut()
    }
}

impl Hash for Block {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.dyn_hash(state);
    }
}

impl Clone for Block {
    fn clone(&self) -> Self {
        self.0.dyn_clone()
    }
}

impl Debug for Block {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// A value that can be stored in a block.
///
/// Auto derived for all types that implement [`Any`], [`Clone`], [`Hash`],
/// [`Debug`], [`Send`] and [`Sync`].
pub trait Blockable: Debug + Send + Sync + 'static {
    /// Equivalent to `downcast_ref` for the block.
    fn as_any(&self) -> &dyn Any;

    /// Equivalent to `downcast_mut` for the block.
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Equivalent to [`Hash`] for the block.
    fn dyn_hash(&self, state: &mut dyn Hasher);

    /// Equivalent to [`Clone`] for the block.
    fn dyn_clone(&self) -> Block;
}

impl<T: Clone + Hash + Debug + Send + Sync + 'static> Blockable for T {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn dyn_hash(&self, mut state: &mut dyn Hasher) {
        self.hash(&mut state);
    }

    fn dyn_clone(&self) -> Block {
        Block(Box::new(self.clone()))
    }
}
