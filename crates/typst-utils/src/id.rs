use std::hash::Hash;
use std::marker::PhantomData;
use std::num::NonZeroU32;

use indexmap::{Equivalent, IndexMap};
use rustc_hash::FxBuildHasher;

/// Specify alternative types that are allowed as a generic [`Id`] tag, to
/// index a collection of `U`.
pub trait KeyFor<U> {}

impl<U> KeyFor<U> for U {}

/// A strongly typed ID.
pub struct Id<T> {
    id: NonZeroU32,
    ty: PhantomData<T>,
}

impl<T> Id<T> {
    /// Create a new ID from an index.
    #[inline]
    pub const fn new(idx: usize) -> Self {
        Self {
            id: NonZeroU32::new(idx as u32 + 1).unwrap(),
            ty: PhantomData::<T>,
        }
    }

    /// The underlying index this ID represents.
    #[inline]
    pub const fn idx(self) -> usize {
        self.id.get() as usize - 1
    }

    /// Cast to a compatible ID.
    /// Downcasting may be invalid.
    #[inline]
    pub const fn downcast<U: KeyFor<T>>(self) -> Id<U> {
        Id::new(self.idx())
    }

    /// Cast to a compatible ID.
    /// Upcasting is always valid.
    #[inline]
    pub const fn upcast<U>(self) -> Id<U>
    where
        T: KeyFor<U>,
    {
        Id::new(self.idx())
    }
}

impl<T> Copy for Id<T> {}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> std::fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let full_name = std::any::type_name::<T>();
        let start = full_name.rfind("::").map(|i| i + 2).unwrap_or(0);
        let short_name = &full_name[start..];
        write!(f, "Id::<{}>({})", short_name, self.id)
    }
}

impl<T> Eq for Id<T> {}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T> Ord for Id<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

impl<T> PartialOrd for Id<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> std::hash::Hash for Id<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

/// An end-exclusive [`Id`] range.
pub struct IdRange<I> {
    pub start: I,
    pub end: I,
}

impl<T> IdRange<Id<T>> {
    /// Creates a new range.
    pub fn new(start: Id<T>, end: Id<T>) -> Self {
        Self { start, end }
    }

    /// Creates an empty range directly after the [`Id`].
    pub fn after(id: Id<T>) -> Self {
        Self::at(Id::new(id.idx() + 1))
    }

    /// Creates an empty range at the [`Id`].
    pub fn at(id: Id<T>) -> Self {
        Self::new(id, id)
    }

    /// The length of the range.
    pub fn len(&self) -> usize {
        self.end.idx() - self.start.idx()
    }

    /// Whether this range is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Whether this range contains the id.
    pub fn contains(self, id: Id<T>) -> bool {
        (self.start..self.end).contains(&id)
    }

    /// Returns the range of the [`Id::idx`] indices.
    pub fn idx(self) -> std::ops::Range<usize> {
        self.start.idx()..self.end.idx()
    }
}

impl<T> Copy for IdRange<Id<T>> {}

impl<T> Clone for IdRange<Id<T>> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> std::fmt::Debug for IdRange<Id<T>> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let full_name = std::any::type_name::<T>();
        let start = full_name.rfind("::").map(|i| i + 2).unwrap_or(0);
        let short_name = &full_name[start..];
        write!(f, "IdRange::<{}>({}..={})", short_name, self.start.idx(), self.end.idx())
    }
}

impl<T> Eq for IdRange<Id<T>> {}

impl<T> PartialEq for IdRange<Id<T>> {
    fn eq(&self, other: &Self) -> bool {
        self.start == other.start && self.end == other.end
    }
}

impl<T> std::hash::Hash for IdRange<Id<T>> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.start.hash(state);
        self.end.hash(state);
    }
}

/// An append-only wrapper over [`Vec`] that allows retrieving elements by a
/// stable [`Id`].
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct IdVec<T> {
    inner: Vec<T>,
}

impl<T> Default for IdVec<T> {
    fn default() -> Self {
        Self { inner: Default::default() }
    }
}

impl<T> IdVec<T> {
    /// Create a new ID vector.
    pub const fn new() -> Self {
        Self { inner: Vec::new() }
    }

    /// Create a new ID vector with the given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self { inner: Vec::with_capacity(capacity) }
    }

    /// The length of the vector.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether the vector is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns the ID that will be assigned to the next inserted element.
    pub fn next_id<U>(&self) -> Id<U>
    where
        U: KeyFor<T>,
    {
        Id::new(self.inner.len())
    }

    /// Insert and element and return its ID.
    pub fn push(&mut self, val: T) -> Id<T> {
        let id = self.next_id();
        self.inner.push(val);
        id
    }
}

impl<T> IdVec<T> {
    /// Retrieve an element by ID.
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn get<U>(&self, id: Id<U>) -> &T
    where
        U: KeyFor<T>,
    {
        &self.inner[id.idx()]
    }

    /// Retrieve an element by ID.
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn get_mut<U>(&mut self, id: Id<U>) -> &mut T
    where
        U: KeyFor<T>,
    {
        &mut self.inner[id.idx()]
    }

    /// Retrieve a slice of elements by ID.
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn get_range<U>(&self, ids: IdRange<Id<U>>) -> &[T]
    where
        U: KeyFor<T>,
    {
        &self.inner[ids.idx()]
    }

    /// Returns an iterator over the elements.
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.inner.iter()
    }

    /// Returns an iterator over the elements.
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
        self.inner.iter_mut()
    }

    /// Returns an iterator over both IDs and elements.
    pub fn id_iter(
        &self,
    ) -> impl ExactSizeIterator<Item = (Id<T>, &T)> + DoubleEndedIterator {
        self.ids().zip(self.inner.iter())
    }

    /// Returns an iterator over both IDs and elements.
    pub fn id_iter_mut(
        &mut self,
    ) -> impl ExactSizeIterator<Item = (Id<T>, &mut T)> + DoubleEndedIterator {
        self.ids().zip(self.inner.iter_mut())
    }

    /// Returns an iterator over IDs of the elements.
    pub fn ids(
        &self,
    ) -> impl ExactSizeIterator<Item = Id<T>> + DoubleEndedIterator + use<T> {
        (0..self.inner.len()).map(|i| Id::new(i))
    }
}

impl<T> FromIterator<T> for IdVec<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let inner = Vec::from_iter(iter);
        Self { inner }
    }
}

/// An wrapper over [`IndexMap`] that allows retrieving elements by a stable
/// [`Id`].
#[derive(Debug, Clone)]
pub struct IdMap<K, V> {
    inner: IndexMap<K, V, FxBuildHasher>,
}

impl<K, V> Default for IdMap<K, V> {
    fn default() -> Self {
        Self { inner: Default::default() }
    }
}

impl<K, V> From<IndexMap<K, V, FxBuildHasher>> for IdMap<K, V> {
    fn from(inner: IndexMap<K, V, FxBuildHasher>) -> Self {
        Self { inner }
    }
}

impl<K, V> IdMap<K, V> {
    /// Create a new ID map.
    pub fn new() -> Self {
        Self { inner: IndexMap::default() }
    }

    /// The length of the map.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether the map is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns the ID that will be assigned to the next newly inserted element.
    pub fn next_id<U>(&self) -> Id<U>
    where
        U: KeyFor<V>,
    {
        Id::new(self.inner.len())
    }

    /// Retrieve both the key and value by ID.
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn get_id_full<U>(&self, id: Id<U>) -> (&K, &V)
    where
        U: KeyFor<V>,
    {
        self.inner.get_index(id.idx()).unwrap()
    }

    /// Retrieve the key by ID.
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn get_id_key<U>(&self, id: Id<U>) -> &K
    where
        U: KeyFor<V>,
    {
        self.inner.get_index(id.idx()).unwrap().0
    }

    /// Retrieve the value by ID.
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn get_id<U>(&self, id: Id<U>) -> &V
    where
        U: KeyFor<V>,
    {
        &self.inner[id.idx()]
    }

    /// Retrieve the value by ID.
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn get_id_mut<U>(&mut self, id: Id<U>) -> &mut V
    where
        U: KeyFor<V>,
    {
        &mut self.inner[id.idx()]
    }

    /// Returns an iterator over both keys and values.
    pub fn iter(&self) -> indexmap::map::Iter<'_, K, V> {
        self.inner.iter()
    }

    /// Returns an iterator over both keys and values.
    pub fn iter_mut(&mut self) -> indexmap::map::IterMut<'_, K, V> {
        self.inner.iter_mut()
    }

    /// Returns an iterator over just the values.
    pub fn values(&self) -> indexmap::map::Values<'_, K, V> {
        self.inner.values()
    }

    /// Returns an iterator over IDs, keys and values.
    pub fn id_iter(
        &self,
    ) -> impl ExactSizeIterator<Item = (Id<V>, &K, &V)> + DoubleEndedIterator {
        self.ids()
            .zip(self.inner.iter())
            .map(|(id, (key, val))| (id, key, val))
    }

    /// Returns an iterator over IDs of the entries.
    pub fn ids(
        &self,
    ) -> impl ExactSizeIterator<Item = Id<V>> + DoubleEndedIterator + use<K, V> {
        (0..self.inner.len()).map(|i| Id::new(i))
    }

    /// Returns the inner type.
    pub fn into_inner(self) -> IndexMap<K, V, FxBuildHasher> {
        self.inner
    }
}

impl<K, V> IdMap<K, V>
where
    K: Hash + Eq,
{
    /// Insert a key-value pair.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.inner.insert(key, value)
    }

    /// Lookup the ID for a key.
    pub fn lookup_id<Q, U>(&self, key: &Q) -> Option<Id<U>>
    where
        Q: ?Sized + Hash + Equivalent<K>,
        U: KeyFor<V>,
    {
        let idx = self.inner.get_index_of(key)?;
        Some(Id::new(idx))
    }

    /// Retrieve a value by its key.
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        Q: ?Sized + Hash + Equivalent<K>,
    {
        self.inner.get(key)
    }

    /// Retrieve a value by its key.
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        Q: ?Sized + Hash + Equivalent<K>,
    {
        self.inner.get_mut(key)
    }

    /// Retrieve the ID, key and value by its key.
    pub fn get_full<Q, U>(&self, key: &Q) -> Option<(Id<U>, &K, &V)>
    where
        Q: ?Sized + Hash + Equivalent<K>,
        U: KeyFor<V>,
    {
        let (idx, key, val) = self.inner.get_full(key)?;
        Some((Id::new(idx), key, val))
    }

    /// Retrieve the ID, key and value by its key.
    pub fn get_full_mut<Q, U>(&mut self, key: &Q) -> Option<(Id<U>, &K, &mut V)>
    where
        Q: ?Sized + Hash + Equivalent<K>,
        U: KeyFor<V>,
    {
        let (idx, key, val) = self.inner.get_full_mut(key)?;
        Some((Id::new(idx), key, val))
    }

    /// Whether the map contains the key.
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        Q: ?Sized + Hash + Equivalent<K>,
    {
        self.inner.contains_key(key)
    }

    /// Retrieve an entry for insertion and/or in-place manipulation.
    pub fn entry(&mut self, key: K) -> IdEntry<'_, K, V> {
        IdEntry { inner: self.inner.entry(key) }
    }
}

/// Entry for an existing of vacant key-value pair in an [`IdMap`].
pub struct IdEntry<'a, K, V> {
    inner: indexmap::map::Entry<'a, K, V>,
}

impl<'a, K, V> IdEntry<'a, K, V> {
    /// Insert a value if the entry is vacant.
    pub fn or_insert(self, default: V) -> &'a mut V {
        self.inner.or_insert(default)
    }

    /// Insert a value if the entry is vacant.
    pub fn or_insert_with<F>(self, call: F) -> &'a mut V
    where
        F: FnOnce() -> V,
    {
        self.inner.or_insert_with(call)
    }

    /// Insert a default value if the entry is vacant.
    pub fn or_default(self) -> &'a mut V
    where
        V: Default,
    {
        self.inner.or_default()
    }

    /// The ID of the entry.
    pub fn id<U>(&self) -> Id<U>
    where
        U: KeyFor<V>,
    {
        Id::new(self.inner.index())
    }

    /// Whether the entry belonging to this key is not yet present.
    pub fn is_vacant(&self) -> bool {
        matches!(self.inner, indexmap::map::Entry::Vacant(_))
    }
}

impl<K, V> FromIterator<(K, V)> for IdMap<K, V>
where
    K: Hash + Eq,
{
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iterable: I) -> Self {
        Self { inner: IndexMap::from_iter(iterable) }
    }
}
