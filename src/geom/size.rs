use super::*;

use serde::{Deserialize, Serialize};

/// A size in 2D.
#[derive(Default, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Size {
    /// The width.
    pub w: Length,
    /// The height.
    pub h: Length,
}

impl Size {
    /// The zero size.
    pub fn zero() -> Self {
        Self { w: Length::zero(), h: Length::zero() }
    }

    /// Create a new size from width and height.
    pub fn new(w: Length, h: Length) -> Self {
        Self { w, h }
    }

    /// Create an instance with two equal components.
    pub fn splat(v: Length) -> Self {
        Self { w: v, h: v }
    }

    /// Limit width and height at that of another size.
    pub fn cap(self, limit: Self) -> Self {
        Self {
            w: self.w.min(limit.w),
            h: self.h.min(limit.h),
        }
    }

    /// Whether the other size fits into this one (smaller width and height).
    pub fn fits(self, other: Self) -> bool {
        self.w.fits(other.w) && self.h.fits(other.h)
    }

    /// Whether both components are finite.
    pub fn is_finite(self) -> bool {
        self.w.is_finite() && self.h.is_finite()
    }

    /// Whether any of the two components is infinite.
    pub fn is_infinite(self) -> bool {
        self.w.is_infinite() || self.h.is_infinite()
    }

    /// Convert to a point.
    pub fn to_point(self) -> Point {
        Point::new(self.w, self.h)
    }

    /// Convert to a Spec.
    pub fn to_spec(self) -> Spec<Length> {
        Spec::new(self.w, self.h)
    }

    /// Convert to the generic representation.
    pub fn to_gen(self, block: SpecAxis) -> Gen<Length> {
        match block {
            SpecAxis::Horizontal => Gen::new(self.h, self.w),
            SpecAxis::Vertical => Gen::new(self.w, self.h),
        }
    }
}

impl Get<SpecAxis> for Size {
    type Component = Length;

    fn get(self, axis: SpecAxis) -> Length {
        match axis {
            SpecAxis::Horizontal => self.w,
            SpecAxis::Vertical => self.h,
        }
    }

    fn get_mut(&mut self, axis: SpecAxis) -> &mut Length {
        match axis {
            SpecAxis::Horizontal => &mut self.w,
            SpecAxis::Vertical => &mut self.h,
        }
    }
}

impl Debug for Size {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Size({:?}, {:?})", self.w, self.h)
    }
}

impl Neg for Size {
    type Output = Self;

    fn neg(self) -> Self {
        Self { w: -self.w, h: -self.h }
    }
}

impl Add for Size {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self { w: self.w + other.w, h: self.h + other.h }
    }
}

sub_impl!(Size - Size -> Size);

impl Mul<f64> for Size {
    type Output = Self;

    fn mul(self, other: f64) -> Self {
        Self { w: self.w * other, h: self.h * other }
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
        Self { w: self.w / other, h: self.h / other }
    }
}

assign_impl!(Size -= Size);
assign_impl!(Size += Size);
assign_impl!(Size *= f64);
assign_impl!(Size /= f64);
