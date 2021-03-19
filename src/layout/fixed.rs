use super::*;

/// A node that can fix its child's width and height.
#[derive(Debug, Clone, PartialEq)]
pub struct FixedNode {
    /// The fixed width, if any.
    pub width: Option<Linear>,
    /// The fixed height, if any.
    pub height: Option<Linear>,
    /// The child node whose size to fix.
    pub child: Node,
}

impl Layout for FixedNode {
    fn layout(&self, ctx: &mut LayoutContext, areas: &Areas) -> Fragment {
        let Areas { current, full, .. } = areas;
        let size = Size::new(
            self.width.map(|w| w.resolve(full.width)).unwrap_or(current.width),
            self.height.map(|h| h.resolve(full.height)).unwrap_or(current.height),
        );

        let fill_if = |cond| if cond { Expand::Fill } else { Expand::Fit };
        let expand = Spec::new(
            fill_if(self.width.is_some()),
            fill_if(self.height.is_some()),
        );

        let areas = Areas::once(size, expand);
        self.child.layout(ctx, &areas)
    }
}

impl From<FixedNode> for AnyNode {
    fn from(fixed: FixedNode) -> Self {
        Self::new(fixed)
    }
}
