use std::fmt::Debug;
use std::num::{NonZeroU32, NonZeroUsize};
use std::ops::{Add, Div, Mul, Neg, Sub};

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

/// A marker trait for numeric lengths.
pub trait NumericLength: Numeric {}
