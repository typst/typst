use std::fmt::{self, Debug, Formatter};

use crate::layout::{Abs, Axes, Size};

/// A sequence of regions to layout into.
#[derive(Copy, Clone, Hash)]
pub struct Regions<'a> {
    /// The remaining size of the first region.
    pub size: Size,
    /// The full height of the region for relative sizing.
    pub full: Abs,
    /// The height of followup regions. The width is the same for all regions.
    pub backlog: &'a [Abs],
    /// The height of the final region that is repeated once the backlog is
    /// drained. The width is the same for all regions.
    pub last: Option<Abs>,
    /// Whether elements should expand to fill the regions instead of shrinking
    /// to fit the content.
    pub expand: Axes<bool>,
    /// Whether these are the root regions or direct descendants.
    ///
    /// True for the padded page regions and columns directly in the page,
    /// false otherwise.
    pub root: bool,
}

impl Regions<'_> {
    /// Create a new region sequence with exactly one region.
    pub fn one(size: Size, expand: Axes<bool>) -> Self {
        Self {
            size,
            full: size.y,
            backlog: &[],
            last: None,
            expand,
            root: false,
        }
    }

    /// Create a new sequence of same-size regions that repeats indefinitely.
    pub fn repeat(size: Size, expand: Axes<bool>) -> Self {
        Self {
            size,
            full: size.y,
            backlog: &[],
            last: Some(size.y),
            expand,
            root: false,
        }
    }

    /// The base size, which doesn't take into account that the regions is
    /// already partially used up.
    ///
    /// This is also used for relative sizing.
    pub fn base(&self) -> Size {
        Size::new(self.size.x, self.full)
    }

    /// Create new regions where all sizes are mapped with `f`.
    ///
    /// Note that since all regions must have the same width, the width returned
    /// by `f` is ignored for the backlog and the final region.
    pub fn map<'v, F>(&self, backlog: &'v mut Vec<Abs>, mut f: F) -> Regions<'v>
    where
        F: FnMut(Size) -> Size,
    {
        let x = self.size.x;
        backlog.clear();
        backlog.extend(self.backlog.iter().map(|&y| f(Size::new(x, y)).y));
        Regions {
            size: f(self.size),
            full: f(Size::new(x, self.full)).y,
            backlog,
            last: self.last.map(|y| f(Size::new(x, y)).y),
            expand: self.expand,
            root: false,
        }
    }

    /// Whether the first region is full and a region break is called for.
    pub fn is_full(&self) -> bool {
        Abs::zero().fits(self.size.y) && !self.in_last()
    }

    /// Whether the first region is the last usable region.
    ///
    /// If this is true, calling `next()` will have no effect.
    pub fn in_last(&self) -> bool {
        self.backlog.is_empty() && self.last.map_or(true, |height| self.size.y == height)
    }

    /// The same regions, but with different `root` configuration.
    pub fn with_root(self, root: bool) -> Self {
        Self { root, ..self }
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
            self.size.y = height;
            self.full = height;
        }
    }

    /// An iterator that returns the sizes of the first and all following
    /// regions, equivalently to what would be produced by calling
    /// [`next()`](Self::next) repeatedly until all regions are exhausted.
    /// This iterator may be infinite.
    pub fn iter(&self) -> impl Iterator<Item = Size> + '_ {
        let first = std::iter::once(self.size);
        let backlog = self.backlog.iter();
        let last = self.last.iter().cycle();
        first.chain(backlog.chain(last).map(|&h| Size::new(self.size.x, h)))
    }
}

impl Debug for Regions<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("Regions ")?;
        let mut list = f.debug_list();
        let mut prev = self.size.y;
        list.entry(&self.size);
        for &height in self.backlog {
            list.entry(&Size::new(self.size.x, height));
            prev = height;
        }
        if let Some(last) = self.last {
            if last != prev {
                list.entry(&Size::new(self.size.x, last));
            }
            list.entry(&(..));
        }
        list.finish()
    }
}
