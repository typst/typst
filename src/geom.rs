//! Geometrical types.

#[doc(no_inline)]
pub use kurbo::*;

use std::fmt::{self, Debug, Formatter};
use std::ops::*;

use crate::layout::primitive::{Dir, GenAlign, LayoutAlign, LayoutSystem, SpecAxis};

/// Additional methods for [sizes].
///
/// [sizes]: ../../kurbo/struct.Size.html
pub trait SizeExt {
    /// Return the component for the specified axis.
    fn get(self, axis: SpecAxis) -> f64;

    /// Borrow the component for the specified axis mutably.
    fn get_mut(&mut self, axis: SpecAxis) -> &mut f64;

    /// Returns the generalized version of a `Size` based on the layouting
    /// system, that is:
    /// - `x` describes the primary axis instead of the horizontal one.
    /// - `y` describes the secondary axis instead of the vertical one.
    fn generalized(self, sys: LayoutSystem) -> Self;

    /// Returns the specialized version of this generalized Size2D (inverse to
    /// `generalized`).
    fn specialized(self, sys: LayoutSystem) -> Self;

    /// Whether the given size fits into this one, that is, both coordinate
    /// values are smaller or equal.
    fn fits(self, other: Self) -> bool;

    /// The anchor position for an object to be aligned according to `align` in
    /// a container with this size.
    ///
    /// This assumes the size to be generalized such that `width` corresponds to
    /// the primary and `height` to the secondary axis.
    fn anchor(self, align: LayoutAlign, sys: LayoutSystem) -> Point;
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

    fn generalized(self, sys: LayoutSystem) -> Self {
        match sys.primary.axis() {
            SpecAxis::Horizontal => self,
            SpecAxis::Vertical => Self::new(self.height, self.width),
        }
    }

    fn specialized(self, sys: LayoutSystem) -> Self {
        // Even though generalized is its own inverse, we still have this second
        // function, for clarity at the call-site.
        self.generalized(sys)
    }

    fn fits(self, other: Self) -> bool {
        self.width >= other.width && self.height >= other.height
    }

    fn anchor(self, align: LayoutAlign, sys: LayoutSystem) -> Point {
        fn anchor(length: f64, align: GenAlign, dir: Dir) -> f64 {
            match (dir.is_positive(), align) {
                (true, GenAlign::Start) | (false, GenAlign::End) => 0.0,
                (_, GenAlign::Center) => length / 2.0,
                (true, GenAlign::End) | (false, GenAlign::Start) => length,
            }
        }

        Point::new(
            anchor(self.width, align.primary, sys.primary),
            anchor(self.height, align.secondary, sys.secondary),
        )
    }
}

/// Additional methods for [rectangles].
///
/// [rectangles]: ../../kurbo/struct.Rect.html
pub trait RectExt {
    /// Return the side identified by direction and alignment.
    ///
    /// Center alignment is treated the same as origin alignment.
    fn get(&mut self, dir: Dir, align: GenAlign) -> f64;

    /// Get a mutable reference to the side identified by direction and
    /// alignment.
    ///
    /// Center alignment is treated the same as origin alignment.
    fn get_mut(&mut self, dir: Dir, align: GenAlign) -> &mut f64;
}

impl RectExt for Rect {
    fn get(&mut self, dir: Dir, align: GenAlign) -> f64 {
        match if align == GenAlign::End { dir.inv() } else { dir } {
            Dir::LTR => self.x0,
            Dir::TTB => self.y0,
            Dir::RTL => self.x1,
            Dir::BTT => self.y1,
        }
    }

    fn get_mut(&mut self, dir: Dir, align: GenAlign) -> &mut f64 {
        match if align == GenAlign::End { dir.inv() } else { dir } {
            Dir::LTR => &mut self.x0,
            Dir::TTB => &mut self.y0,
            Dir::RTL => &mut self.x1,
            Dir::BTT => &mut self.y1,
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
