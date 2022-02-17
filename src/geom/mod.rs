//! Geometrical primitives.

#[macro_use]
mod macros;
mod align;
mod angle;
mod dir;
mod em;
mod fr;
mod gen;
mod length;
mod linear;
mod paint;
mod path;
mod point;
mod relative;
mod scalar;
mod sides;
mod spec;
mod transform;

pub use align::*;
pub use angle::*;
pub use dir::*;
pub use em::*;
pub use fr::*;
pub use gen::*;
pub use length::*;
pub use linear::*;
pub use paint::*;
pub use path::*;
pub use point::*;
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

/// Round a float to two decimal places.
fn round_2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}
