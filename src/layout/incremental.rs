use std::cmp::Reverse;
use std::collections::HashMap;
use std::rc::Rc;

use itertools::Itertools;

use super::{Constrained, Regions};
use crate::frame::Frame;
use crate::geom::Scalar;

const TEMP_LEN: usize = 4;

/// Caches layouting artifacts.
///
/// _This is only available when the `layout-cache` feature is enabled._
#[derive(Default, Clone)]
pub struct LayoutCache {
    /// Maps from node hashes to the resulting frames and regions in which the
    /// frames are valid. The right hand side of the hash map is a vector of
    /// results because across one or more compilations, multiple different
    /// layouts of the same node may have been requested.
    frames: HashMap<u64, Vec<FramesEntry>>,
    /// In how many compilations this cache has been used.
    age: usize,
    /// What cache eviction policy should be used.
    policy: EvictionPolicy,
    /// The maximum number of entries this cache should have. Can be exceeded if
    /// there are more must-keep entries.
    max_size: usize,
}

impl LayoutCache {
    /// Create a new, empty layout cache.
    pub fn new(policy: EvictionPolicy, max_size: usize) -> Self {
        Self {
            frames: HashMap::default(),
            age: 0,
            policy,
            max_size,
        }
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.frames.values().all(|entry| entry.is_empty())
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
        self.frames
            .get_mut(&hash)?
            .iter_mut()
            .find_map(|entry| entry.lookup(regions))
    }

    /// Insert a new frame entry into the cache.
    pub fn insert(&mut self, hash: u64, entry: FramesEntry) {
        self.frames.entry(hash).or_default().push(entry);
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
        for entries in self.frames.values_mut() {
            entries.retain(|entry| f(entry.level));
        }
    }

    /// Prepare the cache for the next round of compilation.
    pub fn turnaround(&mut self) {
        self.age += 1;
        for entry in self.frames.values_mut().flatten() {
            if entry.temperature[0] > 0 {
                entry.used_cycles += 1;
            }

            let last = *entry.temperature.last().unwrap();
            for i in (1 .. TEMP_LEN).rev() {
                entry.temperature[i] = entry.temperature[i - 1];
            }

            entry.temperature[0] = 0;
            entry.ancient_hits += last as usize;
            entry.age += 1;
        }

        self.evict();
        self.frames.retain(|_, v| !v.is_empty());
    }

    /// Evict the cache according to the policy.
    fn evict(&mut self) {
        let len = self.len();
        if len <= self.max_size {
            return;
        }

        match self.policy {
            EvictionPolicy::LeastRecentlyUsed => {
                // We find the element with the largest cooldown that cannot fit
                // anymore.
                let threshold = self
                    .entries()
                    .map(|f| Reverse(f.cooldown()))
                    .k_smallest(len - self.max_size)
                    .last()
                    .unwrap()
                    .0;

                for entries in self.frames.values_mut() {
                    entries.retain(|f| f.cooldown() < threshold);
                }
            }
            EvictionPolicy::LeastFrequentlyUsed => {
                let threshold = self
                    .entries()
                    .map(|f| Scalar(f.hits() as f64 / f.age() as f64))
                    .k_smallest(len - self.max_size)
                    .last()
                    .unwrap()
                    .0;

                for entries in self.frames.values_mut() {
                    entries.retain(|f| f.hits() as f64 / f.age() as f64 > threshold);
                }
            }
            EvictionPolicy::Random => {
                // Fraction of items that should be kept.
                let threshold = self.max_size as f64 / len as f64;
                for entries in self.frames.values_mut() {
                    entries.retain(|_| rand::random::<f64>() > threshold);
                }
            }
            EvictionPolicy::Patterns => {
                let kept = self.entries().filter(|f| f.properties().must_keep()).count();

                let remaining_capacity = self.max_size - kept.min(self.max_size);
                if len - kept <= remaining_capacity {
                    return;
                }

                let threshold = self
                    .entries()
                    .filter(|f| !f.properties().must_keep())
                    .map(|f| Scalar(f.hits() as f64 / f.age() as f64))
                    .k_smallest((len - kept) - remaining_capacity)
                    .last()
                    .unwrap()
                    .0;

                for entries in self.frames.values_mut() {
                    entries.retain(|f| {
                        f.properties().must_keep()
                            || f.hits() as f64 / f.age() as f64 > threshold
                    });
                }
            }
            EvictionPolicy::None => {}
        }
    }
}

/// Cached frames from past layouting.
///
/// _This is only available when the `layout-cache` feature is enabled._
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
    temperature: [u8; TEMP_LEN],
    /// All past usages that do not fit in the temperature array.
    ancient_hits: usize,
    /// Amount of cycles in which the element has been used at all.
    used_cycles: usize,
}

impl FramesEntry {
    /// Construct a new instance.
    pub fn new(frames: Vec<Constrained<Rc<Frame>>>, level: usize) -> Self {
        Self {
            frames,
            level,
            age: 1,
            temperature: [0; TEMP_LEN],
            ancient_hits: 0,
            used_cycles: 0,
        }
    }

    /// Checks if the cached frames are valid in the given regions and returns
    /// them if so.
    pub fn lookup(&mut self, regions: &Regions) -> Option<Vec<Constrained<Rc<Frame>>>> {
        self.check(regions).then(|| {
            self.temperature[0] = self.temperature[0].saturating_add(1);
            self.frames.clone()
        })
    }

    /// Checks if the cached frames are valid in the given regions.
    pub fn check(&self, regions: &Regions) -> bool {
        let mut iter = regions.iter();
        self.frames.iter().all(|frame| {
            iter.next().map_or(false, |(current, base)| {
                frame.cts.check(current, base, regions.expand)
            })
        })
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

    /// Get the total amount of hits over the lifetime of this item.
    pub fn hits(&self) -> usize {
        self.temperature.into_iter().map(usize::from).sum::<usize>() + self.ancient_hits
    }

    /// The amount of consecutive cycles in which this item has not been used.
    pub fn cooldown(&self) -> usize {
        let mut cycle = 0;
        for &temp in &self.temperature[.. self.age.min(TEMP_LEN)] {
            if temp > 0 {
                return cycle;
            }
            cycle += 1;
        }
        cycle
    }

    /// Properties that describe how this entry's temperature evolved.
    pub fn properties(&self) -> PatternProperties {
        let mut all_zeros = true;
        let mut multi_use = false;
        let mut decreasing = true;
        let mut sparse = false;
        let mut abandoned = false;

        let mut last = None;
        let mut all_same = true;

        for (i, &temp) in self.temperature.iter().enumerate() {
            if temp == 0 && !all_zeros {
                sparse = true;
            }

            if temp != 0 {
                all_zeros = false;
            }

            if all_zeros && i == 1 {
                abandoned = true;
            }

            if temp > 1 {
                multi_use = true;
            }

            if let Some(prev) = last {
                if prev > temp {
                    decreasing = false;
                }

                if temp != prev {
                    all_same = false;
                }
            }

            last = Some(temp);
        }

        if self.age > TEMP_LEN && self.age - TEMP_LEN <= self.ancient_hits {
            multi_use = true;
        }

        if self.ancient_hits > 0 {
            all_zeros = false;
        }

        PatternProperties {
            mature: self.age > TEMP_LEN,
            hit: self.temperature[0] >= 1,
            top_level: self.level == 0,
            all_zeros,
            multi_use,
            decreasing: decreasing && !all_same,
            sparse,
            abandoned,
        }
    }
}

/// Cache eviction strategies.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum EvictionPolicy {
    /// Evict the least recently used item.
    LeastRecentlyUsed,
    /// Evict the least frequently used item.
    LeastFrequentlyUsed,
    /// Evict randomly.
    Random,
    /// Use the pattern verdicts.
    Patterns,
    /// Do not evict.
    None,
}

impl Default for EvictionPolicy {
    fn default() -> Self {
        Self::Patterns
    }
}

/// Describes the properties that this entry's temperature array has.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct PatternProperties {
    /// There only are zero values.
    pub all_zeros: bool,
    /// The entry exists for more or equal time as the temperature array is long.
    pub mature: bool,
    /// The entry was used more than one time in at least one compilation.
    pub multi_use: bool,
    /// The entry was used in the last compilation.
    pub hit: bool,
    /// The temperature is monotonously decreasing in non-terminal temperature fields.
    pub decreasing: bool,
    /// There are zero temperatures after non-zero temperatures.
    pub sparse: bool,
    /// There are multiple zero temperatures at the front of the temperature array.
    pub abandoned: bool,
    /// If the item is on the top level.
    pub top_level: bool,
}

impl PatternProperties {
    /// Check if it is vital to keep an entry based on its properties.
    pub fn must_keep(&self) -> bool {
        // Keep an undo stack.
        (self.top_level && !self.mature)
        // Keep the most recently created items, even if they have not yet
        // been used.
        || (self.all_zeros && !self.mature)
        || (self.multi_use && !self.abandoned)
        || self.hit
        || self.sparse
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::{Size, Spec};
    use crate::layout::Constraints;

    fn empty_frames() -> Vec<Constrained<Rc<Frame>>> {
        vec![Constrained {
            item: Rc::new(Frame::default()),
            cts: Constraints::new(Spec::splat(false)),
        }]
    }

    fn zero_regions() -> Regions {
        Regions::one(Size::zero(), Size::zero(), Spec::splat(false))
    }

    #[test]
    fn test_incremental_temperature() {
        let mut cache = LayoutCache::new(EvictionPolicy::None, 20);
        let regions = zero_regions();
        cache.policy = EvictionPolicy::None;
        cache.insert(0, FramesEntry::new(empty_frames(), 0));

        let entry = cache.frames.get(&0).unwrap().first().unwrap();
        assert_eq!(entry.age(), 1);
        assert_eq!(entry.temperature, [0, 0, 0, 0]);
        assert_eq!(entry.ancient_hits, 0);
        assert_eq!(entry.used_cycles, 0);
        assert_eq!(entry.level, 0);

        cache.get(0, &regions).unwrap();
        let entry = cache.frames.get(&0).unwrap().first().unwrap();
        assert_eq!(entry.age(), 1);
        assert_eq!(entry.temperature, [1, 0, 0, 0]);
        assert_eq!(entry.ancient_hits, 0);

        cache.turnaround();
        let entry = cache.frames.get(&0).unwrap().first().unwrap();
        assert_eq!(entry.age(), 2);
        assert_eq!(entry.temperature, [0, 1, 0, 0]);
        assert_eq!(entry.ancient_hits, 0);
        assert_eq!(entry.used_cycles, 1);

        cache.get(0, &regions).unwrap();
        for _ in 0 .. 4 {
            cache.turnaround();
        }

        let entry = cache.frames.get(&0).unwrap().first().unwrap();
        assert_eq!(entry.age(), 6);
        assert_eq!(entry.temperature, [0, 0, 0, 0]);
        assert_eq!(entry.ancient_hits, 2);
        assert_eq!(entry.used_cycles, 2);
    }

    #[test]
    fn test_incremental_properties() {
        let mut cache = LayoutCache::new(EvictionPolicy::None, 20);
        cache.policy = EvictionPolicy::None;
        cache.insert(0, FramesEntry::new(empty_frames(), 1));

        let props = cache.frames.get(&0).unwrap().first().unwrap().properties();
        assert_eq!(props.top_level, false);
        assert_eq!(props.mature, false);
        assert_eq!(props.multi_use, false);
        assert_eq!(props.hit, false);
        assert_eq!(props.decreasing, false);
        assert_eq!(props.sparse, false);
        assert_eq!(props.abandoned, true);
        assert_eq!(props.all_zeros, true);
        assert_eq!(props.must_keep(), true);
    }
}
