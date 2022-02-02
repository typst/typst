//! Layouting infrastructure.

mod constraints;
#[cfg(feature = "layout-cache")]
mod incremental;
mod regions;

pub use constraints::*;
#[cfg(feature = "layout-cache")]
pub use incremental::*;
pub use regions::*;

use std::any::Any;
use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::eval::{StyleChain, Styled};
use crate::font::FontStore;
use crate::frame::{Element, Frame, Geometry, Shape, Stroke};
use crate::geom::{Align, Linear, Paint, Point, Sides, Size, Spec};
use crate::image::ImageStore;
use crate::library::{AlignNode, Move, PadNode, PageNode, TransformNode};
use crate::Context;

/// The root layout node, a document consisting of top-level page runs.
#[derive(Hash)]
pub struct RootNode(pub Vec<Styled<PageNode>>);

impl RootNode {
    /// Layout the document into a sequence of frames, one per page.
    pub fn layout(&self, ctx: &mut Context) -> Vec<Arc<Frame>> {
        let (mut ctx, styles) = LayoutContext::new(ctx);
        self.0
            .iter()
            .flat_map(|styled| styled.item.layout(&mut ctx, styled.map.chain(&styles)))
            .collect()
    }
}

impl Debug for RootNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("Root ")?;
        f.debug_list().entries(&self.0).finish()
    }
}

/// A node that can be layouted into a sequence of regions.
///
/// Layout return one frame per used region alongside constraints that define
/// whether the result is reusable in other regions.
pub trait Layout {
    /// Layout the node into the given regions, producing constrained frames.
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
        styles: StyleChain,
    ) -> Vec<Constrained<Arc<Frame>>>;

    /// Convert to a packed node.
    fn pack(self) -> PackedNode
    where
        Self: Debug + Hash + Sized + Sync + Send + 'static,
    {
        PackedNode {
            #[cfg(feature = "layout-cache")]
            hash: self.hash64(),
            node: Arc::new(self),
        }
    }
}

/// The context for layouting.
pub struct LayoutContext<'a> {
    /// Stores parsed font faces.
    pub fonts: &'a mut FontStore,
    /// Stores decoded images.
    pub images: &'a mut ImageStore,
    /// Caches layouting artifacts.
    #[cfg(feature = "layout-cache")]
    pub layout_cache: &'a mut LayoutCache,
    /// How deeply nested the current layout tree position is.
    #[cfg(feature = "layout-cache")]
    level: usize,
}

impl<'a> LayoutContext<'a> {
    /// Create a new layout context.
    fn new(ctx: &'a mut Context) -> (Self, StyleChain<'a>) {
        let this = Self {
            fonts: &mut ctx.fonts,
            images: &mut ctx.images,
            #[cfg(feature = "layout-cache")]
            layout_cache: &mut ctx.layout_cache,
            #[cfg(feature = "layout-cache")]
            level: 0,
        };
        (this, StyleChain::new(&ctx.styles))
    }
}

/// A layout node that produces an empty frame.
///
/// The packed version of this is returned by [`PackedNode::default`].
#[derive(Debug, Hash)]
pub struct EmptyNode;

impl Layout for EmptyNode {
    fn layout(
        &self,
        _: &mut LayoutContext,
        regions: &Regions,
        _: StyleChain,
    ) -> Vec<Constrained<Arc<Frame>>> {
        let size = regions.expand.select(regions.current, Size::zero());
        let mut cts = Constraints::new(regions.expand);
        cts.exact = regions.current.filter(regions.expand);
        vec![Frame::new(size).constrain(cts)]
    }
}

/// A packed layouting node with a precomputed hash.
#[derive(Clone)]
pub struct PackedNode {
    /// The type-erased node.
    node: Arc<dyn Bounds>,
    /// A precomputed hash for the node.
    #[cfg(feature = "layout-cache")]
    hash: u64,
}

impl PackedNode {
    /// Check whether the contained node is a specific layout node.
    pub fn is<T: 'static>(&self) -> bool {
        self.node.as_any().is::<T>()
    }

    /// Try to downcast to a specific layout node.
    pub fn downcast<T>(&self) -> Option<&T>
    where
        T: Layout + Debug + Hash + 'static,
    {
        self.node.as_any().downcast_ref()
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
            TransformNode {
                kind: Move(offset.x, offset.y),
                child: self,
            }
            .pack()
        } else {
            self
        }
    }
}

impl Layout for PackedNode {
    #[track_caller]
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
        styles: StyleChain,
    ) -> Vec<Constrained<Arc<Frame>>> {
        let styles = styles.barred(self.node.as_any().type_id());

        #[cfg(not(feature = "layout-cache"))]
        return self.node.layout(ctx, regions, styles);

        #[cfg(feature = "layout-cache")]
        let hash = {
            let mut state = fxhash::FxHasher64::default();
            self.hash(&mut state);
            styles.hash(&mut state);
            state.finish()
        };

        // This is not written with `unwrap_or_else`, because then the
        // #[track_caller] annotation doesn't work.
        #[cfg(feature = "layout-cache")]
        if let Some(frames) = ctx.layout_cache.get(hash, regions) {
            frames
        } else {
            ctx.level += 1;
            let frames = self.node.layout(ctx, regions, styles);
            ctx.level -= 1;

            let entry = FramesEntry::new(frames.clone(), ctx.level);

            #[cfg(debug_assertions)]
            if !entry.check(regions) {
                eprintln!("node: {:#?}", self.node);
                eprintln!("regions: {regions:#?}");
                eprintln!(
                    "constraints: {:#?}",
                    frames.iter().map(|c| c.cts).collect::<Vec<_>>(),
                );
                panic!("constraints did not match regions they were created for");
            }

            ctx.layout_cache.insert(hash, entry);
            frames
        }
    }

    fn pack(self) -> PackedNode {
        self
    }
}

impl Default for PackedNode {
    fn default() -> Self {
        EmptyNode.pack()
    }
}

impl Debug for PackedNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.node.fmt(f)
    }
}

impl PartialEq for PackedNode {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(
            Arc::as_ptr(&self.node) as *const (),
            Arc::as_ptr(&other.node) as *const (),
        )
    }
}

impl Hash for PackedNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash the node.
        #[cfg(feature = "layout-cache")]
        state.write_u64(self.hash);
        #[cfg(not(feature = "layout-cache"))]
        state.write_u64(self.hash64());
    }
}

trait Bounds: Layout + Debug + Sync + Send + 'static {
    fn as_any(&self) -> &dyn Any;
    fn hash64(&self) -> u64;
}

impl<T> Bounds for T
where
    T: Layout + Hash + Debug + Sync + Send + 'static,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn hash64(&self) -> u64 {
        // Also hash the TypeId since nodes with different types but
        // equal data should be different.
        let mut state = fxhash::FxHasher64::default();
        self.type_id().hash(&mut state);
        self.hash(&mut state);
        state.finish()
    }
}

/// Fix the size of a node.
#[derive(Debug, Hash)]
pub struct SizedNode {
    /// How to size the node horizontally and vertically.
    pub sizing: Spec<Option<Linear>>,
    /// The node to be sized.
    pub child: PackedNode,
}

impl Layout for SizedNode {
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
        styles: StyleChain,
    ) -> Vec<Constrained<Arc<Frame>>> {
        let is_auto = self.sizing.map_is_none();
        let is_rel = self.sizing.map(|s| s.map_or(false, Linear::is_relative));

        // The "pod" is the region into which the child will be layouted.
        let pod = {
            // Resolve the sizing to a concrete size.
            let size = self
                .sizing
                .zip(regions.base)
                .map(|(s, b)| s.map(|v| v.resolve(b)))
                .unwrap_or(regions.current);

            // Select the appropriate base and expansion for the child depending
            // on whether it is automatically or linearly sized.
            let base = is_auto.select(regions.base, size);
            let expand = regions.expand | !is_auto;

            Regions::one(size, base, expand)
        };

        let mut frames = self.child.layout(ctx, &pod, styles);
        let Constrained { item: frame, cts } = &mut frames[0];

        // Ensure frame size matches regions size if expansion is on.
        let target = regions.expand.select(regions.current, frame.size);
        Arc::make_mut(frame).resize(target, Align::LEFT_TOP);

        // Set base & exact constraints if the child is automatically sized
        // since we don't know what the child might have done. Also set base if
        // our sizing is relative.
        *cts = Constraints::new(regions.expand);
        cts.exact = regions.current.filter(regions.expand | is_auto);
        cts.base = regions.base.filter(is_rel | is_auto);

        frames
    }
}

/// Fill the frames resulting from a node.
#[derive(Debug, Hash)]
pub struct FillNode {
    /// How to fill the frames resulting from the `child`.
    pub fill: Paint,
    /// The node to fill.
    pub child: PackedNode,
}

impl Layout for FillNode {
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
        styles: StyleChain,
    ) -> Vec<Constrained<Arc<Frame>>> {
        let mut frames = self.child.layout(ctx, regions, styles);
        for Constrained { item: frame, .. } in &mut frames {
            let shape = Shape::filled(Geometry::Rect(frame.size), self.fill);
            Arc::make_mut(frame).prepend(Point::zero(), Element::Shape(shape));
        }
        frames
    }
}

/// Stroke the frames resulting from a node.
#[derive(Debug, Hash)]
pub struct StrokeNode {
    /// How to stroke the frames resulting from the `child`.
    pub stroke: Stroke,
    /// The node to stroke.
    pub child: PackedNode,
}

impl Layout for StrokeNode {
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
        styles: StyleChain,
    ) -> Vec<Constrained<Arc<Frame>>> {
        let mut frames = self.child.layout(ctx, regions, styles);
        for Constrained { item: frame, .. } in &mut frames {
            let shape = Shape::stroked(Geometry::Rect(frame.size), self.stroke);
            Arc::make_mut(frame).prepend(Point::zero(), Element::Shape(shape));
        }
        frames
    }
}
