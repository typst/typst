use std::f64::consts::SQRT_2;

use super::*;

/// Places its child into a sizable and fillable shape.
#[derive(Debug)]
#[cfg_attr(feature = "layout-cache", derive(Hash))]
pub struct ShapeNode {
    /// Which shape to place the child into.
    pub shape: ShapeKind,
    /// The width, if any.
    pub width: Option<Linear>,
    /// The height, if any.
    pub height: Option<Linear>,
    /// How to fill the shape, if at all.
    pub fill: Option<Paint>,
    /// The child node to place into the shape, if any.
    pub child: Option<LayoutNode>,
}

/// The type of a shape.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ShapeKind {
    /// A rectangle with equal side lengths.
    Square,
    /// A quadrilateral with four right angles.
    Rect,
    /// An ellipse with coinciding foci.
    Circle,
    /// A curve around two focal points.
    Ellipse,
}

impl Layout for ShapeNode {
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
    ) -> Vec<Constrained<Rc<Frame>>> {
        // Resolve width and height relative to the region's base.
        let width = self.width.map(|w| w.resolve(regions.base.w));
        let height = self.height.map(|h| h.resolve(regions.base.h));

        // Generate constraints.
        let constraints = {
            let mut cts = Constraints::new(regions.expand);
            cts.set_base_if_linear(regions.base, Spec::new(self.width, self.height));

            // Set tight exact and base constraints if the child is
            // automatically sized since we don't know what the child might do.
            if self.width.is_none() {
                cts.exact.x = Some(regions.current.w);
                cts.base.x = Some(regions.base.w);
            }

            // Same here.
            if self.height.is_none() {
                cts.exact.y = Some(regions.current.h);
                cts.base.y = Some(regions.base.h);
            }

            cts
        };

        // Layout.
        let mut frames = if let Some(child) = &self.child {
            let mut node: &dyn Layout = child;

            let padded;
            if matches!(self.shape, ShapeKind::Circle | ShapeKind::Ellipse) {
                // Padding with this ratio ensures that a rectangular child fits
                // perfectly into a circle / an ellipse.
                padded = PadNode {
                    padding: Sides::splat(Relative::new(0.5 - SQRT_2 / 4.0).into()),
                    child: child.clone(),
                };
                node = &padded;
            }

            // The "pod" is the region into which the child will be layouted.
            let mut pod = {
                let size = Size::new(
                    width.unwrap_or(regions.current.w),
                    height.unwrap_or(regions.current.h),
                );

                let base = Size::new(
                    if width.is_some() { size.w } else { regions.base.w },
                    if height.is_some() { size.h } else { regions.base.h },
                );

                let expand = Spec::new(width.is_some(), height.is_some());
                Regions::one(size, base, expand)
            };

            // Now, layout the child.
            let mut frames = node.layout(ctx, &pod);

            if matches!(self.shape, ShapeKind::Square | ShapeKind::Circle) {
                // Relayout with full expansion into square region to make sure
                // the result is really a square or circle.
                let size = frames[0].item.size;
                pod.current.w = size.w.max(size.h).min(pod.current.w);
                pod.current.h = pod.current.w;
                pod.expand = Spec::splat(true);
                frames = node.layout(ctx, &pod);
            }

            // Validate and set constraints.
            assert_eq!(frames.len(), 1);
            frames[0].constraints = constraints;
            frames
        } else {
            // Resolve shape size.
            let size = Size::new(width.unwrap_or_default(), height.unwrap_or_default());
            vec![Frame::new(size, size.h).constrain(constraints)]
        };

        // Add background shape if desired.
        if let Some(fill) = self.fill {
            let frame = Rc::make_mut(&mut frames[0].item);
            let (pos, geometry) = match self.shape {
                ShapeKind::Square | ShapeKind::Rect => {
                    (Point::zero(), Geometry::Rect(frame.size))
                }
                ShapeKind::Circle | ShapeKind::Ellipse => {
                    (frame.size.to_point() / 2.0, Geometry::Ellipse(frame.size))
                }
            };

            frame.prepend(pos, Element::Geometry(geometry, fill));
        }

        frames
    }
}

impl From<ShapeNode> for LayoutNode {
    fn from(shape: ShapeNode) -> Self {
        Self::new(shape)
    }
}
