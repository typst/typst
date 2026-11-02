use std::iter::{Chain, Flatten, Rev};

use smallvec::SmallVec;

/// Extra methods for [`[T]`](slice).
pub trait SliceExt<T> {
    /// Returns a slice with all matching elements from the start of the slice
    /// removed.
    fn trim_start_matches<F>(&self, f: F) -> &[T]
    where
        F: FnMut(&T) -> bool;

    /// Returns a slice with all matching elements from the end of the slice
    /// removed.
    fn trim_end_matches<F>(&self, f: F) -> &[T]
    where
        F: FnMut(&T) -> bool;

    /// Split a slice into consecutive runs with the same key and yield for
    /// each such run the key and the slice of elements with that key.
    fn group_by_key<K, F>(&self, f: F) -> GroupByKey<'_, T, F>
    where
        F: FnMut(&T) -> K,
        K: PartialEq;

    /// Computes two indices which split a slice into three parts.
    ///
    /// - A prefix which matches `f`
    /// - An inner portion
    /// - A suffix which matches `f` and does not overlap with the prefix
    ///
    /// If all elements match `f`, the prefix becomes `self` and the suffix
    /// will be empty.
    ///
    /// Returns the indices at which the inner portion and the suffix start.
    fn split_prefix_suffix<F>(&self, f: F) -> (usize, usize)
    where
        F: FnMut(&T) -> bool;
}

impl<T> SliceExt<T> for [T] {
    fn trim_start_matches<F>(&self, mut f: F) -> &[T]
    where
        F: FnMut(&T) -> bool,
    {
        let len = self.len();
        let mut i = 0;
        while i < len && f(&self[i]) {
            i += 1;
        }
        &self[i..]
    }

    fn trim_end_matches<F>(&self, mut f: F) -> &[T]
    where
        F: FnMut(&T) -> bool,
    {
        let mut i = self.len();
        while i > 0 && f(&self[i - 1]) {
            i -= 1;
        }
        &self[..i]
    }

    fn group_by_key<K, F>(&self, f: F) -> GroupByKey<'_, T, F> {
        GroupByKey { slice: self, f }
    }

    fn split_prefix_suffix<F>(&self, mut f: F) -> (usize, usize)
    where
        F: FnMut(&T) -> bool,
    {
        let start = self.iter().position(|v| !f(v)).unwrap_or(self.len());
        let end = self
            .iter()
            .skip(start)
            .rposition(|v| !f(v))
            .map_or(start, |i| start + i + 1);
        (start, end)
    }
}

/// A variant of `dedup` that keeps the later value rather than the earlier one.
pub trait Rdedup {
    type Item;

    /// Deduplicates values in a sorted sequence using a key function, but
    /// unlike the standard version keeps the later one.
    fn rdedup_by_key<K, F>(&mut self, key: F)
    where
        F: Fn(&mut Self::Item) -> K,
        K: PartialEq<K>;
}

impl<T: Copy, const N: usize> Rdedup for SmallVec<[T; N]> {
    type Item = T;

    fn rdedup_by_key<K, F>(&mut self, mut key: F)
    where
        T: Copy,
        K: PartialEq<K>,
        F: FnMut(&mut T) -> K,
    {
        let mut k = 0;
        for i in 1..self.len() {
            if key(&mut self[i]) != key(&mut self[k]) {
                k += 1;
            }
            if k < i {
                self[k] = self[i];
            }
        }
        self.truncate(k + 1);
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

/// Adapter for reversing iterators conditionally.
pub trait MaybeReverseIter {
    type RevIfIter;

    /// Reverse this iterator (apply `.rev()`) based on some condition.
    fn rev_if(self, condition: bool) -> Self::RevIfIter
    where
        Self: Sized;
}

impl<I: Iterator + DoubleEndedIterator> MaybeReverseIter for I {
    type RevIfIter =
        Chain<Flatten<std::option::IntoIter<I>>, Flatten<std::option::IntoIter<Rev<I>>>>;

    fn rev_if(self, condition: bool) -> Self::RevIfIter
    where
        Self: Sized,
    {
        let (maybe_self_iter, maybe_rev_iter) =
            if condition { (None, Some(self.rev())) } else { (Some(self), None) };

        maybe_self_iter
            .into_iter()
            .flatten()
            .chain(maybe_rev_iter.into_iter().flatten())
    }
}

#[cfg(test)]
mod tests {
    use smallvec::SmallVec;

    use super::*;

    #[test]
    fn test_rdedup() {
        #[track_caller]
        fn test(given: &[(char, i32)], expected: &[(char, i32)]) {
            let mut vec: SmallVec<[(char, i32); 2]> = given.into();
            vec.rdedup_by_key(|&mut (c, _)| c);
            assert_eq!(vec.as_slice(), expected);
        }

        test(&[], &[]);
        test(&[('a', 1), ('a', 2), ('a', 3), ('b', 2)], &[('a', 3), ('b', 2)]);
        test(&[('b', 2), ('c', 3), ('c', 4)], &[('b', 2), ('c', 4)]);
        test(
            &[('a', 1), ('b', 1), ('c', 1), ('c', 2), ('d', 1)],
            &[('a', 1), ('b', 1), ('c', 2), ('d', 1)],
        );
    }
}
