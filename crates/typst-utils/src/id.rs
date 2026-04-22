use std::hash::Hash;
use std::marker::PhantomData;
use std::num::NonZeroU32;

use indexmap::{Equivalent, IndexMap};
use rustc_hash::FxBuildHasher;

/// Specify alternative types that are allowed as a generic [`Id`] tag, to
/// index a collection of `V`.
pub trait KeyFor<U> {}

impl<U> KeyFor<U> for U {}

/// A strongly typed ID.
pub struct Id<T> {
    id: NonZeroU32,
    ty: PhantomData<T>,
}

impl<T> Id<T> {
    #[inline]
    pub const fn new(idx: usize) -> Self {
        Self {
            id: NonZeroU32::new(idx as u32 + 1).unwrap(),
            ty: PhantomData::<T>,
        }
    }

    #[inline]
    pub const fn idx(self) -> usize {
        self.id.get() as usize - 1
    }

    /// Cast to a compatible ID.
    #[inline]
    pub const fn downcast<U: KeyFor<T>>(self) -> Id<U> {
        Id::new(self.idx())
    }

    /// Cast to a compatible ID.
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

/// An end-inclusive [`Id`] range.
pub struct IdRange<I> {
    start: I,
    /// Inclusive end-index.
    end: I,
}

impl<T> IdRange<Id<T>> {
    pub fn new<U: KeyFor<T>>(id: Id<U>) -> Self {
        Self { start: id.upcast(), end: id.upcast() }
    }

    pub fn include<U: KeyFor<T>>(&mut self, id: Id<U>) {
        self.end = id.upcast();
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
    pub const fn new() -> Self {
        Self { inner: Vec::new() }
    }

    pub fn next_id<U>(&self) -> Id<U>
    where
        U: KeyFor<T>,
    {
        Id::new(self.inner.len())
    }

    pub fn push(&mut self, val: T) -> Id<T> {
        let id = self.next_id();
        self.inner.push(val);
        id
    }
}

impl<T> IdVec<T> {
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn get<U>(&self, id: Id<U>) -> &T
    where
        U: KeyFor<T>,
    {
        &self.inner[id.idx()]
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn get_mut<U>(&mut self, id: Id<U>) -> &mut T
    where
        U: KeyFor<T>,
    {
        &mut self.inner[id.idx()]
    }

    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.inner.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
        self.inner.iter_mut()
    }

    pub fn id_iter(
        &self,
    ) -> impl ExactSizeIterator<Item = (Id<T>, &T)> + DoubleEndedIterator {
        self.ids().zip(self.inner.iter())
    }

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

/// a wrapper over [`indexmap`] that allows retrieving elements by stable
/// [`id`]s.
#[derive(Debug, Clone)]
pub struct IdMap<K, V> {
    inner: IndexMap<K, V, FxBuildHasher>,
}

impl<K, V> Default for IdMap<K, V> {
    fn default() -> Self {
        Self { inner: Default::default() }
    }
}

impl<K, V> IdMap<K, V> {
    pub fn new() -> Self {
        Self { inner: IndexMap::default() }
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn next_id<U>(&self) -> Id<U>
    where
        U: KeyFor<V>,
    {
        Id::new(self.inner.len())
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn get_id_full<U>(&self, id: Id<U>) -> (&K, &V)
    where
        U: KeyFor<V>,
    {
        self.inner.get_index(id.idx()).unwrap()
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn get_id<U>(&self, id: Id<U>) -> &V
    where
        U: KeyFor<V>,
    {
        &self.inner[id.idx()]
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn get_id_mut<U>(&mut self, id: Id<U>) -> &mut V
    where
        U: KeyFor<V>,
    {
        &mut self.inner[id.idx()]
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn lookup_id<Q, U>(&self, key: &Q) -> Option<Id<U>>
    where
        Q: ?Sized + Hash + Equivalent<K>,
        U: KeyFor<V>,
    {
        let idx = self.inner.get_index_of(key)?;
        Some(Id::new(idx))
    }

    pub fn ids(
        &self,
    ) -> impl ExactSizeIterator<Item = Id<V>> + DoubleEndedIterator + use<K, V> {
        (0..self.inner.len()).map(|i| Id::new(i))
    }

    pub fn iter(&self) -> indexmap::map::Iter<'_, K, V> {
        self.inner.iter()
    }

    pub fn iter_mut(&mut self) -> indexmap::map::IterMut<'_, K, V> {
        self.inner.iter_mut()
    }

    pub fn values(&self) -> indexmap::map::Values<'_, K, V> {
        self.inner.values()
    }

    pub fn id_iter(
        &self,
    ) -> impl ExactSizeIterator<Item = (Id<V>, &K, &V)> + DoubleEndedIterator {
        self.ids()
            .zip(self.inner.iter())
            .map(|(id, (key, val))| (id, key, val))
    }

    pub fn into_inner(self) -> IndexMap<K, V, FxBuildHasher> {
        self.inner
    }
}

impl<K, V> IdMap<K, V>
where
    K: Hash + Eq,
{
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.inner.insert(key, value)
    }

    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        Q: ?Sized + Hash + Equivalent<K>,
    {
        self.inner.get(key)
    }

    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        Q: ?Sized + Hash + Equivalent<K>,
    {
        self.inner.get_mut(key)
    }

    pub fn entry(&mut self, key: K) -> Entry<'_, K, V> {
        Entry { inner: self.inner.entry(key) }
    }
}

pub struct Entry<'a, K, V> {
    inner: indexmap::map::Entry<'a, K, V>,
}

impl<'a, K, V> Entry<'a, K, V> {
    pub fn or_default(self) -> &'a mut V
    where
        V: Default,
    {
        self.inner.or_default()
    }

    pub fn id<U>(&self) -> Id<U>
    where
        U: KeyFor<V>,
    {
        Id::new(self.inner.index())
    }

    pub fn is_vacant(&self) -> bool {
        matches!(self.inner, indexmap::map::Entry::Vacant(_))
    }
}

impl<K, V> FromIterator<(K, V)> for IdMap<K, V>
where
    K: Hash + Eq,
{
    /// Create an `IndexMap` from the sequence of key-value pairs in the
    /// iterable.
    ///
    /// `from_iter` uses the same logic as `extend`. See
    /// [`extend`][IndexMap::extend] for more details.
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iterable: I) -> Self {
        Self { inner: IndexMap::from_iter(iterable) }
    }
}
