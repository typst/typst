use std::f64::consts::SQRT_2;

use super::prelude::*;
use crate::util::RcExt;

/// `rect`: A rectangle with optional content.
pub fn rect(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let width = args.named("width")?;
    let height = args.named("height")?;
    shape_impl(args, ShapeKind::Rect, width, height)
}

/// `square`: A square with optional content.
pub fn square(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let size = args.named::<Length>("size")?.map(Linear::from);
    let width = match size {
        None => args.named("width")?,
        size => size,
    };
    let height = match size {
        None => args.named("height")?,
        size => size,
    };
    shape_impl(args, ShapeKind::Square, width, height)
}

/// `ellipse`: An ellipse with optional content.
pub fn ellipse(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let width = args.named("width")?;
    let height = args.named("height")?;
    shape_impl(args, ShapeKind::Ellipse, width, height)
}

/// `circle`: A circle with optional content.
pub fn circle(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let diameter = args.named("radius")?.map(|r: Length| 2.0 * Linear::from(r));
    let width = match diameter {
        None => args.named("width")?,
        diameter => diameter,
    };
    let height = match diameter {
        None => args.named("height")?,
        diameter => diameter,
    };
    shape_impl(args, ShapeKind::Circle, width, height)
}

fn shape_impl(
    args: &mut Args,
    kind: ShapeKind,
    width: Option<Linear>,
    height: Option<Linear>,
) -> TypResult<Value> {
    // The default appearance of a shape.
    let default = Stroke {
        paint: RgbaColor::BLACK.into(),
        thickness: Length::pt(1.0),
    };

    // Parse fill & stroke.
    let fill = args.named("fill")?.unwrap_or(None);
    let stroke = match (args.named("stroke")?, args.named("thickness")?) {
        (None, None) => fill.is_none().then(|| default),
        (color, thickness) => color.unwrap_or(Some(default.paint)).map(|paint| Stroke {
            paint,
            thickness: thickness.unwrap_or(default.thickness),
        }),
    };

    // Shorthand for padding.
    let padding = Sides::splat(args.named("padding")?.unwrap_or_default());

    // The shape's contents.
    let body = args.find::<Template>();

    Ok(Value::Template(Template::from_inline(move |style| {
        ShapeNode {
            kind,
            fill,
            stroke,
            child: body.as_ref().map(|body| body.pack(style).padded(padding)),
        }
        .pack()
        .sized(Spec::new(width, height))
    })))
}

/// Places its child into a sizable and fillable shape.
#[derive(Debug, Hash)]
pub struct ShapeNode {
    /// Which shape to place the child into.
    pub kind: ShapeKind,
    /// How to fill the shape.
    pub fill: Option<Paint>,
    /// How the stroke the shape.
    pub stroke: Option<Stroke>,
    /// The child node to place into the shape, if any.
    pub child: Option<PackedNode>,
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
        // Layout, either with or without child.
        let mut frame = if let Some(child) = &self.child {
            let mut node: &dyn Layout = child;

            let storage;
            if matches!(self.kind, ShapeKind::Circle | ShapeKind::Ellipse) {
                // Padding with this ratio ensures that a rectangular child fits
                // perfectly into a circle / an ellipse.
                let ratio = Relative::new(0.5 - SQRT_2 / 4.0);
                storage = child.clone().padded(Sides::splat(ratio.into()));
                node = &storage;
            }

            // Now, layout the child.
            let mut frames = node.layout(ctx, regions);

            if matches!(self.kind, ShapeKind::Square | ShapeKind::Circle) {
                // Relayout with full expansion into square region to make sure
                // the result is really a square or circle.
                let size = frames[0].item.size;
                let mut pod = regions.clone();
                pod.current.w = size.w.max(size.h).min(pod.current.w);
                pod.current.h = pod.current.w;
                pod.expand = Spec::splat(true);
                frames = node.layout(ctx, &pod);
            }

            // TODO: What if there are multiple or no frames?
            // Extract the frame.
            Rc::take(frames.into_iter().next().unwrap().item)
        } else {
            // When there's no child, fill the area if expansion is on,
            // otherwise fall back to a default size.
            let default = Length::pt(30.0);
            let mut size = Size::new(
                if regions.expand.x {
                    regions.current.w
                } else {
                    // For rectangle and ellipse, the default shape is a bit
                    // wider than high.
                    match self.kind {
                        ShapeKind::Square | ShapeKind::Circle => default,
                        ShapeKind::Rect | ShapeKind::Ellipse => 1.5 * default,
                    }
                },
                if regions.expand.y { regions.current.h } else { default },
            );

            // Don't overflow the region.
            size.w = size.w.min(regions.current.w);
            size.h = size.h.min(regions.current.h);

            if matches!(self.kind, ShapeKind::Square | ShapeKind::Circle) {
                size.w = size.w.min(size.h);
                size.h = size.w;
            }

            Frame::new(size)
        };

        // Add fill and/or stroke.
        if self.fill.is_some() || self.stroke.is_some() {
            let geometry = match self.kind {
                ShapeKind::Square | ShapeKind::Rect => Geometry::Rect(frame.size),
                ShapeKind::Circle | ShapeKind::Ellipse => Geometry::Ellipse(frame.size),
            };

            let shape = Shape {
                geometry,
                fill: self.fill,
                stroke: self.stroke,
            };

            frame.prepend(Point::zero(), Element::Shape(shape));
        }

        // Ensure frame size matches regions size if expansion is on.
        let expand = regions.expand;
        frame.size = Size::new(
            if expand.x { regions.current.w } else { frame.size.w },
            if expand.y { regions.current.h } else { frame.size.h },
        );

        // Return tight constraints for now.
        vec![frame.constrain(Constraints::tight(regions))]
    }
}
