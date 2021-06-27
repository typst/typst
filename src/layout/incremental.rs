use std::{collections::HashMap, ops::Deref};

use super::*;

/// Caches layouting artifacts.
#[derive(Default, Debug, Clone)]
pub struct LayoutCache {
    /// Maps from node hashes to the resulting frames and regions in which the
    /// frames are valid.
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
        for (_, hash_ident) in self.frames.iter_mut() {
            hash_ident.retain(|entry| f(entry.level));
        }
    }

    /// Amount of items in the cache.
    pub fn len(&self) -> usize {
        self.frames.iter().map(|(_, e)| e.len()).sum()
    }

    /// Prepare the cache for the next round of compilation
    pub fn turnaround(&mut self) {
        self.age += 1;
        for entry in self.frames.iter_mut().flat_map(|(_, x)| x.iter_mut()) {
            for i in 0 .. (entry.temperature.len() - 1) {
                entry.temperature[i] = entry.temperature[i + 1];
            }
            *entry.temperature.last_mut().unwrap() = Some(0);
        }
    }

    /// What is the deepest level in the cache?
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
        if let Some(variants) = self.frames.get_mut(&hash) {
            variants.push(entry);
        } else {
            self.frames.insert(hash, vec![entry]);
        }
    }
}

#[derive(Debug, Clone)]
/// Cached frames from past layouting.
pub struct FramesEntry {
    /// The cached frames for a node.
    pub frames: Vec<Constrained<Rc<Frame>>>,
    /// How nested the frame was in the context is was originally appearing in.
    pub level: usize,
    /// How much the element was accessed during the last five compilations, the
    /// most recent one being the last element. `None` variants indicate that
    /// the element is younger than five compilations.
    temperature: [Option<usize>; 5],
}

impl FramesEntry {
    /// Construct a new instance.
    pub fn new(frames: Vec<Constrained<Rc<Frame>>>, level: usize) -> Self {
        let mut temperature = [None; 5];
        temperature[4] = Some(0);
        Self { frames, level, temperature }
    }

    /// Checks if the cached [`Frame`] is valid for the given regions.
    pub fn check(&mut self, mut regions: Regions) -> Option<Vec<Constrained<Rc<Frame>>>> {
        for (i, frame) in self.frames.iter().enumerate() {
            if (i != 0 && !regions.next()) || !frame.constraints.check(&regions) {
                return None;
            }
        }

        let tmp = self.temperature.get_mut(4).unwrap();
        *tmp = Some(tmp.map_or(1, |x| x + 1));

        Some(self.frames.clone())
    }

    /// Get the amount of compilation cycles this item has remained in the
    /// cache.
    pub fn age(&self) -> usize {
        let mut age = 0;
        for &temp in self.temperature.iter().rev() {
            if temp.is_none() {
                break;
            }
            age += 1;
        }
        age
    }

    /// Get the amount of consecutive cycles in which this item has not
    /// been used.
    pub fn cooldown(&self) -> usize {
        let mut age = 0;
        for (i, &temp) in self.temperature.iter().enumerate().rev() {
            match temp {
                Some(temp) => {
                    if temp > 0 {
                        return self.temperature.len() - 1 - i;
                    }
                }
                None => {
                    return age;
                }
            }
            age += 1
        }

        age
    }

    /// Whether this element was used in the last compilation cycle.
    pub fn hit(&self) -> bool {
        self.temperature.last().unwrap().unwrap_or(0) != 0
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

        let res = current.eq_by(&self.min, |&x, y| y.map_or(true, |y| x.fits(y)))
            && current.eq_by(&self.max, |x, y| y.map_or(true, |y| x < &y))
            && current.eq_by(&self.exact, |&x, y| y.map_or(true, |y| x.approx_eq(y)))
            && base.eq_by(&self.base, |&x, y| y.map_or(true, |y| x.approx_eq(y)));

        res
    }

    /// Changes all constraints by adding the argument to them if they are set.
    pub fn mutate(&mut self, size: Size, regions: &Regions) {
        for x in std::array::IntoIter::new([
            &mut self.min,
            &mut self.max,
            &mut self.exact,
            &mut self.base,
        ]) {
            if let Some(horizontal) = x.horizontal.as_mut() {
                *horizontal += size.width;
            }
            if let Some(vertical) = x.vertical.as_mut() {
                *vertical += size.height;
            }
        }

        self.exact = zip(self.exact, regions.current.to_spec(), |_, o| o);
        self.base = zip(self.base, regions.base.to_spec(), |_, o| o);
    }
}

fn zip<F>(
    one: Spec<Option<Length>>,
    other: Spec<Length>,
    mut f: F,
) -> Spec<Option<Length>>
where
    F: FnMut(Length, Length) -> Length,
{
    Spec {
        vertical: one.vertical.map(|r| f(r, other.vertical)),
        horizontal: one.horizontal.map(|r| f(r, other.horizontal)),
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
