//! Layouting infrastructure.

use std::any::{Any, TypeId};
use std::fmt::{self, Debug, Formatter};
use std::hash::Hash;
use std::sync::Arc;

use crate::diag::TypResult;
use crate::eval::StyleChain;
use crate::frame::{Element, Frame, Geometry, Shape, Stroke};
use crate::geom::{Align, Length, Linear, Paint, Point, Sides, Size, Spec, Transform};
use crate::library::graphics::MoveNode;
use crate::library::layout::{AlignNode, PadNode};
use crate::util::Prehashed;
use crate::Context;

/// A node that can be layouted into a sequence of regions.
///
/// Layout return one frame per used region alongside constraints that define
/// whether the result is reusable in other regions.
pub trait Layout {
    /// Layout the node into the given regions, producing constrained frames.
    fn layout(
        &self,
        ctx: &mut Context,
        regions: &Regions,
        styles: StyleChain,
    ) -> TypResult<Vec<Arc<Frame>>>;

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
        self.0.as_any().is::<T>()
    }

    /// The type id of this node.
    pub fn id(&self) -> TypeId {
        self.0.as_any().type_id()
    }

    /// Try to downcast to a specific layout node.
    pub fn downcast<T>(&self) -> Option<&T>
    where
        T: Layout + Debug + Hash + 'static,
    {
        self.0.as_any().downcast_ref()
    }

    /// Force a size for this node.
    pub fn sized(self, sizing: Spec<Option<Linear>>) -> Self {
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

    /// Set alignments for this node.
    pub fn aligned(self, aligns: Spec<Option<Align>>) -> Self {
        if aligns.any(Option::is_some) {
            AlignNode { aligns, child: self }.pack()
        } else {
            self
        }
    }

    /// Pad this node at the sides.
    pub fn padded(self, padding: Sides<Linear>) -> Self {
        if !padding.left.is_zero()
            || !padding.top.is_zero()
            || !padding.right.is_zero()
            || !padding.bottom.is_zero()
        {
            PadNode { padding, child: self }.pack()
        } else {
            self
        }
    }

    /// Transform this node's contents without affecting layout.
    pub fn moved(self, offset: Point) -> Self {
        if !offset.is_zero() {
            MoveNode {
                transform: Transform::translation(offset.x, offset.y),
                child: self,
            }
            .pack()
        } else {
            self
        }
    }
}

impl Layout for LayoutNode {
    fn layout(
        &self,
        ctx: &mut Context,
        regions: &Regions,
        styles: StyleChain,
    ) -> TypResult<Vec<Arc<Frame>>> {
        ctx.query((self, regions, styles), |ctx, (node, regions, styles)| {
            node.0.layout(ctx, regions, styles.barred(node.id()))
        })
        .clone()
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
        self.0.fmt(f)
    }
}

impl PartialEq for LayoutNode {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

trait Bounds: Layout + Debug + Sync + Send + 'static {
    fn as_any(&self) -> &dyn Any;
}

impl<T> Bounds for T
where
    T: Layout + Debug + Hash + Sync + Send + 'static,
{
    fn as_any(&self) -> &dyn Any {
        self
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
        _: &mut Context,
        regions: &Regions,
        _: StyleChain,
    ) -> TypResult<Vec<Arc<Frame>>> {
        Ok(vec![Arc::new(Frame::new(
            regions.expand.select(regions.first, Size::zero()),
        ))])
    }
}

/// Fix the size of a node.
#[derive(Debug, Hash)]
struct SizedNode {
    /// How to size the node horizontally and vertically.
    sizing: Spec<Option<Linear>>,
    /// The node to be sized.
    child: LayoutNode,
}

impl Layout for SizedNode {
    fn layout(
        &self,
        ctx: &mut Context,
        regions: &Regions,
        styles: StyleChain,
    ) -> TypResult<Vec<Arc<Frame>>> {
        // The "pod" is the region into which the child will be layouted.
        let pod = {
            // Resolve the sizing to a concrete size.
            let size = self
                .sizing
                .zip(regions.base)
                .map(|(s, b)| s.map(|v| v.resolve(b)))
                .unwrap_or(regions.first);

            // Select the appropriate base and expansion for the child depending
            // on whether it is automatically or linearly sized.
            let is_auto = self.sizing.map_is_none();
            let base = is_auto.select(regions.base, size);
            let expand = regions.expand | !is_auto;

            Regions::one(size, base, expand)
        };

        // Layout the child.
        let mut frames = self.child.layout(ctx, &pod, styles)?;

        // Ensure frame size matches regions size if expansion is on.
        let frame = &mut frames[0];
        let target = regions.expand.select(regions.first, frame.size);
        Arc::make_mut(frame).resize(target, Align::LEFT_TOP);

        Ok(frames)
    }
}

/// Fill the frames resulting from a node.
#[derive(Debug, Hash)]
struct FillNode {
    /// How to fill the frames resulting from the `child`.
    fill: Paint,
    /// The node to fill.
    child: LayoutNode,
}

impl Layout for FillNode {
    fn layout(
        &self,
        ctx: &mut Context,
        regions: &Regions,
        styles: StyleChain,
    ) -> TypResult<Vec<Arc<Frame>>> {
        let mut frames = self.child.layout(ctx, regions, styles)?;
        for frame in &mut frames {
            let shape = Shape::filled(Geometry::Rect(frame.size), self.fill);
            Arc::make_mut(frame).prepend(Point::zero(), Element::Shape(shape));
        }
        Ok(frames)
    }
}

/// Stroke the frames resulting from a node.
#[derive(Debug, Hash)]
struct StrokeNode {
    /// How to stroke the frames resulting from the `child`.
    stroke: Stroke,
    /// The node to stroke.
    child: LayoutNode,
}

impl Layout for StrokeNode {
    fn layout(
        &self,
        ctx: &mut Context,
        regions: &Regions,
        styles: StyleChain,
    ) -> TypResult<Vec<Arc<Frame>>> {
        let mut frames = self.child.layout(ctx, regions, styles)?;
        for frame in &mut frames {
            let shape = Shape::stroked(Geometry::Rect(frame.size), self.stroke);
            Arc::make_mut(frame).prepend(Point::zero(), Element::Shape(shape));
        }
        Ok(frames)
    }
}
