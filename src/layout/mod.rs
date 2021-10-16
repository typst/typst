//! Layouting.

mod constraints;
mod frame;
mod grid;
mod image;
#[cfg(feature = "layout-cache")]
mod incremental;
mod pad;
mod par;
mod regions;
mod shape;
mod shaping;
mod spacing;
mod stack;
mod tree;

pub use self::image::*;
pub use constraints::*;
pub use frame::*;
pub use grid::*;
#[cfg(feature = "layout-cache")]
pub use incremental::*;
pub use pad::*;
pub use par::*;
pub use regions::*;
pub use shape::*;
pub use shaping::*;
pub use spacing::*;
pub use stack::*;
pub use tree::*;

use std::fmt::Debug;
use std::rc::Rc;

use crate::font::FontStore;
use crate::geom::*;
use crate::image::ImageStore;
use crate::util::OptionExt;
use crate::Context;

/// Layout a tree into a collection of frames.
pub fn layout(ctx: &mut Context, tree: &LayoutTree) -> Vec<Rc<Frame>> {
    let mut ctx = LayoutContext::new(ctx);
    tree.layout(&mut ctx)
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

/// Layout a node.
pub trait Layout: Debug {
    /// Layout the node into the given regions.
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
    ) -> Vec<Constrained<Rc<Frame>>>;
}
