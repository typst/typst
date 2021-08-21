use crate::geom::{Size, Spec};

/// A sequence of regions to layout into.
#[derive(Debug, Clone)]
pub struct Regions {
    /// The remaining size of the current region.
    pub current: Size,
    /// The base size for relative sizing.
    pub base: Size,
    /// An iterator of followup regions.
    pub backlog: std::vec::IntoIter<Size>,
    /// The final region that is repeated once the backlog is drained.
    pub last: Option<Size>,
    /// Whether nodes should expand to fill the regions instead of shrinking to
    /// fit the content.
    ///
    /// This property is only handled by nodes that have the ability to control
    /// their own size.
    pub expand: Spec<bool>,
}

impl Regions {
    /// Create a new region sequence with exactly one region.
    pub fn one(size: Size, base: Size, expand: Spec<bool>) -> Self {
        Self {
            current: size,
            base,
            backlog: vec![].into_iter(),
            last: None,
            expand,
        }
    }

    /// Create a new sequence of same-size regions that repeats indefinitely.
    pub fn repeat(size: Size, base: Size, expand: Spec<bool>) -> Self {
        Self {
            current: size,
            base,
            backlog: vec![].into_iter(),
            last: Some(size),
            expand,
        }
    }

    /// Create new regions where all sizes are mapped with `f`.
    pub fn map<F>(&self, mut f: F) -> Self
    where
        F: FnMut(Size) -> Size,
    {
        let mut regions = self.clone();
        regions.mutate(|s| *s = f(*s));
        regions
    }

    /// Whether `current` is a fully sized (untouched) copy of the last region.
    ///
    /// If this is true, calling `next()` will have no effect.
    pub fn in_full_last(&self) -> bool {
        self.backlog.len() == 0 && self.last.map_or(true, |size| self.current == size)
    }

    /// An iterator that returns pairs of `(current, base)` that are equivalent
    /// to what would be produced by calling [`next()`](Self::next) repeatedly
    /// until all regions are exhausted.
    pub fn iter(&self) -> impl Iterator<Item = (Size, Size)> + '_ {
        let first = std::iter::once((self.current, self.base));
        let backlog = self.backlog.as_slice().iter();
        let last = self.last.iter().cycle();
        first.chain(backlog.chain(last).map(|&s| (s, s)))
    }

    /// Advance to the next region if there is any.
    pub fn next(&mut self) {
        if let Some(size) = self.backlog.next().or(self.last) {
            self.current = size;
            self.base = size;
        }
    }

    /// Mutate all contained sizes in place.
    pub fn mutate<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut Size),
    {
        f(&mut self.current);
        f(&mut self.base);
        self.last.as_mut().map(|x| f(x));
        self.backlog.as_mut_slice().iter_mut().for_each(f);
    }
}
