use super::*;

/// A node that places a rectangular filled background behind another node.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeBackground {
    /// The background fill.
    pub fill: Fill,
    /// The child node to be filled.
    pub child: Node,
}

impl Layout for NodeBackground {
    fn layout(&self, ctx: &mut LayoutContext, areas: &Areas) -> Layouted {
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

impl From<NodeBackground> for NodeAny {
    fn from(background: NodeBackground) -> Self {
        Self::new(background)
    }
}
