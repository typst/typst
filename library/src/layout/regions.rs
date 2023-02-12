use std::fmt::{self, Debug, Formatter};

use typst::geom::{Abs, Axes, Size};

/// A sequence of regions to layout into.
#[derive(Copy, Clone, Hash)]
pub struct Regions<'a> {
    /// The (remaining) size of the first region.
    pub first: Size,
    /// The base size for relative sizing.
    pub base: Size,
    /// The height of followup regions. The width is the same for all regions.
    pub backlog: &'a [Abs],
    /// The height of the final region that is repeated once the backlog is
    /// drained. The width is the same for all regions.
    pub last: Option<Abs>,
    /// Whether nodes should expand to fill the regions instead of shrinking to
    /// fit the content.
    pub expand: Axes<bool>,
}

impl<'a> Regions<'a> {
    /// Create a new region sequence with exactly one region.
    pub fn one(size: Size, base: Size, expand: Axes<bool>) -> Self {
        Self {
            first: size,
            base,
            backlog: &[],
            last: None,
            expand,
        }
    }

    /// Create a new sequence of same-size regions that repeats indefinitely.
    pub fn repeat(size: Size, base: Size, expand: Axes<bool>) -> Self {
        Self {
            first: size,
            base,
            backlog: &[],
            last: Some(size.y),
            expand,
        }
    }

    /// Create new regions where all sizes are mapped with `f`.
    ///
    /// Note that since all regions must have the same width, the width returned
    /// by `f` is ignored for the backlog and the final region.
    pub fn map<'v, F>(&self, backlog: &'v mut Vec<Abs>, mut f: F) -> Regions<'v>
    where
        F: FnMut(Size) -> Size,
    {
        let x = self.first.x;
        backlog.clear();
        backlog.extend(self.backlog.iter().map(|&y| f(Size::new(x, y)).y));
        Regions {
            first: f(self.first),
            base: f(self.base),
            backlog,
            last: self.last.map(|y| f(Size::new(x, y)).y),
            expand: self.expand,
        }
    }

    /// Whether the first region is full and a region break is called for.
    pub fn is_full(&self) -> bool {
        Abs::zero().fits(self.first.y) && !self.in_last()
    }

    /// Whether the first region is the last usable region.
    ///
    /// If this is true, calling `next()` will have no effect.
    pub fn in_last(&self) -> bool {
        self.backlog.is_empty() && self.last.map_or(true, |height| self.first.y == height)
    }

    /// Advance to the next region if there is any.
    pub fn next(&mut self) {
        if let Some(height) = self
            .backlog
            .split_first()
            .map(|(first, tail)| {
                self.backlog = tail;
                *first
            })
            .or(self.last)
        {
            self.first.y = height;
            self.base.y = height;
        }
    }

    /// An iterator that returns the sizes of the first and all following
    /// regions, equivalently to what would be produced by calling
    /// [`next()`](Self::next) repeatedly until all regions are exhausted.
    /// This iterator may be infinite.
    pub fn iter(&self) -> impl Iterator<Item = Size> + '_ {
        let first = std::iter::once(self.first);
        let backlog = self.backlog.iter();
        let last = self.last.iter().cycle();
        first.chain(backlog.chain(last).map(|&h| Size::new(self.first.x, h)))
    }
}

impl Debug for Regions<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("Regions ")?;
        let mut list = f.debug_list();
        let mut prev = self.first.y;
        list.entry(&self.first);
        for &height in self.backlog {
            list.entry(&Size::new(self.first.x, height));
            prev = height;
        }
        if let Some(last) = self.last {
            if last != prev {
                list.entry(&Size::new(self.first.x, last));
            }
            list.entry(&(..));
        }
        list.finish()
    }
}
