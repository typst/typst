//! Layouting.

mod background;
mod fixed;
mod frame;
mod grid;
mod image;
mod incremental;
mod pad;
mod par;
mod shaping;
mod stack;
mod tree;

pub use self::image::*;
pub use background::*;
pub use fixed::*;
pub use frame::*;
pub use grid::*;
pub use incremental::*;
pub use pad::*;
pub use par::*;
pub use shaping::*;
pub use stack::*;
pub use tree::*;

use std::hash::Hash;
#[cfg(feature = "layout-cache")]
use std::hash::Hasher;
use std::rc::Rc;

use crate::font::FontStore;
use crate::geom::*;
use crate::image::ImageStore;
use crate::util::OptionExt;
use crate::Context;

/// Layout a tree into a collection of frames.
pub fn layout(ctx: &mut Context, tree: &LayoutTree) -> Vec<Rc<Frame>> {
    let mut ctx = LayoutContext::new(ctx);
    tree.layout(&mut ctx)
}

/// Layout a node.
pub trait Layout {
    /// Layout the node into the given regions.
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
    ) -> Vec<Constrained<Rc<Frame>>>;
}

/// The context for layouting.
pub struct LayoutContext<'a> {
    /// Stores parsed font faces.
    pub fonts: &'a mut FontStore,
    /// Stores decoded images.
    pub images: &'a mut ImageStore,
    /// Caches layouting artifacts.
    #[cfg(feature = "layout-cache")]
    pub layouts: &'a mut LayoutCache,
    /// How deeply nested the current layout tree position is.
    #[cfg(feature = "layout-cache")]
    pub level: usize,
}

impl<'a> LayoutContext<'a> {
    /// Create a new layout context.
    pub fn new(ctx: &'a mut Context) -> Self {
        Self {
            fonts: &mut ctx.fonts,
            images: &mut ctx.images,
            #[cfg(feature = "layout-cache")]
            layouts: &mut ctx.layouts,
            #[cfg(feature = "layout-cache")]
            level: 0,
        }
    }
}

/// A sequence of regions to layout into.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Regions {
    /// The remaining size of the current region.
    pub current: Size,
    /// The base size for relative sizing.
    pub base: Size,
    /// A stack of followup regions.
    ///
    /// Note that this is a stack and not a queue! The size of the next region is
    /// `backlog.last()`.
    pub backlog: Vec<Size>,
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
    pub fn one(size: Size, expand: Spec<bool>) -> Self {
        Self {
            current: size,
            base: size,
            backlog: vec![],
            last: None,
            expand,
        }
    }

    /// Create a new sequence of same-size regions that repeats indefinitely.
    pub fn repeat(size: Size, expand: Spec<bool>) -> Self {
        Self {
            current: size,
            base: size,
            backlog: vec![],
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
        self.backlog.is_empty() && self.last.map_or(true, |size| self.current == size)
    }

    /// An iterator that returns pairs of `(current, base)` that are equivalent
    /// to what would be produced by calling [`next()`](Self::next) repeatedly
    /// until all regions are exhausted.
    pub fn iter(&self) -> impl Iterator<Item = (Size, Size)> + '_ {
        let first = std::iter::once((self.current, self.base));
        let backlog = self.backlog.iter().rev();
        let last = self.last.iter().cycle();
        first.chain(backlog.chain(last).map(|&s| (s, s)))
    }

    /// Advance to the next region if there is any.
    pub fn next(&mut self) {
        if let Some(size) = self.backlog.pop().or(self.last) {
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
        self.backlog.iter_mut().for_each(f);
    }
}
