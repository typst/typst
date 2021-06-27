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
        let mut original = regions.clone();
        let mut regions = regions.map(|size| size - self.padding.resolve(size).size());

        let mut frames = self.child.layout(ctx, &regions);

        for frame in &mut frames {
            let padded = solve(self.padding, frame.size);
            let padding = self.padding.resolve(padded);
            let origin = Point::new(padding.left, padding.top);

            let mut new = Frame::new(padded, frame.baseline + origin.y);
            let prev = std::mem::take(&mut frame.item);
            new.push_frame(origin, prev);

            frame.constraints.mutate(padding.size(), &original);

            if self.padding.left.is_relative() || self.padding.right.is_relative() {
                frame.constraints.base.horizontal = Some(original.base.width);
            }
            if self.padding.top.is_relative() || self.padding.bottom.is_relative() {
                frame.constraints.base.vertical = Some(original.base.height);
            }

            regions.next();
            original.next();
            *Rc::make_mut(&mut frame.item) = new;
        }
        frames
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
