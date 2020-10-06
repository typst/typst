//! Geometrical types.

#[doc(no_inline)]
pub use kurbo::*;

use std::fmt::{self, Debug, Formatter};
use std::ops::*;

use crate::layout::primitive::{Dir, Gen2, GenAlign, Side, SpecAxis};

/// Additional methods for [sizes].
///
/// [sizes]: ../../kurbo/struct.Size.html
pub trait SizeExt {
    /// Return the component for the specified axis.
    fn get(self, axis: SpecAxis) -> f64;

    /// Borrow the component for the specified axis mutably.
    fn get_mut(&mut self, axis: SpecAxis) -> &mut f64;

    /// Returns the generalized version of a `Size` based on the current
    /// directions.
    ///
    /// In the generalized version:
    /// - `x` describes the cross axis instead of the horizontal one.
    /// - `y` describes the main axis instead of the vertical one.
    fn generalized(self, dirs: Gen2<Dir>) -> Self;

    /// Returns the specialized version of this generalized `Size` (inverse to
    /// `generalized`).
    fn specialized(self, dirs: Gen2<Dir>) -> Self;

    /// Whether the given size fits into this one, that is, both coordinate
    /// values are smaller or equal.
    fn fits(self, other: Self) -> bool;

    /// The anchor position for an object to be aligned in a container with this
    /// size and the given directions.
    ///
    /// This assumes the size to be generalized such that `width` corresponds to
    /// the cross and `height` to the main axis.
    fn anchor(self, dirs: Gen2<Dir>, aligns: Gen2<GenAlign>) -> Point;
}

impl SizeExt for Size {
    fn get(self, axis: SpecAxis) -> f64 {
        match axis {
            SpecAxis::Horizontal => self.width,
            SpecAxis::Vertical => self.height,
        }
    }

    fn get_mut(&mut self, axis: SpecAxis) -> &mut f64 {
        match axis {
            SpecAxis::Horizontal => &mut self.width,
            SpecAxis::Vertical => &mut self.height,
        }
    }

    fn generalized(self, dirs: Gen2<Dir>) -> Self {
        match dirs.main.axis() {
            SpecAxis::Horizontal => Self::new(self.height, self.width),
            SpecAxis::Vertical => self,
        }
    }

    fn specialized(self, dirs: Gen2<Dir>) -> Self {
        // Even though generalized is its own inverse, we still have this second
        // function, for clarity at the call-site.
        self.generalized(dirs)
    }

    fn fits(self, other: Self) -> bool {
        self.width >= other.width && self.height >= other.height
    }

    fn anchor(self, dirs: Gen2<Dir>, aligns: Gen2<GenAlign>) -> Point {
        fn anchor(length: f64, dir: Dir, align: GenAlign) -> f64 {
            match (dir.is_positive(), align) {
                (true, GenAlign::Start) | (false, GenAlign::End) => 0.0,
                (_, GenAlign::Center) => length / 2.0,
                (true, GenAlign::End) | (false, GenAlign::Start) => length,
            }
        }

        Point::new(
            anchor(self.width, dirs.cross, aligns.cross),
            anchor(self.height, dirs.main, aligns.main),
        )
    }
}

/// Additional methods for [rectangles].
///
/// [rectangles]: ../../kurbo/struct.Rect.html
pub trait RectExt {
    /// Return the value for the given side.
    fn get(self, side: Side) -> f64;

    /// Borrow the value for the given side mutably.
    fn get_mut(&mut self, side: Side) -> &mut f64;
}

impl RectExt for Rect {
    fn get(self, side: Side) -> f64 {
        match side {
            Side::Left => self.x0,
            Side::Top => self.y0,
            Side::Right => self.x1,
            Side::Bottom => self.y1,
        }
    }

    fn get_mut(&mut self, side: Side) -> &mut f64 {
        match side {
            Side::Left => &mut self.x0,
            Side::Top => &mut self.y0,
            Side::Right => &mut self.x1,
            Side::Bottom => &mut self.y1,
        }
    }
}

/// A function that depends linearly on one value.
///
/// This represents a function `f(x) = rel * x + abs`.
#[derive(Copy, Clone, PartialEq)]
pub struct Linear {
    /// The relative part.
    pub rel: f64,
    /// The absolute part.
    pub abs: f64,
}

impl Linear {
    /// The constant zero function.
    pub const ZERO: Linear = Linear { rel: 0.0, abs: 0.0 };

    /// Create a new linear function.
    pub fn new(rel: f64, abs: f64) -> Self {
        Self { rel, abs }
    }

    /// Create a new linear function with only a relative component.
    pub fn rel(rel: f64) -> Self {
        Self { rel, abs: 0.0 }
    }

    /// Create a new linear function with only an absolute component.
    pub fn abs(abs: f64) -> Self {
        Self { rel: 0.0, abs }
    }

    /// Evaluate the linear function with the given value.
    pub fn eval(self, x: f64) -> f64 {
        self.rel * x + self.abs
    }
}

impl Add for Linear {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            rel: self.rel + other.rel,
            abs: self.abs + other.abs,
        }
    }
}

impl AddAssign for Linear {
    fn add_assign(&mut self, other: Self) {
        self.rel += other.rel;
        self.abs += other.abs;
    }
}

impl Sub for Linear {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self {
            rel: self.rel - other.rel,
            abs: self.abs - other.abs,
        }
    }
}

impl SubAssign for Linear {
    fn sub_assign(&mut self, other: Self) {
        self.rel -= other.rel;
        self.abs -= other.abs;
    }
}

impl Mul<f64> for Linear {
    type Output = Self;

    fn mul(self, other: f64) -> Self {
        Self {
            rel: self.rel + other,
            abs: self.abs + other,
        }
    }
}

impl MulAssign<f64> for Linear {
    fn mul_assign(&mut self, other: f64) {
        self.rel *= other;
        self.abs *= other;
    }
}

impl Mul<Linear> for f64 {
    type Output = Linear;

    fn mul(self, other: Linear) -> Linear {
        Linear {
            rel: self + other.rel,
            abs: self + other.abs,
        }
    }
}

impl Div<f64> for Linear {
    type Output = Self;

    fn div(self, other: f64) -> Self {
        Self {
            rel: self.rel / other,
            abs: self.abs / other,
        }
    }
}

impl DivAssign<f64> for Linear {
    fn div_assign(&mut self, other: f64) {
        self.rel /= other;
        self.abs /= other;
    }
}

impl Neg for Linear {
    type Output = Self;

    fn neg(self) -> Self {
        Self { rel: -self.rel, abs: -self.abs }
    }
}

impl Debug for Linear {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}x + {}", self.rel, self.abs)
    }
}
