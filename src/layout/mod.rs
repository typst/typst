//! Layouting.

mod background;
mod fixed;
mod frame;
mod grid;
mod incremental;
mod pad;
mod par;
mod shaping;
mod stack;

pub use background::*;
pub use fixed::*;
pub use frame::*;
pub use grid::*;
pub use incremental::*;
pub use pad::*;
pub use par::*;
pub use shaping::*;
pub use stack::*;

use std::any::Any;
use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};

use fxhash::FxHasher64;

use crate::cache::Cache;
use crate::geom::*;
use crate::loading::Loader;

/// Layout a tree into a collection of frames.
pub fn layout(loader: &mut dyn Loader, cache: &mut Cache, tree: &Tree) -> Vec<Frame> {
    tree.layout(&mut LayoutContext { loader, cache })
}

/// A tree of layout nodes.
#[derive(Debug, Clone, PartialEq)]
pub struct Tree {
    /// Runs of pages with the same properties.
    pub runs: Vec<PageRun>,
}

impl Tree {
    /// Layout the tree into a collection of frames.
    pub fn layout(&self, ctx: &mut LayoutContext) -> Vec<Frame> {
        self.runs.iter().flat_map(|run| run.layout(ctx)).collect()
    }
}

/// A run of pages that all have the same properties.
#[derive(Debug, Clone, PartialEq)]
pub struct PageRun {
    /// The size of each page.
    pub size: Size,
    /// The layout node that produces the actual pages (typically a
    /// [`StackNode`]).
    pub child: AnyNode,
}

impl PageRun {
    /// Layout the page run.
    pub fn layout(&self, ctx: &mut LayoutContext) -> Vec<Frame> {
        // When one of the lengths is infinite the page fits its content along
        // that axis.
        let Size { width, height } = self.size;
        let expand = Spec::new(width.is_finite(), height.is_finite());
        let regions = Regions::repeat(self.size, expand);
        self.child.layout(ctx, &regions).into_iter().map(|c| c.item).collect()
    }
}

/// A wrapper around a dynamic layouting node.
pub struct AnyNode {
    node: Box<dyn Bounds>,
    hash: u64,
}

impl AnyNode {
    /// Create a new instance from any node that satisifies the required bounds.
    pub fn new<T>(node: T) -> Self
    where
        T: Layout + Debug + Clone + PartialEq + Hash + 'static,
    {
        let mut state = FxHasher64::default();
        node.type_id().hash(&mut state);
        node.hash(&mut state);
        let hash = state.finish();

        Self { node: Box::new(node), hash }
    }
}

impl Layout for AnyNode {
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
    ) -> Vec<Constrained<Frame>> {
        ctx.cache
            .layout
            .frames
            .get(&self.hash)
            .and_then(|x| x.check(regions.clone()))
            .unwrap_or_else(|| {
                let frames = self.node.layout(ctx, regions);
                ctx.cache
                    .layout
                    .frames
                    .insert(self.hash, FramesEntry { frames: frames.clone() });
                frames
            })
    }
}

impl Clone for AnyNode {
    fn clone(&self) -> Self {
        Self {
            node: self.node.dyn_clone(),
            hash: self.hash,
        }
    }
}

impl PartialEq for AnyNode {
    fn eq(&self, other: &Self) -> bool {
        self.node.dyn_eq(other.node.as_ref())
    }
}

impl Hash for AnyNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.hash);
    }
}

impl Debug for AnyNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.node.fmt(f)
    }
}

trait Bounds: Layout + Debug + 'static {
    fn as_any(&self) -> &dyn Any;
    fn dyn_eq(&self, other: &dyn Bounds) -> bool;
    fn dyn_clone(&self) -> Box<dyn Bounds>;
}

impl<T> Bounds for T
where
    T: Layout + Debug + PartialEq + Clone + 'static,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn dyn_eq(&self, other: &dyn Bounds) -> bool {
        if let Some(other) = other.as_any().downcast_ref::<Self>() {
            self == other
        } else {
            false
        }
    }

    fn dyn_clone(&self) -> Box<dyn Bounds> {
        Box::new(self.clone())
    }
}

/// Layout a node.
pub trait Layout {
    /// Layout the node into the given regions.
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
    ) -> Vec<Constrained<Frame>>;
}

/// The context for layouting.
pub struct LayoutContext<'a> {
    /// The loader from which fonts are loaded.
    pub loader: &'a mut dyn Loader,
    /// A cache for loaded fonts and artifacts from past layouting.
    pub cache: &'a mut Cache,
}

/// A sequence of regions to layout into.
#[derive(Debug, Clone, PartialEq)]
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

    /// Advance to the next region if there is any.
    pub fn next(&mut self) -> bool {
        if let Some(size) = self.backlog.pop().or(self.last) {
            self.current = size;
            self.base = size;
            true
        } else {
            false
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
