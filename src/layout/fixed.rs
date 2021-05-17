use super::*;

/// A node that can fix its child's width and height.
#[derive(Debug, Clone, PartialEq)]
pub struct FixedNode {
    /// The fixed width, if any.
    pub width: Option<Linear>,
    /// The fixed height, if any.
    pub height: Option<Linear>,
    /// The child node whose size to fix.
    pub child: AnyNode,
}

impl Layout for FixedNode {
    fn layout(&self, ctx: &mut LayoutContext, areas: &Areas) -> Vec<Frame> {
        let Areas { current, full, .. } = areas;
        let size = Size::new(
            self.width.map_or(current.width, |w| w.resolve(full.width)),
            self.height.map_or(current.height, |h| h.resolve(full.height)),
        );

        let fixed = Spec::new(self.width.is_some(), self.height.is_some());
        let areas = Areas::once(size, size, fixed);
        self.child.layout(ctx, &areas)
    }
}

impl From<FixedNode> for AnyNode {
    fn from(fixed: FixedNode) -> Self {
        Self::new(fixed)
    }
}
