//! Layouting.

mod background;
mod fixed;
mod frame;
mod node;
mod pad;
mod par;
mod shaping;
mod spacing;
mod stack;
mod text;

pub use background::*;
pub use fixed::*;
pub use frame::*;
pub use node::*;
pub use pad::*;
pub use par::*;
pub use shaping::*;
pub use spacing::*;
pub use stack::*;
pub use text::*;

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
    pub child: Node,
}

impl PageRun {
    /// Layout the page run.
    pub fn layout(&self, ctx: &mut LayoutContext) -> Vec<Frame> {
        let areas = Areas::repeat(self.size, Spec::uniform(Expand::Fill));
        let layouted = self.child.layout(ctx, &areas);
        layouted.into_frames()
    }
}

/// Layout a node.
pub trait Layout {
    /// Layout the node into the given areas.
    fn layout(&self, ctx: &mut LayoutContext, areas: &Areas) -> Fragment;
}

/// The context for layouting.
#[derive(Debug)]
pub struct LayoutContext<'a> {
    /// The environment from which fonts are gathered.
    pub env: &'a mut Env,
}

/// A sequence of areas to layout into.
#[derive(Debug, Clone, PartialEq)]
pub struct Areas {
    /// The remaining size of the current area.
    pub current: Size,
    /// The full size the current area once had (used for relative sizing).
    pub full: Size,
    /// A stack of followup areas (the next area is the last element).
    pub backlog: Vec<Size>,
    /// The final area that is repeated when the backlog is empty.
    pub last: Option<Size>,
    /// Whether the frames resulting from layouting into this areas should be
    /// shrunk to fit their content or expanded to fill the area.
    pub expand: Spec<Expand>,
}

impl Areas {
    /// Create a new length-1 sequence of areas with just one `area`.
    pub fn once(size: Size, expand: Spec<Expand>) -> Self {
        Self {
            current: size,
            full: size,
            backlog: vec![],
            last: None,
            expand,
        }
    }

    /// Create a new sequence of areas that repeats `area` indefinitely.
    pub fn repeat(size: Size, expand: Spec<Expand>) -> Self {
        Self {
            current: size,
            full: size,
            backlog: vec![],
            last: Some(size),
            expand,
        }
    }

    /// Advance to the next area if there is any.
    pub fn next(&mut self) {
        if let Some(size) = self.backlog.pop().or(self.last) {
            self.current = size;
            self.full = size;
        }
    }

    /// Whether `current` is a fully sized (untouched) copy of the last area.
    ///
    /// If this is false calling `next()` will have no effect.
    pub fn in_full_last(&self) -> bool {
        self.backlog.is_empty()
            && self.last.map_or(true, |size| {
                self.current.is_nan() || size.is_nan() || self.current == size
            })
    }
}

/// Whether to expand or shrink a node along an axis.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Expand {
    /// Fit the content.
    Fit,
    /// Fill the available space.
    Fill,
}

impl Expand {
    /// Resolve the expansion to either the `fit` or `fill` length.
    ///
    /// Prefers `fit` if `fill` is infinite.
    pub fn resolve(self, fit: Length, fill: Length) -> Length {
        match self {
            Self::Fill if fill.is_finite() => fill,
            _ => fit,
        }
    }
}

/// The result of layouting a node.
#[derive(Debug, Clone, PartialEq)]
pub enum Fragment {
    /// Spacing that should be added to the parent.
    Spacing(Length),
    /// A layout that should be added to and aligned in the parent.
    Frame(Frame, LayoutAligns),
    /// Multiple layouts.
    Frames(Vec<Frame>, LayoutAligns),
}

impl Fragment {
    /// Return a reference to all frames contained in this variant (zero, one or
    /// arbitrarily many).
    pub fn frames(&self) -> &[Frame] {
        match self {
            Self::Spacing(_) => &[],
            Self::Frame(frame, _) => std::slice::from_ref(frame),
            Self::Frames(frames, _) => frames,
        }
    }

    /// Return a mutable reference to all frames contained in this variant.
    pub fn frames_mut(&mut self) -> &mut [Frame] {
        match self {
            Self::Spacing(_) => &mut [],
            Self::Frame(frame, _) => std::slice::from_mut(frame),
            Self::Frames(frames, _) => frames,
        }
    }

    /// Return all frames contained in this varian.
    pub fn into_frames(self) -> Vec<Frame> {
        match self {
            Self::Spacing(_) => vec![],
            Self::Frame(frame, _) => vec![frame],
            Self::Frames(frames, _) => frames,
        }
    }
}
