use super::*;

/// A node that places a rectangular filled background behind its child.
#[derive(Debug, Clone, PartialEq)]
pub struct BackgroundNode {
    /// The background fill.
    pub fill: Fill,
    /// The child node to be filled.
    pub child: Node,
}

impl Layout for BackgroundNode {
    fn layout(&self, ctx: &mut LayoutContext, areas: &Areas) -> Fragment {
        let mut layouted = self.child.layout(ctx, areas);

        for frame in layouted.frames_mut() {
            let element = Element::Geometry(Geometry {
                shape: Shape::Rect(frame.size),
                fill: self.fill,
            });
            frame.elements.insert(0, (Point::ZERO, element));
        }

        layouted
    }
}

impl From<BackgroundNode> for AnyNode {
    fn from(background: BackgroundNode) -> Self {
        Self::new(background)
    }
}
