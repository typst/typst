use std::f64::consts::PI;
use std::fmt::{self, Debug, Formatter};
use std::iter::Sum;
use std::ops::{Add, Div, Mul, Neg};

use ecow::EcoString;
use typst_utils::{Numeric, Scalar};

use crate::foundations::{func, repr, scope, ty, Repr};

/// An angle describing a rotation.
///
/// Typst supports the following angular units:
///
/// - Degrees: `{180deg}`
/// - Radians: `{3.14rad}`
///
/// # Example
/// ```example
/// #rotate(10deg)[Hello there!]
/// ```
#[ty(scope, cast)]
#[derive(Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Angle(Scalar);

impl Angle {
    /// The zero angle.
    pub const fn zero() -> Self {
        Self(Scalar::ZERO)
    }

    /// Create an angle from a number of raw units.
    pub const fn raw(raw: f64) -> Self {
        Self(Scalar::new(raw))
    }

    /// Create an angle from a value in a unit.
    pub fn with_unit(val: f64, unit: AngleUnit) -> Self {
        Self(Scalar::new(val * unit.raw_scale()))
    }

    /// Create an angle from a number of radians.
    pub fn rad(rad: f64) -> Self {
        Self::with_unit(rad, AngleUnit::Rad)
    }

    /// Create an angle from a number of degrees.
    pub fn deg(deg: f64) -> Self {
        Self::with_unit(deg, AngleUnit::Deg)
    }

    /// Get the value of this angle in raw units.
    pub const fn to_raw(self) -> f64 {
        (self.0).get()
    }

    /// Get the value of this angle in a unit.
    pub fn to_unit(self, unit: AngleUnit) -> f64 {
        self.to_raw() / unit.raw_scale()
    }

    /// The absolute value of the this angle.
    pub fn abs(self) -> Self {
        Self::raw(self.to_raw().abs())
    }

    /// Get the sine of this angle in radians.
    pub fn sin(self) -> f64 {
        self.to_rad().sin()
    }

    /// Get the cosine of this angle in radians.
    pub fn cos(self) -> f64 {
        self.to_rad().cos()
    }

    /// Get the tangent of this angle in radians.
    pub fn tan(self) -> f64 {
        self.to_rad().tan()
    }

    /// Get the quadrant of the Cartesian plane that this angle lies in.
    ///
    /// The angle is automatically normalized to the range `0deg..=360deg`.
    ///
    /// The quadrants are defined as follows:
    /// - First: `0deg..=90deg` (top-right)
    /// - Second: `90deg..=180deg` (top-left)
    /// - Third: `180deg..=270deg` (bottom-left)
    /// - Fourth: `270deg..=360deg` (bottom-right)
    pub fn quadrant(self) -> Quadrant {
        let angle = self.to_deg().rem_euclid(360.0);
        if angle <= 90.0 {
            Quadrant::First
        } else if angle <= 180.0 {
            Quadrant::Second
        } else if angle <= 270.0 {
            Quadrant::Third
        } else {
            Quadrant::Fourth
        }
    }
}

#[scope]
impl Angle {
    /// Converts this angle to radians.
    #[func(name = "rad", title = "Radians")]
    pub fn to_rad(self) -> f64 {
        self.to_unit(AngleUnit::Rad)
    }

    /// Converts this angle to degrees.
    #[func(name = "deg", title = "Degrees")]
    pub fn to_deg(self) -> f64 {
        self.to_unit(AngleUnit::Deg)
    }
}

impl Numeric for Angle {
    fn zero() -> Self {
        Self::zero()
    }

    fn is_finite(self) -> bool {
        self.0.is_finite()
    }
}

impl Debug for Angle {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}deg", self.to_deg())
    }
}

impl Repr for Angle {
    fn repr(&self) -> EcoString {
        repr::format_float_with_unit(self.to_deg(), "deg")
    }
}

impl Neg for Angle {
    type Output = Self;

    fn neg(self) -> Self {
        Self(-self.0)
    }
}

impl Add for Angle {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

typst_utils::sub_impl!(Angle - Angle -> Angle);

impl Mul<f64> for Angle {
    type Output = Self;

    fn mul(self, other: f64) -> Self {
        Self(self.0 * other)
    }
}

impl Mul<Angle> for f64 {
    type Output = Angle;

    fn mul(self, other: Angle) -> Angle {
        other * self
    }
}

impl Div for Angle {
    type Output = f64;

    fn div(self, other: Self) -> f64 {
        self.to_raw() / other.to_raw()
    }
}

impl Div<f64> for Angle {
    type Output = Self;

    fn div(self, other: f64) -> Self {
        Self(self.0 / other)
    }
}

typst_utils::assign_impl!(Angle += Angle);
typst_utils::assign_impl!(Angle -= Angle);
typst_utils::assign_impl!(Angle *= f64);
typst_utils::assign_impl!(Angle /= f64);

impl Sum for Angle {
    fn sum<I: Iterator<Item = Angle>>(iter: I) -> Self {
        Self(iter.map(|s| s.0).sum())
    }
}

/// Different units of angular measurement.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum AngleUnit {
    /// Radians.
    Rad,
    /// Degrees.
    Deg,
}

impl AngleUnit {
    /// How many raw units correspond to a value of `1.0` in this unit.
    fn raw_scale(self) -> f64 {
        match self {
            Self::Rad => 1.0,
            Self::Deg => PI / 180.0,
        }
    }
}

/// A quadrant of the Cartesian plane.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Quadrant {
    /// The first quadrant, containing positive x and y values.
    First,
    /// The second quadrant, containing negative x and positive y values.
    Second,
    /// The third quadrant, containing negative x and y values.
    Third,
    /// The fourth quadrant, containing positive x and negative y values.
    Fourth,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_angle_unit_conversion() {
        assert!((Angle::rad(2.0 * PI).to_deg() - 360.0) < 1e-4);
        assert!((Angle::deg(45.0).to_rad() - std::f64::consts::FRAC_PI_4) < 1e-4);
    }
}
