//! Utilities.

pub mod fat;

use std::fmt::{self, Debug, Formatter};
use std::hash::Hash;
use std::num::NonZeroUsize;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use siphasher::sip128::{Hasher128, SipHasher13};

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

/// Calculate a 128-bit siphash of a value.
pub fn hash128<T: Hash + ?Sized>(value: &T) -> u128 {
    let mut state = SipHasher13::new();
    value.hash(&mut state);
    state.finish128().as_u128()
}

/// An extra constant for [`NonZeroUsize`].
pub trait NonZeroExt {
    /// The number `1`.
    const ONE: Self;
}

impl NonZeroExt for NonZeroUsize {
    const ONE: Self = match Self::new(1) {
        Some(v) => v,
        None => unreachable!(),
    };
}

/// Extra methods for [`Arc`].
pub trait ArcExt<T> {
    /// Takes the inner value if there is exactly one strong reference and
    /// clones it otherwise.
    fn take(self) -> T;
}

impl<T: Clone> ArcExt<T> for Arc<T> {
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
    /// Treat `self` as a virtual root relative to which the `path` is resolved.
    ///
    /// Returns `None` if the path lexically escapes the root. The path
    /// might still escape through symlinks.
    fn join_rooted(&self, path: &Path) -> Option<PathBuf>;
}

impl PathExt for Path {
    fn join_rooted(&self, path: &Path) -> Option<PathBuf> {
        let mut parts: Vec<_> = self.components().collect();
        let root = parts.len();
        for component in path.components() {
            match component {
                Component::Prefix(_) => return None,
                Component::RootDir => parts.truncate(root),
                Component::CurDir => {}
                Component::ParentDir => {
                    if parts.len() <= root {
                        return None;
                    }
                    parts.pop();
                }
                Component::Normal(_) => parts.push(component),
            }
        }
        if parts.len() < root {
            return None;
        }
        Some(parts.into_iter().collect())
    }
}

/// Format pieces separated with commas and a final "and" or "or".
pub fn separated_list(pieces: &[impl AsRef<str>], last: &str) -> String {
    let mut buf = String::new();
    for (i, part) in pieces.iter().enumerate() {
        match i {
            0 => {}
            1 if pieces.len() == 2 => {
                buf.push(' ');
                buf.push_str(last);
                buf.push(' ');
            }
            i if i + 1 == pieces.len() => {
                buf.push_str(", ");
                buf.push_str(last);
                buf.push(' ');
            }
            _ => buf.push_str(", "),
        }
        buf.push_str(part.as_ref());
    }
    buf
}

/// Format a comma-separated list.
///
/// Tries to format horizontally, but falls back to vertical formatting if the
/// pieces are too long.
pub fn pretty_comma_list(pieces: &[impl AsRef<str>], trailing_comma: bool) -> String {
    const MAX_WIDTH: usize = 50;

    let mut buf = String::new();
    let len = pieces.iter().map(|s| s.as_ref().len()).sum::<usize>()
        + 2 * pieces.len().saturating_sub(1);

    if len <= MAX_WIDTH {
        for (i, piece) in pieces.iter().enumerate() {
            if i > 0 {
                buf.push_str(", ");
            }
            buf.push_str(piece.as_ref());
        }
        if trailing_comma {
            buf.push(',');
        }
    } else {
        for piece in pieces {
            buf.push_str(piece.as_ref().trim());
            buf.push_str(",\n");
        }
    }

    buf
}

/// Format an array-like construct.
///
/// Tries to format horizontally, but falls back to vertical formatting if the
/// pieces are too long.
pub fn pretty_array_like(parts: &[impl AsRef<str>], trailing_comma: bool) -> String {
    let list = pretty_comma_list(parts, trailing_comma);
    let mut buf = String::new();
    buf.push('(');
    if list.contains('\n') {
        buf.push('\n');
        for (i, line) in list.lines().enumerate() {
            if i > 0 {
                buf.push('\n');
            }
            buf.push_str("  ");
            buf.push_str(line);
        }
        buf.push('\n');
    } else {
        buf.push_str(&list);
    }
    buf.push(')');
    buf
}

/// Check if the [`Option`]-wrapped L is same to R.
pub fn option_eq<L, R>(left: Option<L>, other: R) -> bool
where
    L: PartialEq<R>,
{
    left.map(|v| v == other).unwrap_or(false)
}
