use std::fmt::{self, Debug, Formatter};
use std::ops::{Add, Div, Mul, Neg};

use crate::{Numeric, Scalar};

/// A vector in 2D.
#[derive(Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Vec2 {
    /// The x component.
    pub x: Scalar,
    /// The y component.
    pub y: Scalar,
}

impl Vec2 {
    /// The zero vector.
    pub const fn zero() -> Self {
        Self { x: Scalar::ZERO, y: Scalar::ZERO }
    }

    /// Create a new vector from x and y component.
    pub const fn new(x: Scalar, y: Scalar) -> Self {
        Self { x, y }
    }

    /// Create a new vector from x and y component.
    pub fn from_xy(x: impl Into<Scalar>, y: impl Into<Scalar>) -> Self {
        Self { x: x.into(), y: y.into() }
    }

    /// Create an instance with two equal components.
    pub const fn splat(value: Scalar) -> Self {
        Self { x: value, y: value }
    }

    /// Create a new vector with y set to zero.
    pub const fn with_x(x: Scalar) -> Self {
        Self { x, y: Scalar::ZERO }
    }

    /// Create a new vector with x set to zero.
    pub const fn with_y(y: Scalar) -> Self {
        Self { x: Scalar::ZERO, y }
    }

    /// The component-wise minimum of this and another vector.
    pub fn min(self, other: Self) -> Self {
        Self { x: self.x.min(other.x), y: self.y.min(other.y) }
    }

    /// The component-wise minimum of this and another vector.
    pub fn max(self, other: Self) -> Self {
        Self { x: self.x.max(other.x), y: self.y.max(other.y) }
    }

    /// Maps the vector with the given function.
    pub fn map(self, f: impl Fn(Scalar) -> Scalar) -> Self {
        Self { x: f(self.x), y: f(self.y) }
    }

    /// The magnitude of this vector.
    pub fn hypot(self) -> Scalar {
        // The `sqrt` function is defined by IEEE-754 and thus deterministic.
        // In addition this should be faster than `libm::hypot`.
        self.hypot2().sqrt()
    }

    /// The squared distance between this point and the origin.
    pub fn hypot2(self) -> Scalar {
        self.dot(self)
    }

    /// Returns a vector of magnitude 1.
    pub fn normalized(self) -> Self {
        self / self.hypot()
    }

    /// The dot product of two vectors.
    pub fn dot(self, other: Self) -> Scalar {
        self.x * other.x + self.y * other.y
    }
}

impl Numeric for Vec2 {
    fn zero() -> Self {
        Self::zero()
    }

    fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

impl Debug for Vec2 {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Vec2({:?}, {:?})", self.x, self.y)
    }
}

impl Neg for Vec2 {
    type Output = Self;

    fn neg(self) -> Self {
        Self { x: -self.x, y: -self.y }
    }
}

impl Add for Vec2 {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self { x: self.x + other.x, y: self.y + other.y }
    }
}

sub_impl!(Vec2 - Vec2 -> Vec2);

impl Mul<f64> for Vec2 {
    type Output = Self;

    fn mul(self, other: f64) -> Self {
        Self { x: self.x * other, y: self.y * other }
    }
}

impl Mul<Vec2> for f64 {
    type Output = Vec2;

    fn mul(self, other: Vec2) -> Vec2 {
        other * self
    }
}

impl Mul<Scalar> for Vec2 {
    type Output = Self;

    fn mul(self, other: Scalar) -> Self {
        Self { x: self.x * other, y: self.y * other }
    }
}

impl Mul<Vec2> for Scalar {
    type Output = Vec2;

    fn mul(self, other: Vec2) -> Vec2 {
        other * self
    }
}

impl Div<f64> for Vec2 {
    type Output = Self;

    fn div(self, other: f64) -> Self {
        Self { x: self.x / other, y: self.y / other }
    }
}

impl Div<Scalar> for Vec2 {
    type Output = Self;

    fn div(self, other: Scalar) -> Self {
        Self { x: self.x / other, y: self.y / other }
    }
}

assign_impl!(Vec2 += Vec2);
assign_impl!(Vec2 -= Vec2);
assign_impl!(Vec2 *= Scalar);
assign_impl!(Vec2 /= Scalar);
assign_impl!(Vec2 *= f64);
assign_impl!(Vec2 /= f64);
