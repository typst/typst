//! Geometrical primitives.

#[macro_use]
mod macros;
mod abs;
mod align;
mod angle;
mod axes;
mod corners;
mod dir;
mod ellipse;
mod em;
mod fr;
mod length;
mod mix;
mod paint;
mod path;
mod point;
mod ratio;
mod rel;
mod rounded;
mod scalar;
mod shape;
mod sides;
mod size;
mod smart;
mod stroke;
mod transform;

pub use self::abs::*;
pub use self::align::*;
pub use self::angle::*;
pub use self::axes::*;
pub use self::corners::*;
pub use self::dir::*;
pub use self::ellipse::*;
pub use self::em::*;
pub use self::fr::*;
pub use self::length::*;
pub use self::mix::*;
pub use self::paint::*;
pub use self::path::*;
pub use self::point::*;
pub use self::ratio::*;
pub use self::rel::*;
pub use self::rounded::*;
pub use self::scalar::*;
pub use self::shape::*;
pub use self::sides::*;
pub use self::size::*;
pub use self::smart::*;
pub use self::stroke::*;
pub use self::transform::*;

use std::cmp::Ordering;
use std::f64::consts::PI;
use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::iter::Sum;
use std::ops::*;

use crate::diag::StrResult;
use crate::eval::{array, cast_from_value, cast_to_value, Cast, CastInfo, Dict, Value};
use crate::model::{Fold, Resolve, StyleChain};

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
