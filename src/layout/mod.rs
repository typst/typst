//! Layouting.

mod constraints;
#[cfg(feature = "layout-cache")]
mod incremental;
mod levels;
mod regions;

pub use constraints::*;
#[cfg(feature = "layout-cache")]
pub use incremental::*;
pub use levels::*;
pub use regions::*;

use std::rc::Rc;

use crate::font::FontStore;
use crate::frame::Frame;
use crate::image::ImageStore;
use crate::Context;

/// Layout a page-level node into a collection of frames.
pub fn layout<T>(ctx: &mut Context, node: &T) -> Vec<Rc<Frame>>
where
    T: PageLevel + ?Sized,
{
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
