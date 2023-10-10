//! Geometrical primitives.

#[macro_use]
mod macros;
mod abs;
mod align;
mod angle;
mod axes;
mod color;
mod corners;
mod dir;
mod ellipse;
mod em;
mod fr;
mod gradient;
mod length;
mod paint;
mod path;
mod point;
mod ratio;
mod rect;
mod rel;
mod scalar;
mod shape;
mod sides;
mod size;
mod smart;
mod stroke;
mod transform;

pub use self::abs::{Abs, AbsUnit};
pub use self::align::{Align, FixedAlign, HAlign, VAlign};
pub use self::angle::{Angle, AngleUnit, Quadrant};
pub use self::axes::{Axes, Axis};
pub use self::color::{Color, ColorSpace, Hsl, Hsv, WeightedColor};
pub use self::corners::{Corner, Corners};
pub use self::dir::Dir;
pub use self::ellipse::ellipse;
pub use self::em::Em;
pub use self::fr::Fr;
pub use self::gradient::{
    ConicGradient, Gradient, LinearGradient, RatioOrAngle, Relative,
};
pub use self::length::Length;
pub use self::paint::Paint;
pub use self::path::{Path, PathItem};
pub use self::point::Point;
pub use self::ratio::Ratio;
pub use self::rect::{path_rect, styled_rect};
pub use self::rel::Rel;
pub use self::scalar::Scalar;
pub use self::shape::{Geometry, Shape};
pub use self::sides::{Side, Sides};
pub use self::size::Size;
pub use self::smart::Smart;
pub use self::stroke::{DashLength, DashPattern, FixedStroke, LineCap, LineJoin, Stroke};
pub use self::transform::Transform;

use std::cmp::Ordering;
use std::f64::consts::PI;
use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::iter::Sum;
use std::ops::*;

use ecow::{eco_format, EcoString};

use crate::diag::{bail, StrResult};
use crate::eval::{array, cast, func, scope, ty, Array, Dict, Repr, Value};
use crate::model::{Fold, Resolve, StyleChain};
use crate::util::fmt::format_float;

/// Generic access to a structure's components.
pub trait Get<Index> {
    /// The structure's component type.
    type Component;

    /// Borrow the component for the specified index.
    fn get_ref(&self, index: Index) -> &Self::Component;

    /// Borrow the component for the specified index mutably.
    fn get_mut(&mut self, index: Index) -> &mut Self::Component;

    /// Convenience method for getting a copy of a component.
    fn get(self, index: Index) -> Self::Component
    where
        Self: Sized,
        Self::Component: Copy,
    {
        *self.get_ref(index)
    }

    /// Convenience method for setting a component.
    fn set(&mut self, index: Index, component: Self::Component) {
        *self.get_mut(index) = component;
    }
}

/// A numeric type.
pub trait Numeric:
    Sized
    + Debug
    + Copy
    + PartialEq
    + Neg<Output = Self>
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<f64, Output = Self>
    + Div<f64, Output = Self>
{
    /// The identity element for addition.
    fn zero() -> Self;

    /// Whether `self` is zero.
    fn is_zero(self) -> bool {
        self == Self::zero()
    }

    /// Whether `self` consists only of finite parts.
    fn is_finite(self) -> bool;
}

/// Round a float to two decimal places.
pub fn round_2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}
