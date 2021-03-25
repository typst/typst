//! Geometrical primitives.

#[macro_use]
mod macros;
mod align;
mod angle;
mod dir;
mod gen;
mod length;
mod linear;
mod path;
mod point;
mod relative;
mod sides;
mod size;
mod spec;

pub use align::*;
pub use angle::*;
pub use dir::*;
pub use gen::*;
pub use length::*;
pub use linear::*;
pub use path::*;
pub use point::*;
pub use relative::*;
pub use sides::*;
pub use size::*;
pub use spec::*;

use std::f64::consts::PI;
use std::fmt::{self, Debug, Display, Formatter};
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
}

/// Switch between the specific and generic representations of a type.
///
/// The generic representation deals with main and cross axes while the specific
/// representation deals with horizontal and vertical axes.
pub trait Switch {
    /// The type of the other version.
    type Other;

    /// The other version of this type based on the current main axis.
    fn switch(self, main: SpecAxis) -> Self::Other;
}
