//! Utilities.

#[macro_use]
mod eco_string;
mod mac_roman;
mod prehashed;

pub use eco_string::EcoString;
pub use mac_roman::decode_mac_roman;
pub use prehashed::Prehashed;

use std::cmp::Ordering;
use std::fmt::{self, Debug, Formatter};
use std::ops::{Deref, Range};
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

/// Additional methods for strings.
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

/// Additional methods for options.
pub trait OptionExt<T> {
    /// Sets `other` as the value if `self` is `None` or if it contains a value
    /// larger than `other`.
    fn set_min(&mut self, other: T)
    where
        T: Ord;

    /// Sets `other` as the value if `self` is `None` or if it contains a value
    /// smaller than `other`.
    fn set_max(&mut self, other: T)
    where
        T: Ord;
}

impl<T> OptionExt<T> for Option<T> {
    fn set_min(&mut self, other: T)
    where
        T: Ord,
    {
        if self.as_ref().map_or(true, |x| other < *x) {
            *self = Some(other);
        }
    }

    fn set_max(&mut self, other: T)
    where
        T: Ord,
    {
        if self.as_ref().map_or(true, |x| other > *x) {
            *self = Some(other);
        }
    }
}

/// Additional methods for reference-counted pointers.
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

/// Either owned or shared.
pub enum MaybeShared<T> {
    /// Owned data.
    Owned(T),
    /// Shared data.
    Shared(Arc<T>),
}

impl<T> AsRef<T> for MaybeShared<T> {
    fn as_ref(&self) -> &T {
        self
    }
}

impl<T> Deref for MaybeShared<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Owned(owned) => owned,
            Self::Shared(shared) => shared,
        }
    }
}

/// Additional methods for slices.
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

/// Additional methods for [`Range<usize>`].
pub trait RangeExt {
    /// Locate a position relative to a range.
    ///
    /// This can be used for binary searching the range that contains the
    /// position as follows:
    /// ```
    /// # use typst::util::RangeExt;
    /// assert_eq!(
    ///     [1..2, 2..7, 7..10].binary_search_by(|r| r.locate(5)),
    ///     Ok(1),
    /// );
    /// ```
    fn locate(&self, pos: usize) -> Ordering;
}

impl RangeExt for Range<usize> {
    fn locate(&self, pos: usize) -> Ordering {
        if pos < self.start {
            Ordering::Greater
        } else if pos < self.end {
            Ordering::Equal
        } else {
            Ordering::Less
        }
    }
}

/// Additional methods for [`Path`].
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
