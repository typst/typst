use std::{collections::HashMap, ops::Deref};

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
    /// The cached frames for a node.
    pub frames: Vec<Constrained<Frame>>,
}

impl FramesEntry {
    pub fn check(&self, mut regions: Regions) -> Option<Vec<Constrained<Frame>>> {
        for (i, frame) in self.frames.iter().enumerate() {
            if (i != 0 && !regions.next()) || !frame.constraints.check(&regions) {
                return None;
            }
        }

        Some(self.frames.clone())
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
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

    fn check(&self, regions: &Regions) -> bool {
        if self.expand != regions.expand {
            return false;
        }

        let base = regions.base.to_spec();
        let current = regions.current.to_spec();

        current.eq_by(&self.min, |x, y| y.map_or(true, |y| x >= &y))
            && current.eq_by(&self.max, |x, y| y.map_or(true, |y| x < &y))
            && current.eq_by(&self.exact, |x, y| y.map_or(true, |y| x == &y))
            && base.eq_by(&self.base, |x, y| y.map_or(true, |y| x == &y))
    }

    pub fn map(&mut self, size: Size) {
        for x in &mut [self.min, self.max, self.exact, self.base] {
            if let Some(horizontal) = x.horizontal.as_mut() {
                *horizontal += size.width;
            }
            if let Some(vertical) = x.vertical.as_mut() {
                *vertical += size.height;
            }
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Constrained<T> {
    pub item: T,
    pub constraints: Constraints,
}

impl<T> Deref for Constrained<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.item
    }
}

pub trait OptionExt {
    fn set_min(&mut self, other: Length);
    fn set_max(&mut self, other: Length);
}

impl OptionExt for Option<Length> {
    fn set_min(&mut self, other: Length) {
        match self {
            Some(x) => x.set_min(other),
            None => *self = Some(other),
        }
    }

    fn set_max(&mut self, other: Length) {
        match self {
            Some(x) => x.set_max(other),
            None => *self = Some(other),
        }
    }
}
