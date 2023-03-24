#[allow(clippy::wildcard_imports /* this module exists to reduce file size, not to introduce a new scope */)]
use super::*;

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
    #[must_use]
    #[inline]
    pub const fn zero() -> Self {
        Self { x: Abs::zero(), y: Abs::zero() }
    }

    /// Create a new point from x and y coordinates.
    #[must_use]
    #[inline]
    pub const fn new(x: Abs, y: Abs) -> Self {
        Self { x, y }
    }

    /// Create an instance with two equal components.
    #[must_use]
    #[inline]
    pub const fn splat(value: Abs) -> Self {
        Self { x: value, y: value }
    }

    /// Create a new point with y set to zero.
    #[must_use]
    #[inline]
    pub const fn with_x(x: Abs) -> Self {
        Self { x, y: Abs::zero() }
    }

    /// Create a new point with x set to zero.
    #[must_use]
    #[inline]
    pub const fn with_y(y: Abs) -> Self {
        Self { x: Abs::zero(), y }
    }

    /// The component-wise minimum of this and another point.
    #[must_use]
    #[inline]
    pub fn min(self, other: Self) -> Self {
        Self { x: self.x.min(other.x), y: self.y.min(other.y) }
    }

    /// The component-wise minimum of this and another point.
    #[must_use]
    #[inline]
    pub fn max(self, other: Self) -> Self {
        Self { x: self.x.max(other.x), y: self.y.max(other.y) }
    }

    /// Transform the point with the given transformation.
    #[must_use]
    #[inline]
    pub fn transform(self, ts: Transform) -> Self {
        Self::new(
            ts.sx.of(self.x) + ts.kx.of(self.y) + ts.tx,
            ts.ky.of(self.x) + ts.sy.of(self.y) + ts.ty,
        )
    }

    /// Convert to a size.
    #[must_use]
    #[inline]
    pub fn to_size(self) -> Size {
        Size::new(self.x, self.y)
    }
}

impl Numeric for Point {
    #[must_use]
    #[inline]
    fn zero() -> Self {
        Self::zero()
    }

    #[must_use]
    #[inline]
    fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

impl Get<Axis> for Point {
    type Component = Abs;

    #[must_use]
    #[inline]
    fn get(self, axis: Axis) -> Abs {
        match axis {
            Axis::X => self.x,
            Axis::Y => self.y,
        }
    }

    #[must_use]
    #[inline]
    fn get_mut(&mut self, axis: Axis) -> &mut Abs {
        match axis {
            Axis::X => &mut self.x,
            Axis::Y => &mut self.y,
        }
    }
}

impl Debug for Point {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Point({:?}, {:?})", self.x, self.y)
    }
}

impl Neg for Point {
    type Output = Self;

    #[must_use]
    #[inline]
    fn neg(self) -> Self {
        Self { x: -self.x, y: -self.y }
    }
}

impl Add for Point {
    type Output = Self;

    #[must_use]
    #[inline]
    fn add(self, other: Self) -> Self {
        Self { x: self.x + other.x, y: self.y + other.y }
    }
}

sub_impl!(Point - Point -> Point);

impl Mul<f64> for Point {
    type Output = Self;

    #[must_use]
    #[inline]
    fn mul(self, other: f64) -> Self {
        Self { x: self.x * other, y: self.y * other }
    }
}

impl Mul<Point> for f64 {
    type Output = Point;

    #[must_use]
    #[inline]
    fn mul(self, other: Point) -> Point {
        other * self
    }
}

impl Div<f64> for Point {
    type Output = Self;

    #[must_use]
    #[inline]
    fn div(self, other: f64) -> Self {
        Self { x: self.x / other, y: self.y / other }
    }
}

assign_impl!(Point += Point);
assign_impl!(Point -= Point);
assign_impl!(Point *= f64);
assign_impl!(Point /= f64);
