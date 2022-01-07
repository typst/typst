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

use crate::eval::{StyleChain, Styled};
use crate::font::FontStore;
use crate::frame::Frame;
use crate::geom::{Align, Linear, Point, Sides, Size, Spec};
use crate::image::ImageStore;
use crate::library::{AlignNode, Move, PadNode, PageNode, SizedNode, TransformNode};
use crate::Context;

/// The root layout node, a document consisting of top-level page runs.
#[derive(Hash)]
pub struct RootNode(pub Vec<Styled<PageNode>>);

impl RootNode {
    /// Layout the document into a sequence of frames, one per page.
    pub fn layout(&self, ctx: &mut Context) -> Vec<Rc<Frame>> {
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
    ) -> Vec<Constrained<Rc<Frame>>>;

    /// Convert to a packed node.
    fn pack(self) -> PackedNode
    where
        Self: Debug + Hash + Sized + 'static,
    {
        PackedNode {
            #[cfg(feature = "layout-cache")]
            hash: self.hash64(),
            node: Rc::new(self),
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
    pub layouts: &'a mut LayoutCache,
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
            layouts: &mut ctx.layouts,
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
    ) -> Vec<Constrained<Rc<Frame>>> {
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
    node: Rc<dyn Bounds>,
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
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
        styles: StyleChain,
    ) -> Vec<Constrained<Rc<Frame>>> {
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

        #[cfg(feature = "layout-cache")]
        ctx.layouts.get(hash, regions).unwrap_or_else(|| {
            ctx.level += 1;
            let frames = self.node.layout(ctx, regions, styles);
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

            ctx.layouts.insert(hash, entry);
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
            Rc::as_ptr(&self.node) as *const (),
            Rc::as_ptr(&other.node) as *const (),
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

trait Bounds: Layout + Debug + 'static {
    fn as_any(&self) -> &dyn Any;
    fn hash64(&self) -> u64;
}

impl<T> Bounds for T
where
    T: Layout + Hash + Debug + 'static,
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
