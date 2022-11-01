//! Layouting infrastructure.

use std::hash::Hash;

use comemo::Tracked;

use super::{Builder, Capability, Content, Scratch, StyleChain};
use crate::diag::SourceResult;
use crate::frame::Frame;
use crate::geom::{Abs, Axes, Size};
use crate::World;

/// Layout content into a collection of pages.
#[comemo::memoize]
pub fn layout(world: Tracked<dyn World>, content: &Content) -> SourceResult<Vec<Frame>> {
    let styles = StyleChain::with_root(&world.config().styles);
    let scratch = Scratch::default();

    let mut builder = Builder::new(world, &scratch, true);
    builder.accept(content, styles)?;

    let (doc, shared) = builder.into_doc(styles)?;
    doc.layout(world, shared)
}

/// A node that can be layouted into a sequence of regions.
///
/// Layouting returns one frame per used region.
pub trait Layout: 'static + Sync + Send {
    /// Layout this node into the given regions, producing frames.
    fn layout(
        &self,
        world: Tracked<dyn World>,
        regions: &Regions,
        styles: StyleChain,
    ) -> SourceResult<Vec<Frame>>;

    /// Whether this is an inline-level or block-level node.
    fn level(&self) -> Level;
}

impl Capability for dyn Layout {}

/// At which level a node operates.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Level {
    Inline,
    Block,
}

/// A sequence of regions to layout into.
#[derive(Debug, Clone, Hash)]
pub struct Regions {
    /// The (remaining) size of the first region.
    pub first: Size,
    /// The base size for relative sizing.
    pub base: Size,
    /// The height of followup regions. The width is the same for all regions.
    pub backlog: Vec<Abs>,
    /// The height of the final region that is repeated once the backlog is
    /// drained. The width is the same for all regions.
    pub last: Option<Abs>,
    /// Whether nodes should expand to fill the regions instead of shrinking to
    /// fit the content.
    pub expand: Axes<bool>,
}

impl Regions {
    /// Create a new region sequence with exactly one region.
    pub fn one(size: Size, base: Size, expand: Axes<bool>) -> Self {
        Self {
            first: size,
            base,
            backlog: vec![],
            last: None,
            expand,
        }
    }

    /// Create a new sequence of same-size regions that repeats indefinitely.
    pub fn repeat(size: Size, base: Size, expand: Axes<bool>) -> Self {
        Self {
            first: size,
            base,
            backlog: vec![],
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
        let x = self.first.x;
        Self {
            first: f(self.first),
            base: f(self.base),
            backlog: self.backlog.iter().map(|&y| f(Size::new(x, y)).y).collect(),
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
        self.backlog.len() == 0 && self.last.map_or(true, |height| self.first.y == height)
    }

    /// Advance to the next region if there is any.
    pub fn next(&mut self) {
        if let Some(height) = (!self.backlog.is_empty())
            .then(|| self.backlog.remove(0))
            .or(self.last)
        {
            self.first.y = height;
            self.base.y = height;
        }
    }

    /// An iterator that returns the sizes of the first and all following
    /// regions, equivalently to what would be produced by calling
    /// [`next()`](Self::next) repeatedly until all regions are exhausted.
    /// This iterater may be infinite.
    pub fn iter(&self) -> impl Iterator<Item = Size> + '_ {
        let first = std::iter::once(self.first);
        let backlog = self.backlog.iter();
        let last = self.last.iter().cycle();
        first.chain(backlog.chain(last).map(|&h| Size::new(self.first.x, h)))
    }
}
