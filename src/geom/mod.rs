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
mod sides;
mod size;
mod spec;

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
pub use sides::*;
pub use size::*;
pub use spec::*;

use std::f64::consts::PI;
use std::fmt::{self, Debug, Formatter};
use std::iter::Sum;
use std::ops::*;

use decorum::N64;
use serde::{Deserialize, Serialize};

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
