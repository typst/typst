use std::{fmt::{self, Debug}, hash::{Hash, Hasher}, ops::{Deref, DerefMut}, sync::atomic::{AtomicU64, Ordering}};

use atomic::Atomic;

use super::hash128;

pub static HITS: AtomicU64 = AtomicU64::new(0);
pub static MISSES: AtomicU64 = AtomicU64::new(0);

pub struct LazyHash<T: ?Sized> {
    hash: Atomic<u128>,
    pub value: T,
}

impl<T> LazyHash<T> {
    pub fn new(value: T) -> Self {
        Self {
            hash: Atomic::new(0),
            value,
        }
    }

    pub fn into_inner(self) -> T {
        self.value
    }
}

impl<T: Hash + ?Sized> LazyHash<T> {
    pub fn get_hash(&self) -> u128 {
        self.hash.load(Ordering::Acquire)
    }

    pub fn reset_hash(&self) {
        self.hash.store(0, Ordering::Release);
    }

    pub fn get_or_set_hash(&self) -> u128 {
        let hash = self.get_hash();
        if hash == 0 {
            MISSES.fetch_add(1, Ordering::Relaxed);
            let hash = hash128(&self.value);
            self.hash.store(hash, Ordering::Release);
            hash
        } else {
            HITS.fetch_add(1, Ordering::Relaxed);
            hash
        }
    }
}

impl<T: Hash + ?Sized> Hash for LazyHash<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.get_or_set_hash().hash(state);
    }
}

impl<T: Hash + ?Sized> PartialEq for LazyHash<T> {
    fn eq(&self, other: &Self) -> bool {
        self.get_or_set_hash() == other.get_or_set_hash()
    }
}

impl<T: Hash + ?Sized> Eq for LazyHash<T> {}

impl<T: ?Sized> Deref for LazyHash<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T: Hash + ?Sized> DerefMut for LazyHash<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.reset_hash();
        &mut self.value
    }
}

impl<T: Hash + Clone> Clone for LazyHash<T> {
    fn clone(&self) -> Self {
        Self {
            hash: Atomic::new(self.get_hash()),
            value: self.value.clone(),
        }
    }
}

impl<T: Debug> Debug for LazyHash<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.value.fmt(f)
    }
}
