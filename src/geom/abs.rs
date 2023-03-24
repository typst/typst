#[allow(clippy::wildcard_imports /* this module exists to reduce file size, not to introduce a new scope */)]
use super::*;

/// An absolute length.
#[derive(Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Abs(Scalar);

impl Abs {
    /// The zero length.
    #[must_use]
    #[inline]
    pub const fn zero() -> Self {
        Self(Scalar(0.0))
    }

    /// The infinite length.
    #[must_use]
    #[inline]
    pub const fn inf() -> Self {
        Self(Scalar(f64::INFINITY))
    }

    /// Create an absolute length from a number of raw units.
    #[must_use]
    #[inline]
    pub const fn raw(raw: f64) -> Self {
        Self(Scalar(raw))
    }

    /// Create an absolute length from a value in a unit.
    #[must_use]
    #[inline]
    pub fn with_unit(val: f64, unit: AbsUnit) -> Self {
        Self(Scalar(val * unit.raw_scale()))
    }

    /// Create an absolute length from a number of points.
    #[must_use]
    #[inline]
    pub fn pt(pt: f64) -> Self {
        Self::with_unit(pt, AbsUnit::Pt)
    }

    /// Create an absolute length from a number of millimeters.
    #[must_use]
    #[inline]
    pub fn mm(mm: f64) -> Self {
        Self::with_unit(mm, AbsUnit::Mm)
    }

    /// Create an absolute length from a number of centimeters.
    #[must_use]
    #[inline]
    pub fn cm(cm: f64) -> Self {
        Self::with_unit(cm, AbsUnit::Cm)
    }

    /// Create an absolute length from a number of inches.
    #[must_use]
    #[inline]
    pub fn inches(inches: f64) -> Self {
        Self::with_unit(inches, AbsUnit::In)
    }

    /// Get the value of this absolute length in raw units.
    #[must_use]
    #[inline]
    pub const fn to_raw(self) -> f64 {
        (self.0).0
    }

    /// Get the value of this absolute length in a unit.
    #[must_use]
    #[inline]
    pub fn to_unit(self, unit: AbsUnit) -> f64 {
        self.to_raw() / unit.raw_scale()
    }

    /// Convert this to a number of points.
    #[must_use]
    #[inline]
    pub fn to_pt(self) -> f64 {
        self.to_unit(AbsUnit::Pt)
    }

    /// Convert this to a number of millimeters.
    #[must_use]
    #[inline]
    pub fn to_mm(self) -> f64 {
        self.to_unit(AbsUnit::Mm)
    }

    /// Convert this to a number of centimeters.
    #[must_use]
    #[inline]
    pub fn to_cm(self) -> f64 {
        self.to_unit(AbsUnit::Cm)
    }

    /// Convert this to a number of inches.
    #[must_use]
    #[inline]
    pub fn to_inches(self) -> f64 {
        self.to_unit(AbsUnit::In)
    }

    /// The absolute value of this length.
    #[must_use]
    #[inline]
    pub fn abs(self) -> Self {
        Self::raw(self.to_raw().abs())
    }

    /// The minimum of this and another absolute length.
    #[must_use]
    #[inline]
    pub fn min(self, other: Self) -> Self {
        Self(self.0.min(other.0))
    }

    /// Set to the minimum of this and another absolute length.
    #[inline]
    pub fn set_min(&mut self, other: Self) {
        *self = (*self).min(other);
    }

    /// The maximum of this and another absolute length.
    #[must_use]
    #[inline]
    pub fn max(self, other: Self) -> Self {
        Self(self.0.max(other.0))
    }

    /// Set to the maximum of this and another absolute length.
    #[inline]
    pub fn set_max(&mut self, other: Self) {
        *self = (*self).max(other);
    }

    /// Whether the other absolute length fits into this one (i.e. is smaller).
    /// Allows for a bit of slack.
    #[must_use]
    #[inline]
    pub fn fits(self, other: Self) -> bool {
        self.0 + 1e-6 >= other.0
    }

    /// Compares two absolute lengths for whether they are approximately equal.
    #[must_use]
    #[inline]
    pub fn approx_eq(self, other: Self) -> bool {
        self == other || (self - other).to_raw().abs() < 1e-6
    }

    /// Perform a checked division by a number,
    /// returning zero if the result is not finite.
    #[must_use]
    #[inline]
    pub fn safe_div(self, number: f64) -> Self {
        let result = self.to_raw() / number;
        if result.is_finite() {
            Self::raw(result)
        } else {
            Self::zero()
        }
    }
}

impl Numeric for Abs {
    #[inline]
    fn zero() -> Self {
        Self::zero()
    }

    #[inline]
    fn is_finite(self) -> bool {
        self.0.is_finite()
    }
}

impl Debug for Abs {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}pt", round_2(self.to_pt()))
    }
}

impl Neg for Abs {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        Self(-self.0)
    }
}

impl Add for Abs {
    type Output = Self;

    #[inline]
    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

sub_impl!(Abs - Abs -> Abs);

impl Mul<f64> for Abs {
    type Output = Self;

    #[inline]
    fn mul(self, other: f64) -> Self {
        Self(self.0 * other)
    }
}

impl Mul<Abs> for f64 {
    type Output = Abs;

    #[inline]
    fn mul(self, other: Abs) -> Abs {
        other * self
    }
}

impl Div<f64> for Abs {
    type Output = Self;

    #[inline]
    fn div(self, other: f64) -> Self {
        Self(self.0 / other)
    }
}

impl Div for Abs {
    type Output = f64;

    #[inline]
    fn div(self, other: Self) -> f64 {
        self.to_raw() / other.to_raw()
    }
}

assign_impl!(Abs += Abs);
assign_impl!(Abs -= Abs);
assign_impl!(Abs *= f64);
assign_impl!(Abs /= f64);

impl Rem for Abs {
    type Output = Self;

    #[inline]
    fn rem(self, other: Self) -> Self::Output {
        Self(self.0 % other.0)
    }
}

impl Sum for Abs {
    #[inline]
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Self(iter.map(|s| s.0).sum())
    }
}

impl<'a> Sum<&'a Self> for Abs {
    #[inline]
    fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        Self(iter.map(|s| s.0).sum())
    }
}

cast_to_value! {
    v: Abs => Value::Length(v.into())
}

/// Different units of absolute measurement.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
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
    /// How many raw units correspond to a value of `1.0` in this unit.
    #[must_use]
    #[inline]
    fn raw_scale(self) -> f64 {
        match self {
            AbsUnit::Pt => 1.0,
            AbsUnit::Mm => 2.83465,
            AbsUnit::Cm => 28.3465,
            AbsUnit::In => 72.0,
        }
    }
}

impl Debug for AbsUnit {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.pad(match self {
            AbsUnit::Mm => "mm",
            AbsUnit::Pt => "pt",
            AbsUnit::Cm => "cm",
            AbsUnit::In => "in",
        })
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
