use super::*;

/// A node that represents a rectangular box.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeBackground {
    /// The background fill.
    pub fill: Fill,
    /// The child node whose sides to pad.
    pub child: NodeFixed,
}

impl Layout for NodeBackground {
    fn layout(&self, ctx: &mut LayoutContext, areas: &Areas) -> Layouted {
        let mut layouted = self.child.layout(ctx, areas);

        if let Some(first) = layouted.frames_mut().first_mut() {
            first.elements.insert(
                0,
                (
                    Point::ZERO,
                    Element::Geometry(Geometry {
                        shape: Shape::Rect(first.size),
                        fill: self.fill.clone(),
                    }),
                ),
            )
        }

        layouted
    }
}

impl From<NodeBackground> for NodeAny {
    fn from(background: NodeBackground) -> Self {
        Self::new(background)
    }
}
