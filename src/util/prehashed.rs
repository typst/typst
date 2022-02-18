use std::any::Any;
use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::Deref;

/// A wrapper around a type that precomputes its hash.
#[derive(Copy, Clone)]
pub struct Prehashed<T: ?Sized> {
    /// The precomputed hash.
    hash: u64,
    /// The wrapped item.
    item: T,
}

impl<T: Hash + 'static> Prehashed<T> {
    /// Compute an item's hash and wrap it.
    pub fn new(item: T) -> Self {
        Self {
            hash: {
                // Also hash the TypeId because the type might be converted
                // through an unsized coercion.
                let mut state = fxhash::FxHasher64::default();
                item.type_id().hash(&mut state);
                item.hash(&mut state);
                state.finish()
            },
            item,
        }
    }

    /// Return the wrapped value.
    pub fn into_iter(self) -> T {
        self.item
    }
}

impl<T: ?Sized> Deref for Prehashed<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.item
    }
}

impl<T: Debug + ?Sized> Debug for Prehashed<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.item.fmt(f)
    }
}

impl<T: ?Sized> Hash for Prehashed<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.hash);
    }
}

impl<T: Eq + ?Sized> Eq for Prehashed<T> {}

impl<T: ?Sized> PartialEq for Prehashed<T> {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}
