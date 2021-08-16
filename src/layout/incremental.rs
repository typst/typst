#[cfg(feature = "layout-cache")]
use std::collections::HashMap;
use std::ops::Deref;

use crate::util::OptionExt;

use decorum::N32;
use rand;

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
    /// What cache eviction policy should be used.
    pub policy: EvictionStrategy,
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
        self.frames
            .entry(hash)
            .or_default()
            .push(FramesEntry::new(frames, level));
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
            if entry.temperature[0] > 0 {
                entry.used_cycles += 1;
            }

            let mut temperature = [0; 5];
            temperature[entry.temperature.len() - 1] =
                entry.temperature[entry.temperature.len() - 1];

            for i in 0 .. (entry.temperature.len() - 1) {
                if i + 1 == entry.temperature.len() - 1 {
                    temperature[i + 1] += entry.temperature[i];
                } else {
                    temperature[i + 1] = entry.temperature[i];
                }
            }
            entry.temperature = temperature;
            entry.age += 1;
        }

        self.evict();

        self.frames.retain(|_, v| !v.is_empty());
    }

    fn evict(&mut self) {
        let max_size = 20;

        match self.policy {
            EvictionStrategy::LeastRecentlyUsed => {
                let last_access = if let Some(max) =
                    self.frames.iter().flat_map(|(_, f)| f).map(|f| f.cooldown()).max()
                {
                    max
                } else {
                    return;
                };

                self.frames
                    .iter_mut()
                    .for_each(|(_, f)| f.retain(|e| e.cooldown() < last_access));
                if self.len() > max_size {
                    self.evict()
                }
            }
            EvictionStrategy::LeastFrequentlyUsed => {
                let mut usage_frequencies = self
                    .frames
                    .iter()
                    .flat_map(|(_, f)| f)
                    .map(|f| N32::from(f.total() as f32) / N32::from(f.age() as f32))
                    .collect::<Vec<_>>();

                usage_frequencies.sort_unstable();

                if usage_frequencies.len() <= max_size {
                    return;
                }

                let threshold = usage_frequencies[max_size];

                for (_, entries) in self.frames.iter_mut() {
                    entries.retain(|f| {
                        N32::from(f.total() as f32) / N32::from(f.age() as f32)
                            < threshold
                    });
                }
            }
            EvictionStrategy::Random => {
                let len = self.len();
                if len <= max_size {
                    return;
                }

                // Fraction of items that should be kept.
                let threshold = max_size as f32 / len as f32;
                for (_, entries) in self.frames.iter_mut() {
                    entries.retain(|_| rand::random::<f32>() > threshold);
                }
            }
            EvictionStrategy::Patterns => {
                let should_keep: Vec<bool> = self
                    .frames
                    .iter()
                    .flat_map(|(_, f)| f)
                    .map(|f| f.properties().should_keep())
                    .collect();
                let kept = should_keep.iter().filter(|&&b| b).count();

                let mut usage_frequencies = self
                    .frames
                    .iter()
                    .flat_map(|(_, f)| f)
                    .zip(should_keep)
                    .filter(|(_, keep)| !keep)
                    .map(|(f, _)| N32::from(f.total() as f32) / N32::from(f.age() as f32))
                    .collect::<Vec<_>>();

                usage_frequencies.sort_unstable();

                let remaining = max_size - kept.min(max_size);
                if usage_frequencies.len() <= remaining {
                    return;
                }

                let threshold = usage_frequencies[remaining];

                for (_, entries) in self.frames.iter_mut() {
                    entries.retain(|f| {
                        f.properties().should_keep()
                            || N32::from(f.total() as f32) / N32::from(f.age() as f32)
                                < threshold
                    });
                }
            }
            EvictionStrategy::None => {}
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
    /// most recent one being the first element. The last element will collect
    /// all usages that are farther in the past.
    temperature: [usize; 5],
    /// Amount of cycles in which the element has been used at all.
    used_cycles: usize,
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
            used_cycles: 0,
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
        let id = if self.age > self.temperature.len() {
            self.temperature.len()
        } else {
            self.age
        };
        for &temp in &self.temperature[.. id] {
            if temp > 0 {
                return cycle;
            }
            cycle += 1;
        }
        cycle
    }

    /// Get the total amount of hits over the lifetime of this item.
    pub fn total(&self) -> usize {
        self.temperature.iter().sum()
    }

    pub fn properties(&self) -> PatternProperties {
        let mut all_zeros = true;
        let mut multi_use = false;
        let mut decreasing = true;
        let mut sparse = false;
        let mut abandoned = false;

        let mut last = None;
        let mut partially_sparse = false;
        let mut all_same = true;

        for (i, &temp) in
            self.temperature[.. self.temperature.len() - 1].iter().enumerate()
        {
            if temp != 0 {
                all_zeros = false;
            } else if !all_zeros {
                partially_sparse = true;
            }

            if temp == 0 && partially_sparse {
                sparse = true;
            }

            if temp == 0 && i == 1 {
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

        if self.age > self.temperature.len() {
            if self.age - (self.temperature.len() - 1) < *self.temperature.last().unwrap()
            {
                multi_use = true;
            }
        }

        if self.temperature.last().unwrap() > &0 {
            all_zeros = false;
        }

        decreasing = decreasing && !all_same;

        PatternProperties {
            mature: self.age >= self.temperature.len(),
            hit: self.temperature[0] >= 1,
            top_level: self.level == 0,
            all_zeros,
            multi_use,
            decreasing,
            sparse,
            abandoned,
        }
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

/// Cache eviction strategies.
#[cfg(feature = "layout-cache")]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum EvictionStrategy {
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

impl Default for EvictionStrategy {
    fn default() -> Self {
        Self::Patterns
    }
}

/// Possible descisions on eviction that may arise from the pattern type.
#[cfg(feature = "layout-cache")]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum EvictionVerdict {
    /// Always evict.
    Evict,
    /// The item may be evicted.
    MayEvict,
    /// The item should be kept.
    Keep,
}

/// Describes the properties that this entry's temperature array has.
#[cfg(feature = "layout-cache")]
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
    pub fn should_keep(&self) -> bool {
        if self.top_level && !self.mature {
            // Keep an undo stack.
            true
        } else if self.all_zeros && !self.mature {
            // Keep the most recently created items, even if they have not yet
            // been used.
            true
        } else if self.multi_use && !self.abandoned {
            true
        } else if self.hit {
            true
        } else if self.sparse {
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_frame() -> Vec<Constrained<Rc<Frame>>> {
        vec![Constrained {
            item: Rc::new(Frame::new(Size::zero(), Length::zero())),
            constraints: Constraints::new(Spec::splat(false)),
        }]
    }

    fn zero_region() -> Regions {
        Regions::one(Size::zero(), Spec::splat(false))
    }

    #[test]
    fn test_temperature() {
        let mut cache = LayoutCache::new();
        let zero_region = zero_region();
        cache.policy = EvictionStrategy::None;
        cache.insert(0, empty_frame(), 0);

        let entry = cache.frames.get(&0).unwrap().first().unwrap();
        assert_eq!(entry.age(), 1);
        assert_eq!(entry.temperature, [0, 0, 0, 0, 0]);
        assert_eq!(entry.used_cycles, 0);
        assert_eq!(entry.level, 0);

        cache.get(0, &zero_region).unwrap();
        let entry = cache.frames.get(&0).unwrap().first().unwrap();
        assert_eq!(entry.age(), 1);
        assert_eq!(entry.temperature, [1, 0, 0, 0, 0]);

        println!("{:?}", entry.properties().should_keep());

        cache.turnaround();
        let entry = cache.frames.get(&0).unwrap().first().unwrap();
        assert_eq!(entry.age(), 2);
        assert_eq!(entry.temperature, [0, 1, 0, 0, 0]);
        assert_eq!(entry.used_cycles, 1);

        cache.get(0, &zero_region).unwrap();
        for _ in 0 .. 4 {
            let entry = cache.frames.get(&0).unwrap().first().unwrap();
            println!("{:?}", entry.properties().should_keep());
            cache.turnaround();
        }

        let entry = cache.frames.get(&0).unwrap().first().unwrap();
        assert_eq!(entry.age(), 6);
        assert_eq!(entry.temperature, [0, 0, 0, 0, 2]);
        assert_eq!(entry.used_cycles, 2);
    }

    #[test]
    fn test_properties() {
        let mut cache = LayoutCache::new();
        cache.policy = EvictionStrategy::None;
        cache.insert(0, empty_frame(), 1);

        let props = cache.frames.get(&0).unwrap().first().unwrap().properties();
        assert_eq!(props.top_level, false);
        assert_eq!(props.mature, false);
        assert_eq!(props.multi_use, false);
        assert_eq!(props.hit, false);
        assert_eq!(props.decreasing, false);
        assert_eq!(props.sparse, false);
        assert_eq!(props.abandoned, true);
        assert_eq!(props.all_zeros, true);
        assert_eq!(props.should_keep(), true);
    }
}
