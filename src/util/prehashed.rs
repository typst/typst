use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::Deref;

#[cfg(feature = "layout-cache")]
use std::any::Any;

/// A wrapper around a type that precomputes its hash.
#[derive(Copy, Clone)]
pub struct Prehashed<T: ?Sized> {
    /// The precomputed hash.
    #[cfg(feature = "layout-cache")]
    hash: u64,
    /// The wrapped item.
    item: T,
}

impl<T: Hash + 'static> Prehashed<T> {
    /// Compute an item's hash and wrap it.
    pub fn new(item: T) -> Self {
        Self {
            #[cfg(feature = "layout-cache")]
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

impl<T: Hash + ?Sized> Hash for Prehashed<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash the node.
        #[cfg(feature = "layout-cache")]
        state.write_u64(self.hash);
        #[cfg(not(feature = "layout-cache"))]
        self.item.hash(state);
    }
}

impl<T: Eq + ?Sized> Eq for Prehashed<T> {}

impl<T: PartialEq + ?Sized> PartialEq for Prehashed<T> {
    fn eq(&self, other: &Self) -> bool {
        #[cfg(feature = "layout-cache")]
        return self.hash == other.hash;
        #[cfg(not(feature = "layout-cache"))]
        self.item.eq(&other.item)
    }
}
