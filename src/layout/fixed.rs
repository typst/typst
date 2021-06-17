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
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
    ) -> Vec<Constrained<Frame>> {
        let Regions { current, base, .. } = regions;
        let mut constraints = Constraints::new(regions.expand);
        constraints.set_base_using_linears(Spec::new(self.width, self.height), &regions);

        let size = Size::new(
            self.width.map_or(current.width, |w| w.resolve(base.width)),
            self.height.map_or(current.height, |h| h.resolve(base.height)),
        );

        // If one dimension was not specified, the `current` size needs to remain static.
        if self.width.is_none() {
            constraints.exact.horizontal = Some(current.width);
        }
        if self.height.is_none() {
            constraints.exact.vertical = Some(current.height);
        }

        let expand = Spec::new(self.width.is_some(), self.height.is_some());
        let regions = Regions::one(size, expand);
        let mut frames = self.child.layout(ctx, &regions);

        if let Some(frame) = frames.first_mut() {
            frame.constraints = constraints;
        }

        frames
    }
}

impl From<FixedNode> for AnyNode {
    fn from(fixed: FixedNode) -> Self {
        Self::new(fixed)
    }
}
