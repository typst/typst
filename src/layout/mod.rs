//! Layouting.

mod fixed;
mod node;
mod pad;
mod par;
mod spacing;
mod stack;
mod text;

use crate::env::{ResourceId, SharedEnv};
use crate::geom::*;
use crate::shaping::Shaped;

pub use fixed::*;
pub use node::*;
pub use pad::*;
pub use par::*;
pub use spacing::*;
pub use stack::*;
pub use text::*;

/// Layout a tree into a collection of frames.
pub fn layout(tree: &Tree, env: SharedEnv) -> Vec<Frame> {
    tree.layout(&mut LayoutContext { env })
}

/// A tree of layout nodes.
#[derive(Debug, Clone, PartialEq)]
pub struct Tree {
    /// Runs of pages with the same properties.
    pub runs: Vec<NodePages>,
}

impl Tree {
    /// Layout the tree into a collection of frames.
    pub fn layout(&self, ctx: &mut LayoutContext) -> Vec<Frame> {
        self.runs.iter().flat_map(|run| run.layout(ctx)).collect()
    }
}

/// A run of pages that all have the same properties.
#[derive(Debug, Clone, PartialEq)]
pub struct NodePages {
    /// The size of each page.
    pub size: Size,
    /// The layout node that produces the actual pages (typically a
    /// [`NodeStack`]).
    pub child: Node,
}

impl NodePages {
    /// Layout the page run.
    pub fn layout(&self, ctx: &mut LayoutContext) -> Vec<Frame> {
        let areas = Areas::repeat(self.size);
        let layouted = self.child.layout(ctx, &areas);
        layouted.frames()
    }
}

/// Layout a node.
pub trait Layout {
    /// Layout the node into the given areas.
    fn layout(&self, ctx: &mut LayoutContext, areas: &Areas) -> Layouted;
}

/// The context for layouting.
#[derive(Debug, Clone)]
pub struct LayoutContext {
    /// The environment from which fonts are gathered.
    pub env: SharedEnv,
}

/// A collection of areas to layout into.
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
}

impl Areas {
    /// Create a new length-1 sequence of areas with just one `area`.
    pub fn once(size: Size) -> Self {
        Self {
            current: size,
            full: size,
            backlog: vec![],
            last: None,
        }
    }

    /// Create a new sequence of areas that repeats `area` indefinitely.
    pub fn repeat(size: Size) -> Self {
        Self {
            current: size,
            full: size,
            backlog: vec![],
            last: Some(size),
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
pub enum Expansion {
    /// Fit the content.
    Fit,
    /// Fill the available space.
    Fill,
}

impl Expansion {
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
pub enum Layouted {
    /// Spacing that should be added to the parent.
    Spacing(Length),
    /// A layout that should be added to and aligned in the parent.
    Frame(Frame, ChildAlign),
    /// Multiple layouts.
    Frames(Vec<Frame>, ChildAlign),
}

impl Layouted {
    /// Return all frames contained in this variant (zero, one or arbitrarily
    /// many).
    pub fn frames(self) -> Vec<Frame> {
        match self {
            Self::Spacing(_) => vec![],
            Self::Frame(frame, _) => vec![frame],
            Self::Frames(frames, _) => frames,
        }
    }
}

/// A finished layout with elements at fixed positions.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    /// The size of the frame.
    pub size: Size,
    /// The elements composing this layout.
    pub elements: Vec<(Point, Element)>,
}

impl Frame {
    /// Create a new, empty frame.
    pub fn new(size: Size) -> Self {
        Self { size, elements: vec![] }
    }

    /// Add an element at a position.
    pub fn push(&mut self, pos: Point, element: Element) {
        self.elements.push((pos, element));
    }

    /// Add all elements of another frame, placing them relative to the given
    /// position.
    pub fn push_frame(&mut self, pos: Point, subframe: Self) {
        for (subpos, element) in subframe.elements {
            self.push(pos + subpos, element);
        }
    }
}

/// The building block frames are composed of.
#[derive(Debug, Clone, PartialEq)]
pub enum Element {
    /// Shaped text.
    Text(Shaped),
    /// An image.
    Image(Image),
}

/// An image element.
#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    /// The image resource.
    pub res: ResourceId,
    /// The size of the image in the document.
    pub size: Size,
}
