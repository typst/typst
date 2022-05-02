//! Geometrical primitives.

#[macro_use]
mod macros;
mod align;
mod angle;
mod dir;
mod em;
mod fraction;
mod gen;
mod length;
mod paint;
mod path;
mod point;
mod ratio;
mod rect;
mod relative;
mod scalar;
mod sides;
mod spec;
mod transform;

pub use align::*;
pub use angle::*;
pub use dir::*;
pub use em::*;
pub use fraction::*;
pub use gen::*;
pub use length::*;
pub use paint::*;
pub use path::*;
pub use point::*;
pub use ratio::*;
pub use rect::*;
pub use relative::*;
pub use scalar::*;
pub use sides::*;
pub use spec::*;
pub use transform::*;

use std::cmp::Ordering;
use std::f64::consts::PI;
use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::iter::Sum;
use std::ops::*;

/// Generic access to a structure's components.
pub trait Get<Index> {
    /// The structure's component type.
    type Component;

    /// Return the component for the specified index.
    fn get(self, index: Index) -> Self::Component;

    /// Borrow the component for the specified index mutably.
    fn get_mut(&mut self, index: Index) -> &mut Self::Component;

    /// Convenience method for setting a component.
    fn set(&mut self, index: Index, component: Self::Component) {
        *self.get_mut(index) = component;
    }
}

/// A geometric shape with optional fill and stroke.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Shape {
    /// The shape's geometry.
    pub geometry: Geometry,
    /// The shape's background fill.
    pub fill: Option<Paint>,
    /// The shape's border stroke.
    pub stroke: Option<Stroke>,
}

/// A shape's geometry.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Geometry {
    /// A line to a point (relative to its position).
    Line(Point),
    /// A rectangle with its origin in the topleft corner.
    Rect(Size),
    /// A ellipse with its origin in the topleft corner.
    Ellipse(Size),
    /// A bezier path.
    Path(Path),
}

impl Geometry {
    /// Fill the geometry without a stroke.
    pub fn filled(self, fill: Paint) -> Shape {
        Shape {
            geometry: self,
            fill: Some(fill),
            stroke: None,
        }
    }

    /// Stroke the geometry without a fill.
    pub fn stroked(self, stroke: Stroke) -> Shape {
        Shape {
            geometry: self,
            fill: None,
            stroke: Some(stroke),
        }
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
fn round_2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}
