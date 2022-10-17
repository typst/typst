//! Layouting infrastructure.

use std::any::Any;
use std::fmt::{self, Debug, Formatter, Write};
use std::hash::Hash;
use std::sync::Arc;

use comemo::{Prehashed, Tracked};

use super::{Barrier, NodeId, Resolve, StyleChain, StyleEntry};
use super::{Builder, Content, RawLength, Scratch};
use crate::diag::SourceResult;
use crate::frame::{Element, Frame};
use crate::geom::{Align, Geometry, Length, Paint, Point, Relative, Size, Spec, Stroke};
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
pub trait Layout: 'static {
    /// Layout this node into the given regions, producing frames.
    fn layout(
        &self,
        world: Tracked<dyn World>,
        regions: &Regions,
        styles: StyleChain,
    ) -> SourceResult<Vec<Frame>>;

    /// Convert to a packed node.
    fn pack(self) -> LayoutNode
    where
        Self: Debug + Hash + Sized + Sync + Send + 'static,
    {
        LayoutNode::new(self)
    }
}

/// A sequence of regions to layout into.
#[derive(Debug, Clone, Hash)]
pub struct Regions {
    /// The (remaining) size of the first region.
    pub first: Size,
    /// The base size for relative sizing.
    pub base: Size,
    /// The height of followup regions. The width is the same for all regions.
    pub backlog: Vec<Length>,
    /// The height of the final region that is repeated once the backlog is
    /// drained. The width is the same for all regions.
    pub last: Option<Length>,
    /// Whether nodes should expand to fill the regions instead of shrinking to
    /// fit the content.
    pub expand: Spec<bool>,
}

impl Regions {
    /// Create a new region sequence with exactly one region.
    pub fn one(size: Size, base: Size, expand: Spec<bool>) -> Self {
        Self {
            first: size,
            base,
            backlog: vec![],
            last: None,
            expand,
        }
    }

    /// Create a new sequence of same-size regions that repeats indefinitely.
    pub fn repeat(size: Size, base: Size, expand: Spec<bool>) -> Self {
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
        Length::zero().fits(self.first.y) && !self.in_last()
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

/// A type-erased layouting node with a precomputed hash.
#[derive(Clone, Hash)]
pub struct LayoutNode(Arc<Prehashed<dyn Bounds>>);

impl LayoutNode {
    /// Pack any layoutable node.
    pub fn new<T>(node: T) -> Self
    where
        T: Layout + Debug + Hash + Sync + Send + 'static,
    {
        Self(Arc::new(Prehashed::new(node)))
    }

    /// Check whether the contained node is a specific layout node.
    pub fn is<T: 'static>(&self) -> bool {
        (**self.0).as_any().is::<T>()
    }

    /// The id of this node.
    pub fn id(&self) -> NodeId {
        (**self.0).node_id()
    }

    /// Try to downcast to a specific layout node.
    pub fn downcast<T>(&self) -> Option<&T>
    where
        T: Layout + Debug + Hash + 'static,
    {
        (**self.0).as_any().downcast_ref()
    }

    /// Force a size for this node.
    pub fn sized(self, sizing: Spec<Option<Relative<RawLength>>>) -> Self {
        if sizing.any(Option::is_some) {
            SizedNode { sizing, child: self }.pack()
        } else {
            self
        }
    }

    /// Fill the frames resulting from a node.
    pub fn filled(self, fill: Paint) -> Self {
        FillNode { fill, child: self }.pack()
    }

    /// Stroke the frames resulting from a node.
    pub fn stroked(self, stroke: Stroke) -> Self {
        StrokeNode { stroke, child: self }.pack()
    }
}

impl Layout for LayoutNode {
    #[comemo::memoize]
    fn layout(
        &self,
        world: Tracked<dyn World>,
        regions: &Regions,
        styles: StyleChain,
    ) -> SourceResult<Vec<Frame>> {
        let barrier = StyleEntry::Barrier(Barrier::new(self.id()));
        let styles = barrier.chain(&styles);

        let mut frames = self.0.layout(world, regions, styles)?;
        if let Some(role) = styles.role() {
            for frame in &mut frames {
                frame.apply_role(role);
            }
        }

        Ok(frames)
    }

    fn pack(self) -> LayoutNode {
        self
    }
}

impl Default for LayoutNode {
    fn default() -> Self {
        EmptyNode.pack()
    }
}

impl Debug for LayoutNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("Layout(")?;
        self.0.fmt(f)?;
        f.write_char(')')
    }
}

impl PartialEq for LayoutNode {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

trait Bounds: Layout + Debug + Sync + Send + 'static {
    fn as_any(&self) -> &dyn Any;
    fn node_id(&self) -> NodeId;
}

impl<T> Bounds for T
where
    T: Layout + Debug + Hash + Sync + Send + 'static,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn node_id(&self) -> NodeId {
        NodeId::of::<Self>()
    }
}

/// A layout node that produces an empty frame.
///
/// The packed version of this is returned by [`PackedNode::default`].
#[derive(Debug, Hash)]
struct EmptyNode;

impl Layout for EmptyNode {
    fn layout(
        &self,
        _: Tracked<dyn World>,
        regions: &Regions,
        _: StyleChain,
    ) -> SourceResult<Vec<Frame>> {
        Ok(vec![Frame::new(
            regions.expand.select(regions.first, Size::zero()),
        )])
    }
}

/// Fix the size of a node.
#[derive(Debug, Hash)]
struct SizedNode {
    /// How to size the node horizontally and vertically.
    sizing: Spec<Option<Relative<RawLength>>>,
    /// The node to be sized.
    child: LayoutNode,
}

impl Layout for SizedNode {
    fn layout(
        &self,
        world: Tracked<dyn World>,
        regions: &Regions,
        styles: StyleChain,
    ) -> SourceResult<Vec<Frame>> {
        // The "pod" is the region into which the child will be layouted.
        let pod = {
            // Resolve the sizing to a concrete size.
            let size = self
                .sizing
                .resolve(styles)
                .zip(regions.base)
                .map(|(s, b)| s.map(|v| v.relative_to(b)))
                .unwrap_or(regions.first);

            // Select the appropriate base and expansion for the child depending
            // on whether it is automatically or relatively sized.
            let is_auto = self.sizing.map_is_none();
            let base = is_auto.select(regions.base, size);
            let expand = regions.expand | !is_auto;

            Regions::one(size, base, expand)
        };

        // Layout the child.
        let mut frames = self.child.layout(world, &pod, styles)?;

        // Ensure frame size matches regions size if expansion is on.
        let frame = &mut frames[0];
        let target = regions.expand.select(regions.first, frame.size());
        frame.resize(target, Align::LEFT_TOP);

        Ok(frames)
    }
}

/// Fill the frames resulting from a node.
#[derive(Debug, Hash)]
struct FillNode {
    /// How to fill the frames resulting from the `child`.
    fill: Paint,
    /// The node whose frames should be filled.
    child: LayoutNode,
}

impl Layout for FillNode {
    fn layout(
        &self,
        world: Tracked<dyn World>,
        regions: &Regions,
        styles: StyleChain,
    ) -> SourceResult<Vec<Frame>> {
        let mut frames = self.child.layout(world, regions, styles)?;
        for frame in &mut frames {
            let shape = Geometry::Rect(frame.size()).filled(self.fill);
            frame.prepend(Point::zero(), Element::Shape(shape));
        }
        Ok(frames)
    }
}

/// Stroke the frames resulting from a node.
#[derive(Debug, Hash)]
struct StrokeNode {
    /// How to stroke the frames resulting from the `child`.
    stroke: Stroke,
    /// The node whose frames should be stroked.
    child: LayoutNode,
}

impl Layout for StrokeNode {
    fn layout(
        &self,
        world: Tracked<dyn World>,
        regions: &Regions,
        styles: StyleChain,
    ) -> SourceResult<Vec<Frame>> {
        let mut frames = self.child.layout(world, regions, styles)?;
        for frame in &mut frames {
            let shape = Geometry::Rect(frame.size()).stroked(self.stroke);
            frame.prepend(Point::zero(), Element::Shape(shape));
        }
        Ok(frames)
    }
}
