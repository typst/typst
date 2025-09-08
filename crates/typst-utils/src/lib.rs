//! Utilities for Typst.

pub mod fat;

#[macro_use]
mod macros;
mod bitset;
mod deferred;
mod duration;
mod hash;
mod listset;
mod pico;
mod round;
mod scalar;

pub use self::bitset::{BitSet, SmallBitSet};
pub use self::deferred::Deferred;
pub use self::duration::format_duration;
pub use self::hash::{HashLock, LazyHash, ManuallyHash};
pub use self::listset::ListSet;
pub use self::pico::{PicoStr, ResolvedPicoStr};
pub use self::round::{round_int_with_precision, round_with_precision};
pub use self::scalar::Scalar;

#[doc(hidden)]
pub use once_cell;

use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;
use std::iter::{Chain, Flatten, Rev};
use std::num::{NonZeroU32, NonZeroUsize};
use std::ops::{Add, Deref, Div, Mul, Neg, Sub};
use std::sync::Arc;

use siphasher::sip128::{Hasher128, SipHasher13};
use unicode_math_class::MathClass;

/// Turn a closure into a struct implementing [`Debug`].
pub fn debug<F>(f: F) -> impl Debug
where
    F: Fn(&mut Formatter) -> std::fmt::Result,
{
    struct Wrapper<F>(F);

    impl<F> Debug for Wrapper<F>
    where
        F: Fn(&mut Formatter) -> std::fmt::Result,
    {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            self.0(f)
        }
    }

    Wrapper(f)
}

/// Turn a closure into a struct implementing [`Display`].
pub fn display<F>(f: F) -> impl Display
where
    F: Fn(&mut Formatter) -> std::fmt::Result,
{
    struct Wrapper<F>(F);

    impl<F> Display for Wrapper<F>
    where
        F: Fn(&mut Formatter) -> std::fmt::Result,
    {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
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
    const ONE: Self = Self::new(1).unwrap();
}

impl NonZeroExt for NonZeroU32 {
    const ONE: Self = Self::new(1).unwrap();
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

/// Extra methods for [`Option`].
pub trait OptionExt<T> {
    /// Maps an `Option<T>` to `U` by applying a function to a contained value
    /// (if `Some`) or returns a default (if `None`).
    fn map_or_default<U: Default, F>(self, f: F) -> U
    where
        F: FnOnce(T) -> U;
}

impl<T> OptionExt<T> for Option<T> {
    fn map_or_default<U: Default, F>(self, f: F) -> U
    where
        F: FnOnce(T) -> U,
    {
        match self {
            Some(x) => f(x),
            None => U::default(),
        }
    }
}

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

    /// Reverse this iterator (apply .rev()) based on some condition.
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

/// Check if the [`Option`]-wrapped L is same to R.
pub fn option_eq<L, R>(left: Option<L>, other: R) -> bool
where
    L: PartialEq<R>,
{
    left.is_some_and(|v| v == other)
}

/// A container around a static reference that is cheap to clone and hash.
#[derive(Debug)]
pub struct Static<T: 'static>(pub &'static T);

impl<T> Deref for Static<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<T> Copy for Static<T> {}

impl<T> Clone for Static<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Eq for Static<T> {}

impl<T> PartialEq for Static<T> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0, other.0)
    }
}

impl<T> Hash for Static<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_usize(self.0 as *const _ as _);
    }
}

/// Generic access to a structure's components.
pub trait Get<Index> {
    /// The structure's component type.
    type Component;

    /// Borrow the component for the specified index.
    fn get_ref(&self, index: Index) -> &Self::Component;

    /// Borrow the component for the specified index mutably.
    fn get_mut(&mut self, index: Index) -> &mut Self::Component;

    /// Convenience method for getting a copy of a component.
    fn get(self, index: Index) -> Self::Component
    where
        Self: Sized,
        Self::Component: Copy,
    {
        *self.get_ref(index)
    }

    /// Convenience method for setting a component.
    fn set(&mut self, index: Index, component: Self::Component) {
        *self.get_mut(index) = component;
    }

    /// Builder-style method for setting a component.
    fn with(mut self, index: Index, component: Self::Component) -> Self
    where
        Self: Sized,
    {
        self.set(index, component);
        self
    }
}

/// A numeric type.
pub trait Numeric:
    Sized
    + Debug
    + Copy
    + PartialEq
    + Neg<Output = Self>
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<f64, Output = Self>
    + Div<f64, Output = Self>
{
    /// The identity element for addition.
    fn zero() -> Self;

    /// Whether `self` is zero.
    fn is_zero(self) -> bool {
        self == Self::zero()
    }

    /// Whether `self` consists only of finite parts.
    fn is_finite(self) -> bool;
}

/// Returns the default math class of a character in Typst, if it has one.
///
/// This is determined by the Unicode math class, with some manual overrides.
pub fn default_math_class(c: char) -> Option<MathClass> {
    match c {
        // Better spacing.
        // https://github.com/typst/typst/commit/2e039cb052fcb768027053cbf02ce396f6d7a6be
        ':' => Some(MathClass::Relation),

        // Better spacing when used alongside + PLUS SIGN.
        // https://github.com/typst/typst/pull/1726
        '⋯' | '⋱' | '⋰' | '⋮' => Some(MathClass::Normal),

        // Better spacing.
        // https://github.com/typst/typst/pull/1855
        '.' | '/' => Some(MathClass::Normal),

        // ⊥ UP TACK should not be a relation, contrary to ⟂ PERPENDICULAR.
        // https://github.com/typst/typst/pull/5714
        '\u{22A5}' => Some(MathClass::Normal),

        // Used as a binary connector in linear logic, where it is referred to
        // as "par".
        // https://github.com/typst/typst/issues/5764
        '⅋' => Some(MathClass::Binary),

        // Those overrides should become the default in the next revision of
        // MathClass.txt.
        // https://github.com/typst/typst/issues/5764#issuecomment-2632435247
        '⎰' | '⟅' => Some(MathClass::Opening),
        '⎱' | '⟆' => Some(MathClass::Closing),

        // Both ∨ and ⟑ are classified as Binary.
        // https://github.com/typst/typst/issues/5764
        '⟇' => Some(MathClass::Binary),

        // Arabic comma.
        // https://github.com/latex3/unicode-math/pull/633#issuecomment-2028936135
        '،' => Some(MathClass::Punctuation),

        c => unicode_math_class::class(c),
    }
}
