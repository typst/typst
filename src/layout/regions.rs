use crate::geom::{Length, Size, Spec};

/// A sequence of regions to layout into.
#[derive(Debug, Clone)]
pub struct Regions {
    /// The remaining size of the current region.
    pub current: Size,
    /// The base size for relative sizing.
    pub base: Size,
    /// The height of followup regions. The width is the same for all regions.
    pub backlog: std::vec::IntoIter<Length>,
    /// The height of the final region that is repeated once the backlog is
    /// drained. The width is the same for all regions.
    pub last: Option<Length>,
    /// Whether nodes should expand to fill the regions instead of shrinking to
    /// fit the content.
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
            last: Some(size.y),
            expand,
        }
    }

    /// Create new regions where all sizes are mapped with `f`.
    ///
    /// Note that since all regions must have the same width, the width returned
    /// by `f` is ignored for the backlog and the final region.
    pub fn map<F>(&self, mut f: F) -> Self
    where
        F: FnMut(Size) -> Size,
    {
        let x = self.current.x;
        Self {
            current: f(self.current),
            base: f(self.base),
            backlog: self
                .backlog
                .as_slice()
                .iter()
                .map(|&y| f(Size::new(x, y)).y)
                .collect::<Vec<_>>()
                .into_iter(),
            last: self.last.map(|y| f(Size::new(x, y)).y),
            expand: self.expand,
        }
    }

    /// Whether the current region is full and a region break is called for.
    pub fn is_full(&self) -> bool {
        Length::zero().fits(self.current.y) && !self.in_last()
    }

    /// Whether `current` is the last usable region.
    ///
    /// If this is true, calling `next()` will have no effect.
    pub fn in_last(&self) -> bool {
        self.backlog.len() == 0
            && self.last.map_or(true, |height| self.current.y == height)
    }

    /// Advance to the next region if there is any.
    pub fn next(&mut self) {
        if let Some(height) = self.backlog.next().or(self.last) {
            self.current.y = height;
            self.base.y = height;
        }
    }

    /// An iterator that returns pairs of `(current, base)` that are equivalent
    /// to what would be produced by calling [`next()`](Self::next) repeatedly
    /// until all regions are exhausted.
    pub fn iter(&self) -> impl Iterator<Item = (Size, Size)> + '_ {
        let first = std::iter::once((self.current, self.base));
        let backlog = self.backlog.as_slice().iter();
        let last = self.last.iter().cycle();
        first.chain(backlog.chain(last).map(|&height| {
            (
                Size::new(self.current.x, height),
                Size::new(self.base.x, height),
            )
        }))
    }
}
