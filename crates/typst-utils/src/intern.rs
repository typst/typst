use std::collections::HashMap;
use std::fmt::{self, Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::num::{NonZeroU16, NonZeroU32};
use std::ops::Deref;
use std::sync::RwLock;

use rustc_hash::{FxBuildHasher, FxHashMap};

/// An internible type.
pub trait Intern: Hash + Eq + Sized + 'static {
    /// The type of index used to intern this type. Typically, `u16` or `u32`.
    ///
    /// The interner never releases values. Thus, the index type bounds the
    /// number of values of this kind that can ever exist.
    type Index: Index;

    const INTERNER: &'static Interner<Self>;

    /// Interns an instance of this type.
    fn intern(self) -> Id<Self> {
        Self::INTERNER.intern(self)
    }
}

/// An index type for an interned type.
pub trait Index: Copy + Eq + Hash + Sized {
    /// Try to create an instance of an index from a `usize`.
    ///
    /// Can return `None` if the value is out of the representable range of this
    /// index type. In this case, the interner will panic.
    fn from_usize(idx: usize) -> Option<Self>;

    /// Turns an index back into a usize.
    fn to_usize(self) -> usize;
}

impl Index for NonZeroU16 {
    fn from_usize(idx: usize) -> Option<Self> {
        u16::try_from(idx).ok().and_then(Self::new)
    }

    fn to_usize(self) -> usize {
        usize::from(self.get())
    }
}

impl Index for NonZeroU32 {
    fn from_usize(idx: usize) -> Option<Self> {
        u32::try_from(idx).ok().and_then(Self::new)
    }

    fn to_usize(self) -> usize {
        self.get() as usize
    }
}

/// An interned value of type `T`.
pub struct Id<T: Intern>(T::Index);

impl<T: Intern> Id<T> {
    /// Creates a new instance that's globally unique and does not compare equal
    /// to any other interned value regardless of whether the underlying `T`
    /// would compare equal.
    pub fn unique(value: T) -> Self {
        T::INTERNER.intern_unique(value)
    }

    /// Construct from a raw number.
    ///
    /// Should only be used with numbers retrieved via
    /// [`into_raw`](Self::into_raw). Misuse may results in panics, but no
    /// unsafety.
    pub const fn from_raw(v: T::Index) -> Self {
        Self(v)
    }

    /// Extract the raw underlying number.
    pub const fn into_raw(self) -> T::Index {
        self.0
    }

    /// Gets a `'static` reference to the underlying value.
    #[track_caller]
    pub fn get(self) -> &'static T {
        T::INTERNER.get(self)
    }
}

impl<T: Intern> Deref for Id<T> {
    type Target = T;

    #[track_caller]
    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl<T: Intern + Debug> Debug for Id<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&**self, f)
    }
}

impl<T: Intern + Display> Display for Id<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&**self, f)
    }
}

impl<T: Intern> Copy for Id<T> {}

impl<T: Intern> Clone for Id<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Intern> Eq for Id<T> {}

impl<T: Intern> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T: Intern> Hash for Id<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

/// Stores interned values of type.
pub struct Interner<T: Intern>(RwLock<InternerInner<T>>);

impl<T: Intern> Interner<T> {
    /// Creates a new, empty interner.
    pub const fn new() -> Self {
        Self(RwLock::new(InternerInner {
            to_id: HashMap::with_hasher(FxBuildHasher),
            from_id: Vec::new(),
        }))
    }

    fn intern(&self, value: T) -> Id<T> {
        // Try to find an existing entry that we can reuse.
        //
        // We could check with just a read lock, but if the pair is not yet
        // present, we would then need to recheck after acquiring a write lock,
        // which is probably not worth it.
        let mut inner = self.0.write().unwrap();
        if let Some(&id) = inner.to_id.get(&value) {
            return id;
        }
        inner.alloc(value, false)
    }

    fn intern_unique(&self, value: T) -> Id<T> {
        self.0.write().unwrap().alloc(value, true)
    }

    #[track_caller]
    fn get(&self, id: Id<T>) -> &'static T {
        self.0.read().unwrap().get(id)
    }
}

impl<T: Intern> Default for Interner<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// The internal representation of an [`Interner`].
struct InternerInner<T: Intern> {
    to_id: FxHashMap<&'static T, Id<T>>,
    from_id: Vec<&'static T>,
}

impl<T: Intern> InternerInner<T> {
    fn alloc(&mut self, value: T, unique: bool) -> Id<T> {
        let n = T::Index::from_usize(self.from_id.len() + 1).expect("out of file ids");
        let id = Id(n);

        // Create a new entry forever by leaking the pair. We can't leak more
        // values than the index type can differentiate.
        // TODO(perf): This could be optimized with an arena instead of a box.
        let leaked = Box::leak(Box::new(value));

        self.from_id.push(leaked);
        if !unique {
            self.to_id.insert(leaked, id);
        }

        id
    }

    fn get(&self, id: Id<T>) -> &'static T {
        self.from_id[id.0.to_usize() - 1]
    }
}
