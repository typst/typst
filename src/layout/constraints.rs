use std::rc::Rc;

use crate::frame::Frame;
use crate::geom::{Length, Size, Spec};

/// Constrain a frame with constraints.
pub trait Constrain {
    /// Reference-count the frame and wrap it with constraints.
    fn constrain(self, cts: Constraints) -> Constrained<Rc<Frame>>;
}

impl Constrain for Frame {
    fn constrain(self, cts: Constraints) -> Constrained<Rc<Frame>> {
        Constrained::new(Rc::new(self), cts)
    }
}

/// Carries an item that is only valid in certain regions and the constraints
/// that describe these regions.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Constrained<T> {
    /// The item that is only valid if the constraints are fullfilled.
    pub item: T,
    /// Constraints on regions in which the item is valid.
    pub cts: Constraints,
}

impl<T> Constrained<T> {
    /// Constrain an item with constraints.
    pub fn new(item: T, cts: Constraints) -> Self {
        Self { item, cts }
    }
}

/// Describe regions that match them.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Constraints {
    /// The minimum available length in the region.
    pub min: Spec<Option<Length>>,
    /// The maximum available length in the region.
    pub max: Spec<Option<Length>>,
    /// The available length in the region.
    pub exact: Spec<Option<Length>>,
    /// The base length of the region used for relative length resolution.
    pub base: Spec<Option<Length>>,
    /// The expand settings of the region.
    pub expand: Spec<bool>,
}

impl Constraints {
    /// Create a new region constraint.
    pub fn new(expand: Spec<bool>) -> Self {
        Self {
            min: Spec::default(),
            max: Spec::default(),
            exact: Spec::default(),
            base: Spec::default(),
            expand,
        }
    }

    /// Check whether the constraints are fullfilled in a region with the given
    /// properties.
    pub fn check(&self, current: Size, base: Size, expand: Spec<bool>) -> bool {
        self.expand == expand
            && verify(self.min, current, |m, c| c.fits(m))
            && verify(self.max, current, |m, c| m.fits(c))
            && verify(self.exact, current, Length::approx_eq)
            && verify(self.base, base, Length::approx_eq)
    }
}

/// Verify a single constraint.
fn verify(spec: Spec<Option<Length>>, size: Size, f: fn(Length, Length) -> bool) -> bool {
    spec.zip(size).all(|&(opt, s)| opt.map_or(true, |m| f(m, s)))
}
