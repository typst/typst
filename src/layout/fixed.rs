use super::*;
use crate::geom::Linear;

/// A node that can fix its child's width and height.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeFixed {
    /// The fixed width, if any.
    pub width: Option<Linear>,
    /// The fixed height, if any.
    pub height: Option<Linear>,
    /// The child node whose size to fix.
    pub child: Node,
}

impl Layout for NodeFixed {
    fn layout(&self, ctx: &mut LayoutContext, areas: &Areas) -> Layouted {
        let Areas { current, full, .. } = areas;
        let size = Size::new(
            self.width.map(|w| w.resolve(full.width)).unwrap_or(current.width),
            self.height.map(|h| h.resolve(full.height)).unwrap_or(current.height),
        );

        let areas = Areas::once(size);
        self.child.layout(ctx, &areas)
    }
}

impl From<NodeFixed> for NodeAny {
    fn from(fixed: NodeFixed) -> Self {
        Self::new(fixed)
    }
}
