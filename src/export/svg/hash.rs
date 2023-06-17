use std::{
    any::Any,
    hash::{Hash, Hasher},
    ops::Deref,
};

use siphasher::sip128::{Hasher128, SipHasher13};

/// Trait for the objects that directly store the hash value for itself.
pub trait StaticHash128 {
    fn get_hash(&self) -> u128;
}

/// Automatically implement [`Hash`] for the types that implement [`StaticHash128`].
impl Hash for dyn StaticHash128 {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u128(self.get_hash());
    }
}

/// hash with both data and type id.
pub fn make_item_hash<T: Hash + 'static>(item: &T) -> u128 {
    // Also hash the TypeId because the type might be converted
    // through an unsized coercion.
    let mut state = SipHasher13::new();
    item.type_id().hash(&mut state);
    item.hash(&mut state);
    state.finish128().as_u128()
}

/// Delegate the hash for trait objects.
pub struct HashedTrait<T: ?Sized> {
    hash: u128,
    t: Box<T>,
}

impl<T: ?Sized> HashedTrait<T> {
    pub fn new(hash: u128, t: Box<T>) -> Self {
        Self { hash, t }
    }
}

impl<T: ?Sized> Deref for HashedTrait<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.t
    }
}

impl<T> Hash for HashedTrait<T> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u128(self.hash);
    }
}

/// Implement [`HashedTrait::Default`] for items.
impl<T: Hash + Default + 'static> Default for HashedTrait<T> {
    fn default() -> Self {
        let t = T::default();
        Self { hash: make_item_hash(&t), t: Box::new(t) }
    }
}

impl<T: ?Sized> StaticHash128 for HashedTrait<T> {
    fn get_hash(&self) -> u128 {
        self.hash
    }
}

/// This function maintain hash function corresponding to Typst
/// Typst changed the hash function from `siphasher::sip128::SipHasher` to
///   `siphasher::sip128::SipHasher13` since commit
///   <https://github.com/typst/typst/commit/d0afba959d18d1c2c646b99e6ddd864b1a91deb2>
/// Commit log:
/// This seems to significantly improves performance. Inspired by rust-lang/rust#107925
pub fn typst_affinite_hash<T: std::hash::Hash>(t: &T) -> u128 {
    let mut s = SipHasher13::new();
    t.hash(&mut s);
    s.finish128().as_u128()
}
