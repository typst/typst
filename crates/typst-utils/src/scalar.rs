use std::cmp::Ordering;
use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::iter::Sum;
use std::ops::{Add, Div, Mul, Neg, Rem, Sub};

use crate::Numeric;

/// A 64-bit float that implements `Eq`, `Ord` and `Hash`.
///
/// Panics if it's `NaN` during any of those operations.
///
/// All operations implemented for this type are cross-platform deterministic.
#[derive(Default, Copy, Clone)]
pub struct Scalar(f64);

impl Scalar {
    /// The scalar containing `0.0`.
    pub const ZERO: Self = Self(0.0);

    /// The scalar containing `1.0`.
    pub const ONE: Self = Self(1.0);

    /// The scalar containing `f64::INFINITY`.
    pub const INFINITY: Self = Self(f64::INFINITY);

    /// Creates a [`Scalar`] with the given value.
    ///
    /// If the value is NaN, then it is set to `0.0` in the result.
    pub const fn new(x: f64) -> Self {
        Self(if x.is_nan() { 0.0 } else { x })
    }

    /// Gets the value of this [`Scalar`].
    pub const fn get(self) -> f64 {
        self.0
    }

    /// Returns the square root of this scalar.
    pub fn sqrt(self) -> Self {
        Self::new(self.get().sqrt())
    }

    /// Raises a number to an integer power.
    pub fn powi(self, mut b: i32) -> Self {
        // Ported from https://github.com/llvm/llvm-project/blob/0ee439b/compiler-rt/lib/builtins/powidf2.c
        // Copyright: The LLVM Project, under the Apache License v2.0 with LLVM Exceptions.
        // See NOTICE for full attribution.
        let mut a = self.get();
        let recip = b < 0;
        let mut r = 1.0;
        loop {
            if (b & 1) != 0 {
                r *= a;
            }
            b /= 2;
            if b == 0 {
                break;
            }
            a *= a;
        }

        if recip {
            r = 1.0 / r;
        }

        Self::new(r)
    }
}

impl Numeric for Scalar {
    fn zero() -> Self {
        Self(0.0)
    }

    fn is_finite(self) -> bool {
        self.0.is_finite()
    }
}

impl Debug for Scalar {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Eq for Scalar {}

impl PartialEq for Scalar {
    fn eq(&self, other: &Self) -> bool {
        assert!(!self.0.is_nan() && !other.0.is_nan(), "float is NaN");
        self.0 == other.0
    }
}

impl PartialEq<f64> for Scalar {
    fn eq(&self, other: &f64) -> bool {
        self == &Self(*other)
    }
}

impl Ord for Scalar {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.partial_cmp(&other.0).expect("float is NaN")
    }
}

impl PartialOrd for Scalar {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Hash for Scalar {
    fn hash<H: Hasher>(&self, state: &mut H) {
        debug_assert!(!self.0.is_nan(), "float is NaN");
        self.0.to_bits().hash(state);
    }
}

impl From<f64> for Scalar {
    fn from(float: f64) -> Self {
        Self::new(float)
    }
}

impl From<Scalar> for f64 {
    fn from(scalar: Scalar) -> Self {
        scalar.0
    }
}

impl Neg for Scalar {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(-self.0)
    }
}

impl Add<Self> for Scalar {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.0 + rhs.0)
    }
}

impl Add<f64> for Scalar {
    type Output = Self;

    fn add(self, rhs: f64) -> Self::Output {
        Self::new(self.0 + rhs)
    }
}

impl Add<Scalar> for f64 {
    type Output = Scalar;

    fn add(self, rhs: Scalar) -> Self::Output {
        Scalar::new(self + rhs.0)
    }
}

impl Sub<Self> for Scalar {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.0 - rhs.0)
    }
}

impl Sub<f64> for Scalar {
    type Output = Self;

    fn sub(self, rhs: f64) -> Self::Output {
        Self::new(self.0 - rhs)
    }
}

impl Sub<Scalar> for f64 {
    type Output = Scalar;

    fn sub(self, rhs: Scalar) -> Self::Output {
        Scalar::new(self - rhs.0)
    }
}

impl Mul<Self> for Scalar {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self::new(self.0 * rhs.0)
    }
}

impl Mul<f64> for Scalar {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self::new(self.0 * rhs)
    }
}

impl Mul<Scalar> for f64 {
    type Output = Scalar;

    fn mul(self, rhs: Scalar) -> Self::Output {
        Scalar::new(self * rhs.0)
    }
}

impl Div<Self> for Scalar {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self::new(self.0 / rhs.0)
    }
}

impl Div<f64> for Scalar {
    type Output = Self;

    fn div(self, rhs: f64) -> Self::Output {
        Self::new(self.0 / rhs)
    }
}

impl Div<Scalar> for f64 {
    type Output = Scalar;

    fn div(self, rhs: Scalar) -> Self::Output {
        Scalar::new(self / rhs.0)
    }
}

impl Rem<Self> for Scalar {
    type Output = Self;

    fn rem(self, rhs: Self) -> Self::Output {
        Self::new(self.0 % rhs.0)
    }
}

impl Rem<f64> for Scalar {
    type Output = Self;

    fn rem(self, rhs: f64) -> Self::Output {
        Self::new(self.0 % rhs)
    }
}

impl Rem<Scalar> for f64 {
    type Output = Scalar;

    fn rem(self, rhs: Scalar) -> Self::Output {
        Scalar::new(self % rhs.0)
    }
}

impl Sum for Scalar {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Self::new(iter.map(|s| s.0).sum())
    }
}

impl<'a> Sum<&'a Self> for Scalar {
    fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        Self::new(iter.map(|s| s.0).sum())
    }
}

assign_impl!(Scalar += Scalar);
assign_impl!(Scalar += f64);
assign_impl!(Scalar -= Scalar);
assign_impl!(Scalar -= f64);
assign_impl!(Scalar *= Scalar);
assign_impl!(Scalar *= f64);
assign_impl!(Scalar /= Scalar);
assign_impl!(Scalar /= f64);
assign_impl!(Scalar %= Scalar);
assign_impl!(Scalar %= f64);
