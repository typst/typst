use super::*;

use std::any::Any;

#[cfg(feature = "layout-cache")]
use std::hash::{Hash, Hasher};

#[cfg(feature = "layout-cache")]
use fxhash::FxHasher64;

/// A tree of layout nodes.
pub struct LayoutTree {
    /// Runs of pages with the same properties.
    pub runs: Vec<PageRun>,
}

impl LayoutTree {
    /// Layout the tree into a collection of frames.
    pub fn layout(&self, ctx: &mut LayoutContext) -> Vec<Rc<Frame>> {
        self.runs.iter().flat_map(|run| run.layout(ctx)).collect()
    }
}

/// A run of pages that all have the same properties.
pub struct PageRun {
    /// The size of each page.
    pub size: Size,
    /// The layout node that produces the actual pages (typically a
    /// [`StackNode`]).
    pub child: LayoutNode,
}

impl PageRun {
    /// Layout the page run.
    pub fn layout(&self, ctx: &mut LayoutContext) -> Vec<Rc<Frame>> {
        // When one of the lengths is infinite the page fits its content along
        // that axis.
        let expand = self.size.to_spec().map(Length::is_finite);
        let regions = Regions::repeat(self.size, self.size, expand);
        self.child.layout(ctx, &regions).into_iter().map(|c| c.item).collect()
    }
}

/// A dynamic layouting node.
pub struct LayoutNode {
    node: Box<dyn Layout>,
    #[cfg(feature = "layout-cache")]
    hash: u64,
}

impl LayoutNode {
    /// Create a new instance from any node that satisifies the required bounds.
    #[cfg(not(feature = "layout-cache"))]
    pub fn new<T>(node: T) -> Self
    where
        T: Layout + 'static,
    {
        Self { node: Box::new(node) }
    }

    /// Create a new instance from any node that satisifies the required bounds.
    #[cfg(feature = "layout-cache")]
    pub fn new<T>(node: T) -> Self
    where
        T: Layout + Hash + 'static,
    {
        let hash = {
            let mut state = FxHasher64::default();
            node.type_id().hash(&mut state);
            node.hash(&mut state);
            state.finish()
        };

        Self { node: Box::new(node), hash }
    }
}

impl Layout for LayoutNode {
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
            ctx.layouts.insert(self.hash, frames.clone(), ctx.level);
            frames
        })
    }
}

#[cfg(feature = "layout-cache")]
impl Hash for LayoutNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.hash);
    }
}
