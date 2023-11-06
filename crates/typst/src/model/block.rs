use std::any::Any;
use std::fmt::{self, Debug};
use std::hash::{Hash, Hasher};

use smallbox::{smallbox, SmallBox};

/// A small block storage for storing stylechain values
/// either on the stack (if they fit) or on the heap.
pub struct Block(SmallBox<dyn Blockable, S3>);

impl Block {
    /// Creates a new block.
    pub fn new<T: Blockable>(value: T) -> Self {
        Self(smallbox!(value))
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.dyn_debug(f)
    }
}

/// The marker for the size of the block.
#[doc(hidden)]
struct S3 {
    _inner: [usize; 3],
}

/// A value that can be stored in a block.
///
/// Auto derived for all types that implement [`Any`], [`Clone`], [`Hash`],
/// [`Debug`], [`Send`] and [`Sync`].
pub trait Blockable: Send + Sync + 'static {
    /// Equivalent to [`Hash`] for the block.
    fn dyn_hash(&self, state: &mut dyn Hasher);

    /// Equivalent to [`Clone`] for the block.
    fn dyn_clone(&self) -> Block;

    /// Equivalent to [`Debug`] for the block.
    fn dyn_debug(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;

    /// Equivalent to `downcast_ref` for the block.
    fn as_any(&self) -> &dyn Any;

    /// Equivalent to `downcast_mut` for the block.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: Clone + Hash + Debug + Send + Sync + 'static> Blockable for T {
    fn dyn_hash(&self, mut state: &mut dyn Hasher) {
        self.hash(&mut state);
    }

    fn dyn_clone(&self) -> Block {
        Block(smallbox!(self.clone()))
    }

    fn dyn_debug(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(self, f)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
