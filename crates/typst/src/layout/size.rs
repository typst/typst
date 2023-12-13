use std::ops::{Add, Div, Mul, Neg};

use crate::layout::{Abs, Axes, Point, Ratio};
use crate::util::Numeric;

/// A size in 2D.
pub type Size = Axes<Abs>;

impl Size {
    /// The zero value.
    pub const fn zero() -> Self {
        Self { x: Abs::zero(), y: Abs::zero() }
    }

    /// Whether the other size fits into this one (smaller width and height).
    pub fn fits(self, other: Self) -> bool {
        self.x.fits(other.x) && self.y.fits(other.y)
    }

    /// Convert to a point.
    pub fn to_point(self) -> Point {
        Point::new(self.x, self.y)
    }

    /// Converts to a ratio of width to height.
    pub fn aspect_ratio(self) -> Ratio {
        Ratio::new(self.x / self.y)
    }
}

impl Numeric for Size {
    fn zero() -> Self {
        Self::zero()
    }

    fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

impl Neg for Size {
    type Output = Self;

    fn neg(self) -> Self {
        Self { x: -self.x, y: -self.y }
    }
}

impl Add for Size {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self { x: self.x + other.x, y: self.y + other.y }
    }
}

sub_impl!(Size - Size -> Size);

impl Mul<f64> for Size {
    type Output = Self;

    fn mul(self, other: f64) -> Self {
        Self { x: self.x * other, y: self.y * other }
    }
}

impl Mul<Size> for f64 {
    type Output = Size;

    fn mul(self, other: Size) -> Size {
        other * self
    }
}

impl Div<f64> for Size {
    type Output = Self;

    fn div(self, other: f64) -> Self {
        Self { x: self.x / other, y: self.y / other }
    }
}

assign_impl!(Size -= Size);
assign_impl!(Size += Size);
assign_impl!(Size *= f64);
assign_impl!(Size /= f64);
