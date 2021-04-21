use super::*;

use serde::{Deserialize, Serialize};

/// A size in 2D.
#[derive(Default, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct Size {
    /// The width.
    pub width: Length,
    /// The height.
    pub height: Length,
}

impl Size {
    /// The zero size.
    pub const ZERO: Self = Self {
        width: Length::ZERO,
        height: Length::ZERO,
    };

    /// Create a new size from width and height.
    pub fn new(width: Length, height: Length) -> Self {
        Self { width, height }
    }

    /// Create an instance with two equal components.
    pub fn uniform(value: Length) -> Self {
        Self { width: value, height: value }
    }

    /// Whether the other size fits into this one (smaller width and height).
    pub fn fits(self, other: Self) -> bool {
        self.width.fits(other.width) && self.height.fits(other.height)
    }

    /// Whether both components are finite.
    pub fn is_finite(self) -> bool {
        self.width.is_finite() && self.height.is_finite()
    }

    /// Whether any of the two components is infinite.
    pub fn is_infinite(self) -> bool {
        self.width.is_infinite() || self.height.is_infinite()
    }

    /// Whether any of the two components is `NaN`.
    pub fn is_nan(self) -> bool {
        self.width.is_nan() || self.height.is_nan()
    }

    /// Convert to a point.
    pub fn to_point(self) -> Point {
        Point::new(self.width, self.height)
    }
}

impl Get<SpecAxis> for Size {
    type Component = Length;

    fn get(self, axis: SpecAxis) -> Length {
        match axis {
            SpecAxis::Horizontal => self.width,
            SpecAxis::Vertical => self.height,
        }
    }

    fn get_mut(&mut self, axis: SpecAxis) -> &mut Length {
        match axis {
            SpecAxis::Horizontal => &mut self.width,
            SpecAxis::Vertical => &mut self.height,
        }
    }
}

impl Switch for Size {
    type Other = Gen<Length>;

    fn switch(self, main: SpecAxis) -> Self::Other {
        match main {
            SpecAxis::Horizontal => Gen::new(self.width, self.height),
            SpecAxis::Vertical => Gen::new(self.height, self.width),
        }
    }
}

impl Debug for Size {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Size({:?}, {:?})", self.width, self.height)
    }
}

impl Neg for Size {
    type Output = Self;

    fn neg(self) -> Self {
        Self { width: -self.width, height: -self.height }
    }
}

impl Add for Size {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            width: self.width + other.width,
            height: self.height + other.height,
        }
    }
}

sub_impl!(Size - Size -> Size);

impl Mul<f64> for Size {
    type Output = Self;

    fn mul(self, other: f64) -> Self {
        Self {
            width: self.width * other,
            height: self.height * other,
        }
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
        Self {
            width: self.width / other,
            height: self.height / other,
        }
    }
}

assign_impl!(Size -= Size);
assign_impl!(Size += Size);
assign_impl!(Size *= f64);
assign_impl!(Size /= f64);
