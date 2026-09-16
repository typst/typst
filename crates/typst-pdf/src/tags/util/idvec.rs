use std::marker::PhantomData;

#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct IdVec<T> {
    inner: Vec<T>,
}

impl<T> IdVec<T> {
    pub const fn new() -> Self {
        Self { inner: Vec::new() }
    }

    pub fn next_id(&self) -> Id<T> {
        Id::new(self.inner.len() as u32)
    }

    pub fn push(&mut self, val: T) -> Id<T> {
        let id = self.next_id();
        self.inner.push(val);
        id
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn get(&self, id: Id<T>) -> &T {
        &self.inner[id.idx()]
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn get_mut(&mut self, id: Id<T>) -> &mut T {
        &mut self.inner[id.idx()]
    }

    #[allow(unused)]
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.inner.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
        self.inner.iter_mut()
    }

    pub fn ids(
        &self,
    ) -> impl ExactSizeIterator<Item = Id<T>> + DoubleEndedIterator + use<T> {
        (0..self.inner.len()).map(|i| Id::new(i as u32))
    }
}

/// A strongly typed ID.
pub struct Id<T> {
    id: u32,
    _ty: PhantomData<T>,
}

impl<T> Id<T> {
    #[inline]
    pub const fn new(id: u32) -> Self {
        Self { id, _ty: PhantomData::<T> }
    }

    #[inline]
    pub const fn get(self) -> u32 {
        self.id
    }

    #[inline]
    pub const fn idx(self) -> usize {
        self.id as usize
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
