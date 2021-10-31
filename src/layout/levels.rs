use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use super::*;
use crate::geom::{Length, Size};

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

    /// Convert to a packed block-level node.
    fn pack(self) -> BlockNode
    where
        Self: Sized + Hash + 'static,
    {
        BlockNode {
            #[cfg(feature = "layout-cache")]
            hash: hash_node(&self),
            node: Rc::new(self),
        }
    }
}

/// A packed [block-level](BlockLevel) layouting node with precomputed hash.
#[derive(Clone)]
pub struct BlockNode {
    node: Rc<dyn BlockLevel>,
    #[cfg(feature = "layout-cache")]
    hash: u64,
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

    fn pack(self) -> BlockNode
    where
        Self: Sized + Hash + 'static,
    {
        self
    }
}

impl Hash for BlockNode {
    fn hash<H: Hasher>(&self, _state: &mut H) {
        #[cfg(feature = "layout-cache")]
        _state.write_u64(self.hash);
        #[cfg(not(feature = "layout-cache"))]
        unimplemented!()
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

    /// Convert to a packed inline-level node.
    fn pack(self) -> InlineNode
    where
        Self: Sized + Hash + 'static,
    {
        InlineNode {
            #[cfg(feature = "layout-cache")]
            hash: hash_node(&self),
            node: Rc::new(self),
        }
    }
}

/// A packed [inline-level](InlineLevel) layouting node with precomputed hash.
#[derive(Clone)]
pub struct InlineNode {
    node: Rc<dyn InlineLevel>,
    #[cfg(feature = "layout-cache")]
    hash: u64,
}

impl InlineLevel for InlineNode {
    fn layout(&self, ctx: &mut LayoutContext, space: Length, base: Size) -> Frame {
        self.node.layout(ctx, space, base)
    }

    fn pack(self) -> InlineNode
    where
        Self: Sized + Hash + 'static,
    {
        self
    }
}

impl Hash for InlineNode {
    fn hash<H: Hasher>(&self, _state: &mut H) {
        #[cfg(feature = "layout-cache")]
        _state.write_u64(self.hash);
        #[cfg(not(feature = "layout-cache"))]
        unimplemented!()
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
    use std::any::Any;
    let mut state = fxhash::FxHasher64::default();
    node.type_id().hash(&mut state);
    node.hash(&mut state);
    state.finish()
}
