use std::ops::{Add, Div, Mul, Neg};

use typst_utils::Numeric;

use crate::layout::{Abs, Axes, Axis, Point, Ratio};

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

    /// The length along the axis
    pub fn axis_length(&self, axis: Axis) -> Abs {
        match axis {
            Axis::X => self.x,
            Axis::Y => self.y,
        }
    }

    /// The length along the axis
    pub fn axis_length_mut(&mut self, axis: Axis) -> &mut Abs {
        match axis {
            Axis::X => &mut self.x,
            Axis::Y => &mut self.y,
        }
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

typst_utils::sub_impl!(Size - Size -> Size);

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

typst_utils::assign_impl!(Size -= Size);
typst_utils::assign_impl!(Size += Size);
typst_utils::assign_impl!(Size *= f64);
typst_utils::assign_impl!(Size /= f64);
