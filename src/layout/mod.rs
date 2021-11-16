//! Layouting infrastructure.

mod constraints;
#[cfg(feature = "layout-cache")]
mod incremental;
mod regions;

pub use constraints::*;
#[cfg(feature = "layout-cache")]
pub use incremental::*;
pub use regions::*;

use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use crate::font::FontStore;
use crate::frame::Frame;
use crate::image::ImageStore;
use crate::library::DocumentNode;
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
pub trait Layout: Debug {
    /// Layout the node into the given regions, producing constrained frames.
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
    ) -> Vec<Constrained<Rc<Frame>>>;

    /// Convert to a packed node.
    fn pack(self) -> PackedNode
    where
        Self: Sized + Hash + 'static,
    {
        PackedNode {
            #[cfg(feature = "layout-cache")]
            hash: {
                use std::any::Any;
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
    node: Rc<dyn Layout>,
    #[cfg(feature = "layout-cache")]
    hash: u64,
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
