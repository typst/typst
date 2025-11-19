use std::fmt::{self, Debug, Formatter};
use std::ops::{Add, Div, Mul, Neg};

use typst_utils::{Get, Numeric};

use crate::layout::{Abs, Axis, Size, Transform};

/// A point in 2D.
#[derive(Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Point {
    /// The x coordinate.
    pub x: Abs,
    /// The y coordinate.
    pub y: Abs,
}

impl Point {
    /// The origin point.
    pub const fn zero() -> Self {
        Self { x: Abs::zero(), y: Abs::zero() }
    }

    /// Create a new point from x and y coordinates.
    pub const fn new(x: Abs, y: Abs) -> Self {
        Self { x, y }
    }

    /// Create an instance with two equal components.
    pub const fn splat(value: Abs) -> Self {
        Self { x: value, y: value }
    }

    /// Create a new point with y set to zero.
    pub const fn with_x(x: Abs) -> Self {
        Self { x, y: Abs::zero() }
    }

    /// Create a new point with x set to zero.
    pub const fn with_y(y: Abs) -> Self {
        Self { x: Abs::zero(), y }
    }

    /// The component-wise minimum of this and another point.
    pub fn min(self, other: Self) -> Self {
        Self { x: self.x.min(other.x), y: self.y.min(other.y) }
    }

    /// The component-wise minimum of this and another point.
    pub fn max(self, other: Self) -> Self {
        Self { x: self.x.max(other.x), y: self.y.max(other.y) }
    }

    /// Maps the point with the given function.
    pub fn map(self, f: impl Fn(Abs) -> Abs) -> Self {
        Self { x: f(self.x), y: f(self.y) }
    }

    /// The distance between this point and the origin.
    pub fn hypot(self) -> Abs {
        Abs::raw(self.x.to_raw().hypot(self.y.to_raw()))
    }

    // TODO: this is a bit awkward on a point struct.
    pub fn normalized(self) -> Self {
        self / self.hypot().to_raw()
    }

    /// Rotate the point 90 degrees counter-clockwise.
    pub fn rot90ccw(self) -> Self {
        Self { x: self.y, y: -self.x }
    }

    /// Transform the point with the given transformation.
    ///
    /// In the event that one of the coordinates is infinite, the result will
    /// be zero.
    pub fn transform(self, ts: Transform) -> Self {
        Self::new(
            ts.sx.of(self.x) + ts.kx.of(self.y) + ts.tx,
            ts.ky.of(self.x) + ts.sy.of(self.y) + ts.ty,
        )
    }

    /// Transforms the point with the given transformation, without accounting
    /// for infinite values.
    pub fn transform_inf(self, ts: Transform) -> Self {
        Self::new(
            ts.sx.get() * self.x + ts.kx.get() * self.y + ts.tx,
            ts.ky.get() * self.x + ts.sy.get() * self.y + ts.ty,
        )
    }

    /// Convert to a size.
    pub fn to_size(self) -> Size {
        Size::new(self.x, self.y)
    }
}

impl Numeric for Point {
    fn zero() -> Self {
        Self::zero()
    }

    fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

impl Get<Axis> for Point {
    type Component = Abs;

    fn get_ref(&self, axis: Axis) -> &Abs {
        match axis {
            Axis::X => &self.x,
            Axis::Y => &self.y,
        }
    }

    fn get_mut(&mut self, axis: Axis) -> &mut Abs {
        match axis {
            Axis::X => &mut self.x,
            Axis::Y => &mut self.y,
        }
    }
}

impl Debug for Point {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Point({:?}, {:?})", self.x, self.y)
    }
}

impl Neg for Point {
    type Output = Self;

    fn neg(self) -> Self {
        Self { x: -self.x, y: -self.y }
    }
}

impl Add for Point {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self { x: self.x + other.x, y: self.y + other.y }
    }
}

typst_utils::sub_impl!(Point - Point -> Point);

impl Mul<f64> for Point {
    type Output = Self;

    fn mul(self, other: f64) -> Self {
        Self { x: self.x * other, y: self.y * other }
    }
}

impl Mul<Point> for f64 {
    type Output = Point;

    fn mul(self, other: Point) -> Point {
        other * self
    }
}

impl Div<f64> for Point {
    type Output = Self;

    fn div(self, other: f64) -> Self {
        Self { x: self.x / other, y: self.y / other }
    }
}

typst_utils::assign_impl!(Point += Point);
typst_utils::assign_impl!(Point -= Point);
typst_utils::assign_impl!(Point *= f64);
typst_utils::assign_impl!(Point /= f64);
