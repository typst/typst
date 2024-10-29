use std::fmt::{self, Debug, Formatter};
use std::iter::Sum;
use std::ops::{Add, Div, Mul, Neg, Rem};

use ecow::EcoString;
use typst_utils::{Numeric, Scalar};

use crate::foundations::{cast, repr, Fold, Repr, Value};

/// An absolute length.
#[derive(Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Abs(Scalar);

impl Abs {
    /// The zero length.
    pub const fn zero() -> Self {
        Self(Scalar::ZERO)
    }

    /// The infinite length.
    pub const fn inf() -> Self {
        Self(Scalar::INFINITY)
    }

    /// Create an absolute length from a number of raw units.
    pub const fn raw(raw: f64) -> Self {
        Self(Scalar::new(raw))
    }

    /// Create an absolute length from a value in a unit.
    pub fn with_unit(val: f64, unit: AbsUnit) -> Self {
        Self(Scalar::new(val * unit.raw_scale()))
    }

    /// Create an absolute length from a number of points.
    pub fn pt(pt: f64) -> Self {
        Self::with_unit(pt, AbsUnit::Pt)
    }

    /// Create an absolute length from a number of millimeters.
    pub fn mm(mm: f64) -> Self {
        Self::with_unit(mm, AbsUnit::Mm)
    }

    /// Create an absolute length from a number of centimeters.
    pub fn cm(cm: f64) -> Self {
        Self::with_unit(cm, AbsUnit::Cm)
    }

    /// Create an absolute length from a number of inches.
    pub fn inches(inches: f64) -> Self {
        Self::with_unit(inches, AbsUnit::In)
    }

    /// Get the value of this absolute length in raw units.
    pub const fn to_raw(self) -> f64 {
        self.0.get()
    }

    /// Get the value of this absolute length in a unit.
    pub fn to_unit(self, unit: AbsUnit) -> f64 {
        self.to_raw() / unit.raw_scale()
    }

    /// Convert this to a number of points.
    pub fn to_pt(self) -> f64 {
        self.to_unit(AbsUnit::Pt)
    }

    /// Convert this to a number of millimeters.
    pub fn to_mm(self) -> f64 {
        self.to_unit(AbsUnit::Mm)
    }

    /// Convert this to a number of centimeters.
    pub fn to_cm(self) -> f64 {
        self.to_unit(AbsUnit::Cm)
    }

    /// Convert this to a number of inches.
    pub fn to_inches(self) -> f64 {
        self.to_unit(AbsUnit::In)
    }

    /// The absolute value of this length.
    pub fn abs(self) -> Self {
        Self::raw(self.to_raw().abs())
    }

    /// The minimum of this and another absolute length.
    pub fn min(self, other: Self) -> Self {
        Self(self.0.min(other.0))
    }

    /// Set to the minimum of this and another absolute length.
    pub fn set_min(&mut self, other: Self) {
        *self = (*self).min(other);
    }

    /// The maximum of this and another absolute length.
    pub fn max(self, other: Self) -> Self {
        Self(self.0.max(other.0))
    }

    /// Set to the maximum of this and another absolute length.
    pub fn set_max(&mut self, other: Self) {
        *self = (*self).max(other);
    }

    /// Whether the other absolute length fits into this one (i.e. is smaller).
    /// Allows for a bit of slack.
    pub fn fits(self, other: Self) -> bool {
        self.0 + AbsUnit::EPS >= other.0
    }

    /// Compares two absolute lengths for whether they are approximately equal.
    pub fn approx_eq(self, other: Self) -> bool {
        self == other || (self - other).to_raw().abs() < AbsUnit::EPS
    }

    /// Whether the size is close to zero or negative.
    pub fn approx_empty(self) -> bool {
        self.to_raw() <= AbsUnit::EPS
    }

    /// Returns a number that represent the sign of this length
    pub fn signum(self) -> f64 {
        self.0.get().signum()
    }
}

impl Numeric for Abs {
    fn zero() -> Self {
        Self::zero()
    }

    fn is_finite(self) -> bool {
        self.0.is_finite()
    }
}

impl Debug for Abs {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}pt", self.to_pt())
    }
}

impl Repr for Abs {
    fn repr(&self) -> EcoString {
        repr::format_float_with_unit(self.to_pt(), "pt")
    }
}

impl Neg for Abs {
    type Output = Self;

    fn neg(self) -> Self {
        Self(-self.0)
    }
}

impl Add for Abs {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

typst_utils::sub_impl!(Abs - Abs -> Abs);

impl Mul<f64> for Abs {
    type Output = Self;

    fn mul(self, other: f64) -> Self {
        Self(self.0 * other)
    }
}

impl Mul<Abs> for f64 {
    type Output = Abs;

    fn mul(self, other: Abs) -> Abs {
        other * self
    }
}

impl Div<f64> for Abs {
    type Output = Self;

    fn div(self, other: f64) -> Self {
        Self(self.0 / other)
    }
}

impl Div for Abs {
    type Output = f64;

    fn div(self, other: Self) -> f64 {
        self.to_raw() / other.to_raw()
    }
}

typst_utils::assign_impl!(Abs += Abs);
typst_utils::assign_impl!(Abs -= Abs);
typst_utils::assign_impl!(Abs *= f64);
typst_utils::assign_impl!(Abs /= f64);

impl Rem for Abs {
    type Output = Self;

    fn rem(self, other: Self) -> Self::Output {
        Self(self.0 % other.0)
    }
}

impl Sum for Abs {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Self(iter.map(|s| s.0).sum())
    }
}

impl<'a> Sum<&'a Self> for Abs {
    fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        Self(iter.map(|s| s.0).sum())
    }
}

impl Fold for Abs {
    fn fold(self, _: Self) -> Self {
        self
    }
}

cast! {
    Abs,
    self => Value::Length(self.into()),
}

/// Different units of absolute measurement.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum AbsUnit {
    /// Points.
    Pt,
    /// Millimeters.
    Mm,
    /// Centimeters.
    Cm,
    /// Inches.
    In,
}

impl AbsUnit {
    /// The epsilon for approximate length comparisons.
    const EPS: f64 = 1e-4;

    /// How many raw units correspond to a value of `1.0` in this unit.
    const fn raw_scale(self) -> f64 {
        // We choose a raw scale which has an integer conversion value to all
        // four units of interest, so that whole numbers in all units can be
        // represented accurately.
        match self {
            AbsUnit::Pt => 127.0,
            AbsUnit::Mm => 360.0,
            AbsUnit::Cm => 3600.0,
            AbsUnit::In => 9144.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_length_unit_conversion() {
        assert!((Abs::mm(150.0).to_cm() - 15.0) < 1e-4);
    }
}
