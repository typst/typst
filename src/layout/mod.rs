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
use std::rc::Rc;

use crate::font::FontStore;
use crate::frame::Frame;
use crate::geom::{Align, Linear, Point, Sides, Spec, Transform};
use crate::image::ImageStore;
use crate::library::{AlignNode, DocumentNode, PadNode, SizedNode, TransformNode};
use crate::Context;

/// Layout a document node into a collection of frames.
pub fn layout(ctx: &mut Context, node: &DocumentNode) -> Vec<Rc<Frame>> {
    let mut ctx = LayoutContext::new(ctx);
    node.layout(&mut ctx)
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
    ) -> Vec<Constrained<Rc<Frame>>>;

    /// Convert to a packed node.
    fn pack(self) -> PackedNode
    where
        Self: Debug + Hash + Sized + 'static,
    {
        PackedNode {
            #[cfg(feature = "layout-cache")]
            hash: {
                let mut state = fxhash::FxHasher64::default();
                self.type_id().hash(&mut state);
                self.hash(&mut state);
                state.finish()
            },
            node: Rc::new(self),
        }
    }
}

/// A packed layouting node with precomputed hash.
#[derive(Clone)]
pub struct PackedNode {
    node: Rc<dyn Bounds>,
    #[cfg(feature = "layout-cache")]
    hash: u64,
}

impl PackedNode {
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
        self.transformed(Transform::translation(offset.x, offset.y), Align::LEFT_TOP)
    }

    /// Transform this node's contents without affecting layout.
    pub fn transformed(self, transform: Transform, origin: Spec<Align>) -> Self {
        if !transform.is_identity() {
            TransformNode { transform, origin, child: self }.pack()
        } else {
            self
        }
    }
}

impl Layout for PackedNode {
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
    ) -> Vec<Constrained<Rc<Frame>>> {
        #[cfg(not(feature = "layout-cache"))]
        return self.node.layout(ctx, regions);

        #[cfg(feature = "layout-cache")]
        ctx.layouts.get(self.hash, regions).unwrap_or_else(|| {
            ctx.level += 1;
            let frames = self.node.layout(ctx, regions);
            ctx.level -= 1;

            let entry = FramesEntry::new(frames.clone(), ctx.level);

            #[cfg(debug_assertions)]
            if !entry.check(regions) {
                eprintln!("node: {:#?}", self.node);
                eprintln!("regions: {:#?}", regions);
                eprintln!(
                    "constraints: {:#?}",
                    frames.iter().map(|c| c.cts).collect::<Vec<_>>()
                );
                panic!("constraints did not match regions they were created for");
            }

            ctx.layouts.insert(self.hash, entry);
            frames
        })
    }

    fn pack(self) -> PackedNode
    where
        Self: Sized + Hash + 'static,
    {
        self
    }
}

impl Hash for PackedNode {
    fn hash<H: Hasher>(&self, _state: &mut H) {
        #[cfg(feature = "layout-cache")]
        _state.write_u64(self.hash);
        #[cfg(not(feature = "layout-cache"))]
        unimplemented!()
    }
}

impl Debug for PackedNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.node.fmt(f)
    }
}

trait Bounds: Layout + Debug + 'static {
    fn as_any(&self) -> &dyn Any;
}

impl<T> Bounds for T
where
    T: Layout + Debug + 'static,
{
    fn as_any(&self) -> &dyn Any {
        self
    }
}
