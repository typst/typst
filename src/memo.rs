//! Function memoization.

use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};
use std::hash::{Hash, Hasher};

thread_local! {
    /// The thread-local cache.
    static CACHE: RefCell<Cache> = RefCell::default();
}

/// A map from hashes to cache entries.
type Cache = HashMap<u64, CacheEntry>;

/// Access the cache.
fn with<F, R>(f: F) -> R
where
    F: FnOnce(&mut Cache) -> R,
{
    CACHE.with(|cell| f(&mut cell.borrow_mut()))
}

/// An entry in the cache.
struct CacheEntry {
    /// The memoized function's result.
    data: Box<dyn Any>,
    /// How many evictions have passed since the entry has been last used.
    age: usize,
}

/// Execute a memoized function call.
///
/// This hashes all inputs to the function and then either returns a cached
/// version from the thread-local cache or executes the function and saves a
/// copy of the results in the cache.
///
/// Note that `f` must be a pure function.
pub fn memoized<I, O>(input: I, f: fn(input: I) -> O) -> O
where
    I: Hash,
    O: Clone + 'static,
{
    memoized_ref(input, f, Clone::clone)
}

/// Execute a function and then call another function with a reference to the
/// result.
///
/// This hashes all inputs to the function and then either
/// - calls `g` with a cached version from the thread-local cache,
/// - or executes `f`, calls `g` with the fresh version and saves the result in
///   the cache.
///
/// Note that `f` must be a pure function, while `g` does not need to be pure.
pub fn memoized_ref<I, O, G, R>(input: I, f: fn(input: I) -> O, g: G) -> R
where
    I: Hash,
    O: 'static,
    G: Fn(&O) -> R,
{
    let hash = fxhash::hash64(&(f, &input));
    let result = with(|cache| {
        let entry = cache.get_mut(&hash)?;
        entry.age = 0;
        entry.data.downcast_ref().map(|output| g(output))
    });

    result.unwrap_or_else(|| {
        let output = f(input);
        let result = g(&output);
        let entry = CacheEntry { data: Box::new(output), age: 0 };
        with(|cache| cache.insert(hash, entry));
        result
    })
}

/// Garbage-collect the thread-local cache.
///
/// This deletes elements which haven't been used in a while and returns details
/// about the eviction.
pub fn evict() -> Eviction {
    with(|cache| {
        const MAX_AGE: usize = 5;

        let before = cache.len();
        cache.retain(|_, entry| {
            entry.age += 1;
            entry.age <= MAX_AGE
        });

        Eviction { before, after: cache.len() }
    })
}

/// Details about a cache eviction.
pub struct Eviction {
    /// The number of items in the cache before the eviction.
    pub before: usize,
    /// The number of items in the cache after the eviction.
    pub after: usize,
}

impl Display for Eviction {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        writeln!(f, "Before: {}", self.before)?;
        writeln!(f, "Evicted: {}", self.before - self.after)?;
        writeln!(f, "After: {}", self.after)
    }
}

// These impls are temporary and incorrect.

impl Hash for crate::font::FontStore {
    fn hash<H: Hasher>(&self, _: &mut H) {}
}

impl Hash for crate::Context {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.pins.hash(state);
    }
}
