use std::collections::{hash_map::Entry, HashMap};
use std::ops::Deref;

use super::*;

/// Caches layouting artifacts.
#[derive(Default, Debug, Clone)]
pub struct LayoutCache {
    /// Maps from node hashes to the resulting frames and regions in which the
    /// frames are valid. The right hand side of the hash map is a vector of
    /// results because across one or more compilations, multiple different
    /// layouts of the same node may have been requested.
    pub frames: HashMap<u64, Vec<FramesEntry>>,
    /// In how many compilations this cache has been used.
    age: usize,
}

impl LayoutCache {
    /// Create a new, empty layout cache.
    pub fn new() -> Self {
        Self { frames: HashMap::new(), age: 0 }
    }

    /// Clear the cache.
    pub fn clear(&mut self) {
        self.frames.clear();
    }

    /// Retains all elements for which the closure on the level returns `true`.
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(usize) -> bool,
    {
        for (_, entries) in self.frames.iter_mut() {
            entries.retain(|entry| f(entry.level));
        }
    }

    /// Prepare the cache for the next round of compilation
    pub fn turnaround(&mut self) {
        self.age += 1;
        for entry in self.frames.iter_mut().flat_map(|(_, x)| x.iter_mut()) {
            for i in 0 .. (entry.temperature.len() - 1) {
                entry.temperature[i] = entry.temperature[i + 1];
            }
            *entry.temperature.last_mut().unwrap() = 0;
        }
    }

    /// The amount of levels stored in the cache.
    pub fn levels(&self) -> usize {
        self.frames
            .iter()
            .flat_map(|(_, x)| x)
            .map(|entry| entry.level + 1)
            .max()
            .unwrap_or(0)
    }

    /// Fetches the appropriate entry from the cache if there is any.
    pub fn get(
        &mut self,
        hash: u64,
        regions: Regions,
    ) -> Option<Vec<Constrained<Rc<Frame>>>> {
        self.frames.get_mut(&hash).and_then(|frames| {
            for frame in frames {
                let res = frame.check(regions.clone());
                if res.is_some() {
                    return res;
                }
            }

            None
        })
    }

    /// Inserts a new frame set into the cache.
    pub fn insert(
        &mut self,
        hash: u64,
        frames: Vec<Constrained<Rc<Frame>>>,
        level: usize,
    ) {
        let entry = FramesEntry::new(frames, level);
        match self.frames.entry(hash) {
            Entry::Occupied(o) => o.into_mut().push(entry),
            Entry::Vacant(v) => {
                v.insert(vec![entry]);
            }
        }
    }
}

/// Cached frames from past layouting.
#[derive(Debug, Clone)]
pub struct FramesEntry {
    /// The cached frames for a node.
    pub frames: Vec<Constrained<Rc<Frame>>>,
    /// How nested the frame was in the context is was originally appearing in.
    pub level: usize,
    /// How much the element was accessed during the last five compilations, the
    /// most recent one being the last element. `None` variants indicate that
    /// the element is younger than five compilations.
    temperature: [usize; 5],
    /// For how long the element already exists.
    age: usize,
}

impl FramesEntry {
    /// Construct a new instance.
    pub fn new(frames: Vec<Constrained<Rc<Frame>>>, level: usize) -> Self {
        Self {
            frames,
            level,
            temperature: [0; 5],
            age: 1,
        }
    }

    /// Checks if the cached [`Frame`] is valid for the given regions.
    pub fn check(&mut self, mut regions: Regions) -> Option<Vec<Constrained<Rc<Frame>>>> {
        for (i, frame) in self.frames.iter().enumerate() {
            if (i != 0 && !regions.next()) || !frame.constraints.check(&regions) {
                return None;
            }
        }

        self.temperature[4] = self.temperature[4] + 1;

        Some(self.frames.clone())
    }

    /// Get the amount of compilation cycles this item has remained in the
    /// cache.
    pub fn age(&self) -> usize {
        self.age
    }

    /// Get the amount of consecutive cycles in which this item has not
    /// been used.
    pub fn cooldown(&self) -> usize {
        let mut cycle = 0;
        for (i, &temp) in self.temperature.iter().enumerate().rev() {
            if self.age > i {
                if temp > 0 {
                    return self.temperature.len() - 1 - i;
                }
            } else {
                return cycle;
            }

            cycle += 1
        }

        cycle
    }

    /// Whether this element was used in the last compilation cycle.
    pub fn hit(&self) -> bool {
        self.temperature.last().unwrap() != &0
    }
}

/// These constraints describe regions that match them.
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

    fn check(&self, regions: &Regions) -> bool {
        if self.expand != regions.expand {
            return false;
        }

        let base = regions.base.to_spec();
        let current = regions.current.to_spec();

        current.eq_by(&self.min, |&x, y| y.map_or(true, |y| x.fits(y)))
            && current.eq_by(&self.max, |x, y| y.map_or(true, |y| x < &y))
            && current.eq_by(&self.exact, |&x, y| y.map_or(true, |y| x.approx_eq(y)))
            && base.eq_by(&self.base, |&x, y| y.map_or(true, |y| x.approx_eq(y)))
    }

    /// Changes all constraints by adding the `size` to them if they are `Some`.
    pub fn mutate(&mut self, size: Size, regions: &Regions) {
        for spec in std::array::IntoIter::new([
            &mut self.min,
            &mut self.max,
            &mut self.exact,
            &mut self.base,
        ]) {
            if let Some(horizontal) = spec.horizontal.as_mut() {
                *horizontal += size.width;
            }
            if let Some(vertical) = spec.vertical.as_mut() {
                *vertical += size.height;
            }
        }

        self.exact = override_if_some(self.exact, regions.current.to_spec());
        self.base = override_if_some(self.base, regions.base.to_spec());
    }
}

fn override_if_some(
    one: Spec<Option<Length>>,
    other: Spec<Length>,
) -> Spec<Option<Length>> {
    Spec {
        vertical: one.vertical.map(|_| other.vertical),
        horizontal: one.horizontal.map(|_| other.horizontal),
    }
}

/// Carries an item that only applies to certain regions and the constraints
/// that describe these regions.
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

/// Extends length-related options by providing convenience methods for setting
/// minimum and maximum lengths on them, even if they are `None`.
pub trait OptionExt {
    // Sets `other` as the value if the Option is `None` or if it contains a
    // value larger than `other`.
    fn set_min(&mut self, other: Length);
    // Sets `other` as the value if the Option is `None` or if it contains a
    // value smaller than `other`.
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
