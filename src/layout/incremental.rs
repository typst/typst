#[cfg(feature = "layout-cache")]
use std::collections::{hash_map::Entry, HashMap};
use std::ops::Deref;

use super::*;

/// Caches layouting artifacts.
///
/// _This is only available when the `layout-cache` feature is enabled._
#[cfg(feature = "layout-cache")]
#[derive(Default, Clone)]
pub struct LayoutCache {
    /// Maps from node hashes to the resulting frames and regions in which the
    /// frames are valid. The right hand side of the hash map is a vector of
    /// results because across one or more compilations, multiple different
    /// layouts of the same node may have been requested.
    frames: HashMap<u64, Vec<FramesEntry>>,
    /// In how many compilations this cache has been used.
    age: usize,
}

#[cfg(feature = "layout-cache")]
impl LayoutCache {
    /// Create a new, empty layout cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Amount of items in the cache.
    pub fn len(&self) -> usize {
        self.frames.values().map(Vec::len).sum()
    }

    /// The number of levels stored in the cache.
    pub fn levels(&self) -> usize {
        self.entries().map(|entry| entry.level + 1).max().unwrap_or(0)
    }

    /// An iterator over all entries in the cache.
    pub fn entries(&self) -> impl Iterator<Item = &FramesEntry> + '_ {
        self.frames.values().flatten()
    }

    /// Fetch matching cached frames if there are any.
    pub fn get(
        &mut self,
        hash: u64,
        regions: &Regions,
    ) -> Option<Vec<Constrained<Rc<Frame>>>> {
        let entries = self.frames.get_mut(&hash)?;
        for entry in entries {
            if let Some(frames) = entry.check(regions) {
                return Some(frames);
            }
        }
        None
    }

    /// Insert a new frame entry into the cache.
    pub fn insert(
        &mut self,
        hash: u64,
        frames: Vec<Constrained<Rc<Frame>>>,
        level: usize,
    ) {
        let entry = FramesEntry::new(frames, level);
        match self.frames.entry(hash) {
            Entry::Occupied(occupied) => occupied.into_mut().push(entry),
            Entry::Vacant(vacant) => {
                vacant.insert(vec![entry]);
            }
        }
    }

    /// Clear the cache.
    pub fn clear(&mut self) {
        self.frames.clear();
    }

    /// Retain all elements for which the closure on the level returns `true`.
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(usize) -> bool,
    {
        for entries in self.frames.values_mut() {
            entries.retain(|entry| f(entry.level));
        }
    }

    /// Prepare the cache for the next round of compilation.
    pub fn turnaround(&mut self) {
        self.age += 1;
        for entry in self.frames.values_mut().flatten() {
            for i in 0 .. (entry.temperature.len() - 1) {
                entry.temperature[i + 1] = entry.temperature[i];
            }
            entry.temperature[0] = 0;
            entry.age += 1;
        }
    }
}

/// Cached frames from past layouting.
///
/// _This is only available when the `layout-cache` feature is enabled._
#[cfg(feature = "layout-cache")]
#[derive(Debug, Clone)]
pub struct FramesEntry {
    /// The cached frames for a node.
    frames: Vec<Constrained<Rc<Frame>>>,
    /// How nested the frame was in the context is was originally appearing in.
    level: usize,
    /// For how long the element already exists.
    age: usize,
    /// How much the element was accessed during the last five compilations, the
    /// most recent one being the first element.
    temperature: [usize; 5],
}

#[cfg(feature = "layout-cache")]
impl FramesEntry {
    /// Construct a new instance.
    pub fn new(frames: Vec<Constrained<Rc<Frame>>>, level: usize) -> Self {
        Self {
            frames,
            level,
            age: 1,
            temperature: [0; 5],
        }
    }

    /// Checks if the cached frames are valid in the given regions and returns
    /// them if so.
    pub fn check(&mut self, regions: &Regions) -> Option<Vec<Constrained<Rc<Frame>>>> {
        let mut iter = regions.iter();
        for frame in &self.frames {
            let (current, base) = iter.next()?;
            if !frame.constraints.check(current, base, regions.expand) {
                return None;
            }
        }

        self.temperature[0] += 1;
        Some(self.frames.clone())
    }

    /// How nested the frame was in the context is was originally appearing in.
    pub fn level(&self) -> usize {
        self.level
    }

    /// The number of compilation cycles this item has remained in the cache.
    pub fn age(&self) -> usize {
        self.age
    }

    /// Whether this element was used in the last compilation cycle.
    pub fn hit(&self) -> bool {
        self.temperature[0] != 0
    }

    /// The amount of consecutive cycles in which this item has not been used.
    pub fn cooldown(&self) -> usize {
        let mut cycle = 0;
        for &temp in &self.temperature[.. self.age] {
            if temp > 0 {
                return cycle;
            }
            cycle += 1;
        }
        cycle
    }
}

/// Carries an item that is only valid in certain regions and the constraints
/// that describe these regions.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Constrained<T> {
    /// The item that is only valid if the constraints are fullfilled.
    pub item: T,
    /// Constraints on regions in which the item is valid.
    pub constraints: Constraints,
}

impl<T> Deref for Constrained<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.item
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
        let current = current.to_spec();
        let base = base.to_spec();
        self.expand == expand
            && current.eq_by(&self.min, |x, y| y.map_or(true, |y| x.fits(y)))
            && current.eq_by(&self.max, |x, y| y.map_or(true, |y| x < &y))
            && current.eq_by(&self.exact, |x, y| y.map_or(true, |y| x.approx_eq(y)))
            && base.eq_by(&self.base, |x, y| y.map_or(true, |y| x.approx_eq(y)))
    }

    /// Set the appropriate base constraints for (relative) width and height
    /// metrics, respectively.
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

    /// Changes all constraints by adding the `size` to them if they are `Some`.
    pub fn inflate(&mut self, size: Size, regions: &Regions) {
        for spec in [
            &mut self.min,
            &mut self.max,
            &mut self.exact,
            &mut self.base,
        ] {
            if let Some(horizontal) = spec.horizontal.as_mut() {
                *horizontal += size.width;
            }
            if let Some(vertical) = spec.vertical.as_mut() {
                *vertical += size.height;
            }
        }

        let current = regions.current.to_spec();
        let base = regions.base.to_spec();

        self.exact.horizontal.and_set(Some(current.horizontal));
        self.exact.vertical.and_set(Some(current.vertical));
        self.base.horizontal.and_set(Some(base.horizontal));
        self.base.vertical.and_set(Some(base.vertical));
    }
}
