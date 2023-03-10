//! Utilities.

pub mod fat;

mod buffer;

pub use buffer::Buffer;

use std::fmt::{self, Debug, Formatter};
use std::hash::Hash;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use ecow::EcoString;
use siphasher::sip128::{Hasher128, SipHasher};

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
    let mut state = SipHasher::new();
    value.hash(&mut state);
    state.finish128().as_u128()
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

/// Format something as a a comma-separated list that support horizontal
/// formatting but falls back to vertical formatting if the pieces are too long.
pub fn pretty_array(pieces: &[EcoString], trailing_comma: bool) -> String {
    let list = pretty_comma_list(&pieces, trailing_comma);
    let mut buf = String::new();
    buf.push('(');
    if list.contains('\n') {
        buf.push('\n');
        buf.push_str(&indent(&list, 2));
        buf.push('\n');
    } else {
        buf.push_str(&list);
    }
    buf.push(')');
    buf
}

/// Format something as a a comma-separated list that support horizontal
/// formatting but falls back to vertical formatting if the pieces are too long.
pub fn pretty_comma_list(pieces: &[EcoString], trailing_comma: bool) -> String {
    const MAX_WIDTH: usize = 50;

    let mut buf = String::new();
    let len = pieces.iter().map(|s| s.len()).sum::<usize>()
        + 2 * pieces.len().saturating_sub(1);

    if len <= MAX_WIDTH {
        for (i, piece) in pieces.iter().enumerate() {
            if i > 0 {
                buf.push_str(", ");
            }
            buf.push_str(piece);
        }
        if trailing_comma {
            buf.push(',');
        }
    } else {
        for piece in pieces {
            buf.push_str(piece.trim());
            buf.push_str(",\n");
        }
    }

    buf
}

/// Indent a string by two spaces.
pub fn indent(text: &str, amount: usize) -> String {
    let mut buf = String::new();
    for (i, line) in text.lines().enumerate() {
        if i > 0 {
            buf.push('\n');
        }
        for _ in 0..amount {
            buf.push(' ');
        }
        buf.push_str(line);
    }
    buf
}
