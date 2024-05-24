//! Utilities for Typst.

pub mod fat;

#[macro_use]
mod macros;
mod bitset;
mod deferred;
mod hash;
mod pico;
mod scalar;

pub use self::bitset::{BitSet, SmallBitSet};
pub use self::deferred::Deferred;
pub use self::hash::LazyHash;
pub use self::pico::PicoStr;
pub use self::scalar::Scalar;

use std::fmt::{Debug, Formatter};
use std::hash::Hash;
use std::iter::{Chain, Flatten, Rev};
use std::num::NonZeroUsize;
use std::ops::{Add, Deref, Div, Mul, Neg, Sub};
use std::sync::Arc;

use siphasher::sip128::{Hasher128, SipHasher13};

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

/// Round a float to two decimal places.
pub fn round_2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}
