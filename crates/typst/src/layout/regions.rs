use std::fmt::{self, Debug, Formatter};

use crate::layout::{Abs, Axes, Size};

/// A single region to layout into.
#[derive(Debug, Copy, Clone, Hash)]
pub struct Region {
    /// The size of the region.
    pub size: Size,
    /// Whether elements should expand to fill the regions instead of shrinking
    /// to fit the content.
    pub expand: Axes<bool>,
}

impl Region {
    /// Create a new region.
    pub fn new(size: Size, expand: Axes<bool>) -> Self {
        Self { size, expand }
    }
}

impl From<Region> for Regions<'_> {
    fn from(region: Region) -> Self {
        Regions {
            size: region.size,
            expand: region.expand,
            full: region.size.y,
            backlog: &[],
            last: None,
        }
    }
}

/// A sequence of regions to layout into.
///
/// A *region* is a contiguous rectangular space in which elements
/// can be laid out. All regions within a `Regions` object have the
/// same width, namely `self.size.x`. This means that it is not
/// currently possible to, for instance, have content wrap to the
/// side of a floating element.
#[derive(Copy, Clone, Hash)]
pub struct Regions<'a> {
    /// The remaining size of the first region.
    pub size: Size,
    /// Whether elements should expand to fill the regions instead of shrinking
    /// to fit the content.
    pub expand: Axes<bool>,
    /// The full height of the region for relative sizing.
    pub full: Abs,
    /// The height of followup regions. The width is the same for all regions.
    pub backlog: &'a [Abs],
    /// The height of the final region that is repeated once the backlog is
    /// drained. The width is the same for all regions.
    pub last: Option<Abs>,
}

impl Regions<'_> {
    /// Create a new sequence of same-size regions that repeats indefinitely.
    pub fn repeat(size: Size, expand: Axes<bool>) -> Self {
        Self {
            size,
            full: size.y,
            backlog: &[],
            last: Some(size.y),
            expand,
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
        }
    }

    /// Whether the first region is full and a region break is called for.
    pub fn is_full(&self) -> bool {
        Abs::zero().fits(self.size.y) && self.may_progress()
    }

    /// Whether a region break is permitted.
    pub fn may_break(&self) -> bool {
        !self.backlog.is_empty() || self.last.is_some()
    }

    /// Whether calling `next()` may improve a situation where there is a lack
    /// of space.
    pub fn may_progress(&self) -> bool {
        !self.backlog.is_empty() || self.last.is_some_and(|height| self.size.y != height)
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
