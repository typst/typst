//! Layouting.

mod background;
mod fixed;
mod frame;
mod pad;
mod par;
mod shaping;
mod stack;

pub use background::*;
pub use fixed::*;
pub use frame::*;
pub use pad::*;
pub use par::*;
pub use shaping::*;
pub use stack::*;

use std::any::Any;
use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};

use decorum::NotNan;
use fxhash::FxHasher64;

use crate::env::Env;
use crate::geom::*;

/// Layout a tree into a collection of frames.
pub fn layout(env: &mut Env, tree: &Tree) -> Vec<Frame> {
    tree.layout(&mut LayoutContext { env })
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
        let fixed = Spec::new(width.is_finite(), height.is_finite());
        let regions = Regions::repeat(self.size, fixed);
        self.child.layout(ctx, &regions)
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
        let hash = {
            let mut state = FxHasher64::default();
            node.hash(&mut state);
            state.finish()
        };

        Self { node: Box::new(node), hash }
    }

    /// The cached hash for the boxed node.
    pub fn hash(&self) -> u64 {
        self.hash
    }
}

impl Layout for AnyNode {
    fn layout(&self, ctx: &mut LayoutContext, regions: &Regions) -> Vec<Frame> {
        self.node.layout(ctx, regions)
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
    fn layout(&self, ctx: &mut LayoutContext, regions: &Regions) -> Vec<Frame>;
}

/// The context for layouting.
pub struct LayoutContext<'a> {
    /// The environment from which fonts are gathered.
    pub env: &'a mut Env,
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
    /// Whether layouting into these regions should produce frames of the exact
    /// size of `current` instead of shrinking to fit the content.
    ///
    /// This property is only handled by nodes that have the ability to control
    /// their own size.
    pub fixed: Spec<bool>,
}

impl Regions {
    /// Create a new region sequence with exactly one region.
    pub fn one(size: Size, fixed: Spec<bool>) -> Self {
        Self {
            current: size,
            base: size,
            backlog: vec![],
            last: None,
            fixed,
        }
    }

    /// Create a new sequence of same-size regions that repeats indefinitely.
    pub fn repeat(size: Size, fixed: Spec<bool>) -> Self {
        Self {
            current: size,
            base: size,
            backlog: vec![],
            last: Some(size),
            fixed,
        }
    }

    /// Map the size of all regions.
    pub fn map<F>(&self, mut f: F) -> Self
    where
        F: FnMut(Size) -> Size,
    {
        Self {
            current: f(self.current),
            base: f(self.base),
            backlog: self.backlog.iter().copied().map(|s| f(s)).collect(),
            last: self.last.map(f),
            fixed: self.fixed,
        }
    }

    /// Whether `current` is a fully sized (untouched) copy of the last region.
    ///
    /// If this is true, calling `next()` will have no effect.
    pub fn in_full_last(&self) -> bool {
        self.backlog.is_empty() && self.last.map_or(true, |size| self.current == size)
    }

    /// Advance to the next region if there is any.
    pub fn next(&mut self) {
        if let Some(size) = self.backlog.pop().or(self.last) {
            self.current = size;
            self.base = size;
        }
    }

    /// Shrink `current` to ensure that the aspect ratio can be satisfied.
    pub fn apply_aspect_ratio(&mut self, aspect: NotNan<f64>) {
        let width = self.current.width.min(aspect.into_inner() * self.current.height);
        let height = width / aspect.into_inner();
        self.current = Size::new(width, height);
    }
}
