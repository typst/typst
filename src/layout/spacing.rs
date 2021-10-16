use super::*;

/// Spacing between other nodes.
#[derive(Debug)]
#[cfg_attr(feature = "layout-cache", derive(Hash))]
pub struct SpacingNode {
    /// Which axis to space on.
    pub axis: SpecAxis,
    /// How much spacing to add.
    pub amount: Linear,
}

impl Layout for SpacingNode {
    fn layout(
        &self,
        _: &mut LayoutContext,
        regions: &Regions,
    ) -> Vec<Constrained<Rc<Frame>>> {
        let base = regions.base.get(self.axis);
        let resolved = self.amount.resolve(base);
        let limit = regions.current.get(self.axis);

        // Generate constraints.
        let mut cts = Constraints::new(regions.expand);
        if self.amount.is_relative() {
            cts.base.set(self.axis, Some(base));
        }

        // If the spacing fits into the region, any larger region would also do.
        // If it was limited though, any change it region size might lead to
        // different results.
        if resolved < limit {
            cts.min.set(self.axis, Some(resolved));
        } else {
            cts.exact.set(self.axis, Some(limit));
        }

        // Create frame with limited spacing size along spacing axis and zero
        // extent along the other axis.
        let mut size = Size::zero();
        size.set(self.axis, resolved.min(limit));
        vec![Frame::new(size, size.h).constrain(cts)]
    }
}

impl From<SpacingNode> for LayoutNode {
    fn from(spacing: SpacingNode) -> Self {
        Self::new(spacing)
    }
}
