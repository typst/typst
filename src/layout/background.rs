use super::*;

/// A node that places a rectangular filled background behind its child.
#[derive(Debug, Clone, PartialEq)]
pub struct BackgroundNode {
    /// The kind of shape to use as a background.
    pub shape: BackgroundShape,
    /// The background fill.
    pub fill: Fill,
    /// The child node to be filled.
    pub child: AnyNode,
}

/// The kind of shape to use as a background.
#[derive(Debug, Clone, PartialEq)]
pub enum BackgroundShape {
    Rect,
    Ellipse,
}

impl Layout for BackgroundNode {
    fn layout(&self, ctx: &mut LayoutContext, areas: &Areas) -> Vec<Frame> {
        let mut frames = self.child.layout(ctx, areas);

        for frame in &mut frames {
            let (point, shape) = match self.shape {
                BackgroundShape::Rect => (Point::ZERO, Shape::Rect(frame.size)),
                BackgroundShape::Ellipse => {
                    (frame.size.to_point() / 2.0, Shape::Ellipse(frame.size))
                }
            };

            let element = Element::Geometry(Geometry { shape, fill: self.fill });
            frame.elements.insert(0, (point, element));
        }

        frames
    }
}

impl From<BackgroundNode> for AnyNode {
    fn from(background: BackgroundNode) -> Self {
        Self::new(background)
    }
}
