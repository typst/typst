//! Utilities.

mod eco;

pub use eco::EcoString;

use std::cmp::Ordering;
use std::ops::Range;
use std::path::{Component, Path, PathBuf};

/// Additional methods for options.
pub trait OptionExt<T> {
    /// Replace `self` with `other` if `self` is `Some`.
    fn and_set(&mut self, other: Option<T>);

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
    fn and_set(&mut self, other: Option<T>) {
        if self.is_some() {
            *self = other;
        }
    }

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

/// Additional methods for slices.
pub trait SliceExt<T> {
    /// Split a slice into consecutive groups with the same key.
    ///
    /// Returns an iterator of pairs of a key and the group with that key.
    fn group_by_key<K, F>(&self, f: F) -> GroupByKey<'_, T, F>
    where
        F: FnMut(&T) -> K,
        K: PartialEq;
}

impl<T> SliceExt<T> for [T] {
    fn group_by_key<K, F>(&self, f: F) -> GroupByKey<'_, T, F>
    where
        F: FnMut(&T) -> K,
        K: PartialEq,
    {
        GroupByKey { slice: self, f }
    }
}

/// This struct is produced by [`SliceExt::group_by_key`].
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
        let first = self.slice.first()?;
        let key = (self.f)(first);

        let mut i = 1;
        while self.slice.get(i).map_or(false, |t| (self.f)(t) == key) {
            i += 1;
        }

        let (head, tail) = self.slice.split_at(i);
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
