#[allow(clippy::wildcard_imports /* this module exists to reduce file size, not to introduce a new scope */)]
use super::*;

/// A size in 2D.
pub type Size = Axes<Abs>;

impl Size {
    /// The zero value.
    #[must_use]
    #[inline]
    pub const fn zero() -> Self {
        Self { x: Abs::zero(), y: Abs::zero() }
    }

    /// Whether the other size fits into this one (smaller width and height).
    #[must_use]
    #[inline]
    pub fn fits(self, other: Self) -> bool {
        self.x.fits(other.x) && self.y.fits(other.y)
    }

    /// Convert to a point.
    #[must_use]
    #[inline]
    pub fn to_point(self) -> Point {
        Point::new(self.x, self.y)
    }
}

impl Numeric for Size {
    #[inline]
    fn zero() -> Self {
        Self::zero()
    }

    #[inline]
    fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

impl Neg for Size {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        Self { x: -self.x, y: -self.y }
    }
}

impl Add for Size {
    type Output = Self;

    #[inline]
    fn add(self, other: Self) -> Self {
        Self { x: self.x + other.x, y: self.y + other.y }
    }
}

sub_impl!(Size - Size -> Size);

impl Mul<f64> for Size {
    type Output = Self;

    #[inline]
    fn mul(self, other: f64) -> Self {
        Self { x: self.x * other, y: self.y * other }
    }
}

impl Mul<Size> for f64 {
    type Output = Size;

    #[inline]
    fn mul(self, other: Size) -> Size {
        other * self
    }
}

impl Div<f64> for Size {
    type Output = Self;

    #[inline]
    fn div(self, other: f64) -> Self {
        Self { x: self.x / other, y: self.y / other }
    }
}

assign_impl!(Size -= Size);
assign_impl!(Size += Size);
assign_impl!(Size *= f64);
assign_impl!(Size /= f64);
