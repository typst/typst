//! Layouting.

mod constraints;
mod deco;
mod frame;
mod grid;
mod image;
#[cfg(feature = "layout-cache")]
mod incremental;
mod pad;
mod par;
mod regions;
mod shape;
mod stack;
mod text;

pub use self::image::*;
pub use constraints::*;
pub use deco::*;
pub use frame::*;
pub use grid::*;
#[cfg(feature = "layout-cache")]
pub use incremental::*;
pub use pad::*;
pub use par::*;
pub use regions::*;
pub use shape::*;
pub use stack::*;
pub use text::*;

use std::fmt::{self, Debug, Formatter};
use std::rc::Rc;

use crate::font::FontStore;
use crate::geom::*;
use crate::image::ImageStore;
use crate::util::OptionExt;
use crate::Context;

#[cfg(feature = "layout-cache")]
use {
    fxhash::FxHasher64,
    std::any::Any,
    std::hash::{Hash, Hasher},
};

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

/// Page-level nodes directly produce frames representing pages.
///
/// Such nodes create their own regions instead of being supplied with them from
/// some parent.
pub trait PageLevel: Debug {
    /// Layout the node, producing one frame per page.
    fn layout(&self, ctx: &mut LayoutContext) -> Vec<Rc<Frame>>;
}

/// Layouts its children onto one or multiple pages.
#[derive(Debug)]
pub struct PageNode {
    /// The size of the page.
    pub size: Size,
    /// The node that produces the actual pages.
    pub child: BlockNode,
}

impl PageLevel for PageNode {
    fn layout(&self, ctx: &mut LayoutContext) -> Vec<Rc<Frame>> {
        // When one of the lengths is infinite the page fits its content along
        // that axis.
        let expand = self.size.to_spec().map(Length::is_finite);
        let regions = Regions::repeat(self.size, self.size, expand);
        self.child.layout(ctx, &regions).into_iter().map(|c| c.item).collect()
    }
}

impl<T> PageLevel for T
where
    T: AsRef<[PageNode]> + Debug + ?Sized,
{
    fn layout(&self, ctx: &mut LayoutContext) -> Vec<Rc<Frame>> {
        self.as_ref().iter().flat_map(|node| node.layout(ctx)).collect()
    }
}

/// Block-level nodes can be layouted into a sequence of regions.
///
/// They return one frame per used region alongside constraints that define
/// whether the result is reusable in other regions.
pub trait BlockLevel: Debug {
    /// Layout the node into the given regions, producing constrained frames.
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
    ) -> Vec<Constrained<Rc<Frame>>>;
}

/// A dynamic [block-level](BlockLevel) layouting node.
#[derive(Clone)]
pub struct BlockNode {
    node: Rc<dyn BlockLevel>,
    #[cfg(feature = "layout-cache")]
    hash: u64,
}

impl BlockNode {
    /// Create a new dynamic node from any block-level node.
    #[cfg(not(feature = "layout-cache"))]
    pub fn new<T>(node: T) -> Self
    where
        T: BlockLevel + 'static,
    {
        Self { node: Rc::new(node) }
    }

    /// Create a new dynamic node from any block-level node.
    #[cfg(feature = "layout-cache")]
    pub fn new<T>(node: T) -> Self
    where
        T: BlockLevel + Hash + 'static,
    {
        Self {
            hash: hash_node(&node),
            node: Rc::new(node),
        }
    }
}

impl BlockLevel for BlockNode {
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
}

#[cfg(feature = "layout-cache")]
impl Hash for BlockNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.hash);
    }
}

impl Debug for BlockNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.node.fmt(f)
    }
}

/// Inline-level nodes are layouted as part of paragraph layout.
///
/// They only know the width and not the height of the paragraph's region and
/// return only a single frame.
pub trait InlineLevel: Debug {
    /// Layout the node into a frame.
    fn layout(&self, ctx: &mut LayoutContext, space: Length, base: Size) -> Frame;
}

/// A dynamic [inline-level](InlineLevel) layouting node.
#[derive(Clone)]
pub struct InlineNode {
    node: Rc<dyn InlineLevel>,
    #[cfg(feature = "layout-cache")]
    hash: u64,
}

impl InlineNode {
    /// Create a new dynamic node from any inline-level node.
    #[cfg(not(feature = "layout-cache"))]
    pub fn new<T>(node: T) -> Self
    where
        T: InlineLevel + 'static,
    {
        Self { node: Rc::new(node) }
    }

    /// Create a new dynamic node from any inline-level node.
    #[cfg(feature = "layout-cache")]
    pub fn new<T>(node: T) -> Self
    where
        T: InlineLevel + Hash + 'static,
    {
        Self {
            hash: hash_node(&node),
            node: Rc::new(node),
        }
    }
}

impl InlineLevel for InlineNode {
    fn layout(&self, ctx: &mut LayoutContext, space: Length, base: Size) -> Frame {
        self.node.layout(ctx, space, base)
    }
}

#[cfg(feature = "layout-cache")]
impl Hash for InlineNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.hash);
    }
}

impl Debug for InlineNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.node.fmt(f)
    }
}

/// Hash a node alongside its type id.
#[cfg(feature = "layout-cache")]
fn hash_node(node: &(impl Hash + 'static)) -> u64 {
    let mut state = FxHasher64::default();
    node.type_id().hash(&mut state);
    node.hash(&mut state);
    state.finish()
}

/// Kinds of spacing.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Spacing {
    /// A length stated in absolute values and/or relative to the parent's size.
    Linear(Linear),
    /// A length that is the fraction of the remaining free space in the parent.
    Fractional(Fractional),
}
