use super::*;

/// A node that adds padding to its child.
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct PadNode {
    /// The amount of padding.
    pub padding: Sides<Linear>,
    /// The child node whose sides to pad.
    pub child: AnyNode,
}

impl Layout for PadNode {
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
    ) -> Vec<Constrained<Rc<Frame>>> {
        let mut regions = regions.map(|size| size - self.padding.resolve(size).size());

        let frames = self.child.layout(ctx, &regions);

        frames
            .into_iter()
            .map(|frame| {
                let padded = solve(self.padding, frame.size);
                let padding = self.padding.resolve(padded);
                let origin = Point::new(padding.left, padding.top);

                let mut new_frame = Frame::new(padded, frame.baseline + origin.y);
                new_frame.push_frame(origin, frame.item);

                let mut frame = new_frame.constrain(frame.constraints);
                frame.constraints.mutate(padding.size() * -1.0);

                if self.padding.left.is_relative() || self.padding.right.is_relative() {
                    frame.constraints.base.horizontal = Some(regions.base.width);
                }
                if self.padding.top.is_relative() || self.padding.bottom.is_relative() {
                    frame.constraints.base.vertical = Some(regions.base.height);
                }

                regions.next();
                frame
            })
            .collect()
    }
}

/// Solve for the size `padded` that satisfies (approximately):
/// `padded - padding.resolve(padded).size() == size`
fn solve(padding: Sides<Linear>, size: Size) -> Size {
    fn solve_axis(length: Length, padding: Linear) -> Length {
        (length + padding.abs) / (1.0 - padding.rel.get())
    }

    Size::new(
        solve_axis(size.width, padding.left + padding.right),
        solve_axis(size.height, padding.top + padding.bottom),
    )
}

impl From<PadNode> for AnyNode {
    fn from(pad: PadNode) -> Self {
        Self::new(pad)
    }
}
