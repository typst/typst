use super::*;

/// A node that can fix its child's width and height.
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct FixedNode {
    /// The fixed width, if any.
    pub width: Option<Linear>,
    /// The fixed height, if any.
    pub height: Option<Linear>,
    /// The child node whose size to fix.
    pub child: AnyNode,
}

impl Layout for FixedNode {
    fn layout(&self, ctx: &mut LayoutContext, regions: &Regions) -> Vec<Frame> {
        let Regions { current, base, .. } = regions;
        let size = Size::new(
            self.width.map_or(current.width, |w| w.resolve(base.width)),
            self.height.map_or(current.height, |h| h.resolve(base.height)),
        );

        let expand = Spec::new(self.width.is_some(), self.height.is_some());
        let regions = Regions::one(size, expand);
        self.child.layout(ctx, &regions)
    }
}

impl From<FixedNode> for AnyNode {
    fn from(fixed: FixedNode) -> Self {
        Self::new(fixed)
    }
}
