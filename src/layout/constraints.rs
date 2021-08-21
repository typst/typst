use super::*;

/// Carries an item that is only valid in certain regions and the constraints
/// that describe these regions.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Constrained<T> {
    /// The item that is only valid if the constraints are fullfilled.
    pub item: T,
    /// Constraints on regions in which the item is valid.
    pub constraints: Constraints,
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
        let current = current.to_spec();
        let base = base.to_spec();
        self.expand == expand
            && current.eq_by(&self.min, |x, y| y.map_or(true, |y| x.fits(y)))
            && current.eq_by(&self.max, |x, y| y.map_or(true, |y| x < &y))
            && current.eq_by(&self.exact, |x, y| y.map_or(true, |y| x.approx_eq(y)))
            && base.eq_by(&self.base, |x, y| y.map_or(true, |y| x.approx_eq(y)))
    }

    /// Set the appropriate base constraints for linear width and height sizing.
    pub fn set_base_if_linear(&mut self, base: Size, sizing: Spec<Option<Linear>>) {
        // The full sizes need to be equal if there is a relative component in the sizes.
        if sizing.x.map_or(false, |l| l.is_relative()) {
            self.base.x = Some(base.w);
        }
        if sizing.y.map_or(false, |l| l.is_relative()) {
            self.base.y = Some(base.h);
        }
    }
}
