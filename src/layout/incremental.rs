use std::collections::HashMap;

use super::*;

/// Caches layouting artifacts.
#[derive(Default, Debug, Clone)]
pub struct LayoutCache {
    /// Maps from node hashes to the resulting frames and regions in which the
    /// frames are valid.
    pub frames: HashMap<u64, FramesEntry>,
}

impl LayoutCache {
    /// Create a new, empty layout cache.
    pub fn new() -> Self {
        Self { frames: HashMap::new() }
    }

    /// Clear the cache.
    pub fn clear(&mut self) {
        self.frames.clear();
    }
}

#[derive(Debug, Clone)]
/// Cached frames from past layouting.
pub struct FramesEntry {
    /// The regions in which these frames are valid.
    pub regions: Regions,
    /// The cached frames for a node.
    pub frames: Vec<Frame>,
}

#[derive(Debug, Copy, Clone)]
pub struct RegionConstraint {
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

impl RegionConstraint {
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

    pub fn set_base_using_linears(
        &mut self,
        size: Spec<Option<Linear>>,
        regions: &Regions,
    ) {
        // The full sizes need to be equal if there is a relative component in the sizes.
        if size.horizontal.map_or(false, |l| l.is_relative()) {
            self.base.horizontal = Some(regions.base.width);
        }
        if size.vertical.map_or(false, |l| l.is_relative()) {
            self.base.vertical = Some(regions.base.height);
        }
    }

    fn check(&self, mut regions: Regions, amount: usize) -> bool {
        if self.expand != regions.expand {
            return false;
        }

        for _ in 0 .. amount {
            let base = regions.base.to_spec();
            let current = regions.current.to_spec();

            let valid = current.all(&self.min, |x, y| y.map_or(true, |y| x >= &y))
                && current.all(&self.max, |x, y| y.map_or(true, |y| x < &y))
                && current.all(&self.exact, |x, y| y.map_or(true, |y| x == &y))
                && base.all(&self.base, |x, y| y.map_or(true, |y| x == &y));

            if !valid {
                return false;
            }

            regions.next();
        }

        true
    }
}
