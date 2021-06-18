use super::*;

/// A node that places a rectangular filled background behind its child.
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct BackgroundNode {
    /// The kind of shape to use as a background.
    pub shape: BackgroundShape,
    /// The background fill.
    pub fill: Fill,
    /// The child node to be filled.
    pub child: AnyNode,
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
        let frames = self.child.layout(ctx, regions);

        frames
            .into_iter()
            .map(|frame| {
                let (point, shape) = match self.shape {
                    BackgroundShape::Rect => (Point::zero(), Shape::Rect(frame.size)),
                    BackgroundShape::Ellipse => {
                        (frame.size.to_point() / 2.0, Shape::Ellipse(frame.size))
                    }
                };
                let element = Element::Geometry(shape, self.fill);
                let mut new_frame = Frame::new(frame.size, frame.baseline);
                new_frame.push(point, element);
                new_frame.push_frame(Point::zero(), frame.item);
                new_frame.constrain(frame.constraints)
            })
            .collect()
    }
}

impl From<BackgroundNode> for AnyNode {
    fn from(background: BackgroundNode) -> Self {
        Self::new(background)
    }
}
