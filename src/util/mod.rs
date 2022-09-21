//! Utilities.

#[macro_use]
mod eco;
mod buffer;

pub use buffer::Buffer;
pub use eco::EcoString;

use std::any::TypeId;
use std::fmt::{self, Debug, Formatter};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

/// Turn a closure into a struct implementing [`Debug`].
pub fn debug<F>(f: F) -> impl Debug
where
    F: Fn(&mut Formatter) -> fmt::Result,
{
    struct Wrapper<F>(F);

    impl<F> Debug for Wrapper<F>
    where
        F: Fn(&mut Formatter) -> fmt::Result,
    {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            self.0(f)
        }
    }

    Wrapper(f)
}

/// Extra methods for [`str`].
pub trait StrExt {
    /// The number of code units this string would use if it was encoded in
    /// UTF16. This runs in linear time.
    fn len_utf16(&self) -> usize;
}

impl StrExt for str {
    fn len_utf16(&self) -> usize {
        self.chars().map(char::len_utf16).sum()
    }
}

/// Extra methods for [`Arc`].
pub trait ArcExt<T> {
    /// Takes the inner value if there is exactly one strong reference and
    /// clones it otherwise.
    fn take(self) -> T;
}

impl<T> ArcExt<T> for Arc<T>
where
    T: Clone,
{
    fn take(self) -> T {
        match Arc::try_unwrap(self) {
            Ok(v) => v,
            Err(rc) => (*rc).clone(),
        }
    }
}

/// Extra methods for [`[T]`](slice).
pub trait SliceExt<T> {
    /// Split a slice into consecutive runs with the same key and yield for
    /// each such run the key and the slice of elements with that key.
    fn group_by_key<K, F>(&self, f: F) -> GroupByKey<'_, T, F>
    where
        F: FnMut(&T) -> K,
        K: PartialEq;
}

impl<T> SliceExt<T> for [T] {
    fn group_by_key<K, F>(&self, f: F) -> GroupByKey<'_, T, F> {
        GroupByKey { slice: self, f }
    }
}

/// This struct is created by [`SliceExt::group_by_key`].
pub struct GroupByKey<'a, T, F> {
    slice: &'a [T],
    f: F,
}

impl<'a, T, K, F> Iterator for GroupByKey<'a, T, F>
where
    F: FnMut(&T) -> K,
    K: PartialEq,
{
    type Item = (K, &'a [T]);

    fn next(&mut self) -> Option<Self::Item> {
        let mut iter = self.slice.iter();
        let key = (self.f)(iter.next()?);
        let count = 1 + iter.take_while(|t| (self.f)(t) == key).count();
        let (head, tail) = self.slice.split_at(count);
        self.slice = tail;
        Some((key, head))
    }
}

/// Extra methods for [`Path`].
pub trait PathExt {
    /// Lexically normalize a path.
    fn normalize(&self) -> PathBuf;
}

impl PathExt for Path {
    fn normalize(&self) -> PathBuf {
        let mut out = PathBuf::new();
        for component in self.components() {
            match component {
                Component::CurDir => {}
                Component::ParentDir => match out.components().next_back() {
                    Some(Component::Normal(_)) => {
                        out.pop();
                    }
                    _ => out.push(component),
                },
                _ => out.push(component),
            }
        }
        out
    }
}

/// An alternative type id that prints as something readable in debug mode.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct ReadableTypeId {
    id: TypeId,
    #[cfg(debug_assertions)]
    name: &'static str,
}

impl ReadableTypeId {
    /// The type id of the given type.
    pub fn of<T: 'static>() -> Self {
        Self {
            id: TypeId::of::<T>(),
            #[cfg(debug_assertions)]
            name: std::any::type_name::<T>(),
        }
    }
}

impl Debug for ReadableTypeId {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        #[cfg(debug_assertions)]
        f.pad(self.name)?;
        #[cfg(not(debug_assertions))]
        f.pad("ReadableTypeId")?;
        Ok(())
    }
}
