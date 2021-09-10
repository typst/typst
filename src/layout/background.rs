use super::*;

/// A node that places a rectangular filled background behind its child.
#[derive(Debug)]
#[cfg_attr(feature = "layout-cache", derive(Hash))]
pub struct BackgroundNode {
    /// The kind of shape to use as a background.
    pub shape: BackgroundShape,
    /// Background color / texture.
    pub fill: Paint,
    /// The child node to be filled.
    pub child: LayoutNode,
}

/// The kind of shape to use as a background.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum BackgroundShape {
    Rect,
    Ellipse,
}

impl Layout for BackgroundNode {
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
    ) -> Vec<Constrained<Rc<Frame>>> {
        let mut frames = self.child.layout(ctx, regions);

        for Constrained { item: frame, .. } in &mut frames {
            let (point, geometry) = match self.shape {
                BackgroundShape::Rect => (Point::zero(), Geometry::Rect(frame.size)),
                BackgroundShape::Ellipse => {
                    (frame.size.to_point() / 2.0, Geometry::Ellipse(frame.size))
                }
            };

            // Create a new frame with the background geometry and the child's
            // frame.
            let empty = Frame::new(frame.size, frame.baseline);
            let prev = std::mem::replace(frame, Rc::new(empty));
            let new = Rc::make_mut(frame);
            new.push(point, Element::Geometry(geometry, self.fill));
            new.push_frame(Point::zero(), prev);
        }

        frames
    }
}

impl From<BackgroundNode> for LayoutNode {
    fn from(background: BackgroundNode) -> Self {
        Self::new(background)
    }
}
