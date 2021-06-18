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
        let mut frames = self.child.layout(ctx, regions);
        for frame in &mut frames {
            let mut new = Frame::new(frame.size, frame.baseline);

            let (point, shape) = match self.shape {
                BackgroundShape::Rect => (Point::zero(), Shape::Rect(frame.size)),
                BackgroundShape::Ellipse => {
                    (frame.size.to_point() / 2.0, Shape::Ellipse(frame.size))
                }
            };

            let element = Element::Geometry(shape, self.fill);
            new.push(point, element);

            let prev = std::mem::take(&mut frame.item);
            new.push_frame(Point::zero(), prev);

            *Rc::make_mut(&mut frame.item) = new;
        }
        frames
    }
}

impl From<BackgroundNode> for AnyNode {
    fn from(background: BackgroundNode) -> Self {
        Self::new(background)
    }
}
