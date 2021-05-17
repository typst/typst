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
        let areas = Areas::repeat(self.size, fixed);
        self.child.layout(ctx, &areas)
    }
}

/// A wrapper around a dynamic layouting node.
pub struct AnyNode(Box<dyn Bounds>);

impl AnyNode {
    /// Create a new instance from any node that satisifies the required bounds.
    pub fn new<T>(any: T) -> Self
    where
        T: Layout + Debug + Clone + PartialEq + 'static,
    {
        Self(Box::new(any))
    }
}

impl Layout for AnyNode {
    fn layout(&self, ctx: &mut LayoutContext, areas: &Areas) -> Vec<Frame> {
        self.0.layout(ctx, areas)
    }
}

impl Clone for AnyNode {
    fn clone(&self) -> Self {
        Self(self.0.dyn_clone())
    }
}

impl PartialEq for AnyNode {
    fn eq(&self, other: &Self) -> bool {
        self.0.dyn_eq(other.0.as_ref())
    }
}

impl Debug for AnyNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.0.fmt(f)
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
    /// Layout the node into the given areas.
    fn layout(&self, ctx: &mut LayoutContext, areas: &Areas) -> Vec<Frame>;
}

/// The context for layouting.
pub struct LayoutContext<'a> {
    /// The environment from which fonts are gathered.
    pub env: &'a mut Env,
}

/// A sequence of areas to layout into.
#[derive(Debug, Clone, PartialEq)]
pub struct Areas {
    /// The remaining size of the current area.
    pub current: Size,
    /// The base size for relative sizing.
    pub base: Size,
    /// A stack of followup areas.
    ///
    /// Note that this is a stack and not a queue! The size of the next area is
    /// `backlog.last()`.
    pub backlog: Vec<Size>,
    /// The final area that is repeated once the backlog is drained.
    pub last: Option<Size>,
    /// Whether layouting into these areas should produce frames of the exact
    /// size of `current` instead of shrinking to fit the content.
    ///
    /// This property is only handled by nodes that have the ability to control
    /// their own size.
    pub fixed: Spec<bool>,
}

impl Areas {
    /// Create a new area sequence of length one.
    pub fn once(size: Size, fixed: Spec<bool>) -> Self {
        Self {
            current: size,
            base: size,
            backlog: vec![],
            last: None,
            fixed,
        }
    }

    /// Create a new sequence of same-size areas that repeats indefinitely.
    pub fn repeat(size: Size, fixed: Spec<bool>) -> Self {
        Self {
            current: size,
            base: size,
            backlog: vec![],
            last: Some(size),
            fixed,
        }
    }

    /// Map the size of all areas.
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

    /// Whether `current` is a fully sized (untouched) copy of the last area.
    ///
    /// If this is true, calling `next()` will have no effect.
    pub fn in_full_last(&self) -> bool {
        self.backlog.is_empty()
            && self.last.map_or(true, |size| {
                self.current.is_nan() || size.is_nan() || self.current == size
            })
    }

    /// Advance to the next area if there is any.
    pub fn next(&mut self) {
        if let Some(size) = self.backlog.pop().or(self.last) {
            self.current = size;
            self.base = size;
        }
    }

    /// Shrink `current` to ensure that the aspect ratio can be satisfied.
    pub fn apply_aspect_ratio(&mut self, aspect: f64) {
        let width = self.current.width.min(aspect * self.current.height);
        let height = width / aspect;
        self.current = Size::new(width, height);
    }
}
