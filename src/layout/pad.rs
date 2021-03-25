use super::*;

/// A node that adds padding to its child.
#[derive(Debug, Clone, PartialEq)]
pub struct PadNode {
    /// The amount of padding.
    pub padding: Sides<Linear>,
    /// The child node whose sides to pad.
    pub child: AnyNode,
}

impl Layout for PadNode {
    fn layout(&self, ctx: &mut LayoutContext, areas: &Areas) -> Vec<Frame> {
        let areas = shrink(areas, self.padding);
        let mut frames = self.child.layout(ctx, &areas);
        for frame in &mut frames {
            pad(frame, self.padding);
        }
        frames
    }
}

impl From<PadNode> for AnyNode {
    fn from(pad: PadNode) -> Self {
        Self::new(pad)
    }
}

/// Shrink all areas by the padding.
fn shrink(areas: &Areas, padding: Sides<Linear>) -> Areas {
    areas.map(|size| size - padding.resolve(size).size())
}

/// Pad the frame and move all elements inwards.
fn pad(frame: &mut Frame, padding: Sides<Linear>) {
    let padded = solve(padding, frame.size);
    let padding = padding.resolve(padded);
    let origin = Point::new(padding.left, padding.top);

    frame.size = padded;
    for (point, _) in &mut frame.elements {
        *point += origin;
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
