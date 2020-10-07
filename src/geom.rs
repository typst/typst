//! Geometrical types.

#[doc(no_inline)]
pub use kurbo::*;

use std::fmt::{self, Debug, Formatter};
use std::ops::*;

use crate::layout::{Dir, Gen2, GenAlign, Get, Side, Spec2, SpecAxis, Switch};

macro_rules! impl_2d {
    ($t:ty, $x:ident, $y:ident) => {
        impl Get<SpecAxis> for $t {
            type Component = f64;

            fn get(self, axis: SpecAxis) -> f64 {
                match axis {
                    SpecAxis::Horizontal => self.$x,
                    SpecAxis::Vertical => self.$y,
                }
            }

            fn get_mut(&mut self, axis: SpecAxis) -> &mut f64 {
                match axis {
                    SpecAxis::Horizontal => &mut self.$x,
                    SpecAxis::Vertical => &mut self.$y,
                }
            }
        }

        impl Switch for $t {
            type Other = Gen2<f64>;

            fn switch(self, dirs: Gen2<Dir>) -> Self::Other {
                Spec2::new(self.$x, self.$y).switch(dirs)
            }
        }
    };
}

impl_2d!(Point, x, y);
impl_2d!(Vec2, x, y);
impl_2d!(Size, width, height);

impl Get<Side> for Rect {
    type Component = f64;

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

/// Additional methods for [sizes].
///
/// [sizes]: ../../kurbo/struct.Size.html
pub trait SizeExt {
    /// Whether the given size fits into this one, that is, both coordinate
    /// values are smaller or equal.
    fn fits(self, other: Self) -> bool;

    /// The anchor position for an object to be aligned in a container with this
    /// size and the given directions.
    fn anchor(self, dirs: Gen2<Dir>, aligns: Gen2<GenAlign>) -> Point;
}

impl SizeExt for Size {
    fn fits(self, other: Self) -> bool {
        self.width >= other.width && self.height >= other.height
    }

    fn anchor(self, dirs: Gen2<Dir>, aligns: Gen2<GenAlign>) -> Point {
        fn anchor(length: f64, dir: Dir, align: GenAlign) -> f64 {
            match if dir.is_positive() { align } else { align.inv() } {
                GenAlign::Start => 0.0,
                GenAlign::Center => length / 2.0,
                GenAlign::End => length,
            }
        }

        let switched = self.switch(dirs);
        let generic = Gen2::new(
            anchor(switched.main, dirs.main, aligns.main),
            anchor(switched.cross, dirs.cross, aligns.cross),
        );

        generic.switch(dirs).to_point()
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
            rel: self.rel * other,
            abs: self.abs * other,
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
            rel: self * other.rel,
            abs: self * other.abs,
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
