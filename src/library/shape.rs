use std::f64::consts::SQRT_2;

use super::prelude::*;
use super::PadNode;
use crate::util::RcExt;

/// `rect`: A rectangle with optional content.
pub fn rect(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let width = args.named("width")?;
    let height = args.named("height")?;
    let fill = args.named("fill")?;
    let body = args.find();
    Ok(shape_impl(ShapeKind::Rect, width, height, fill, body))
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
    let fill = args.named("fill")?;
    let body = args.find();
    Ok(shape_impl(ShapeKind::Square, width, height, fill, body))
}

/// `ellipse`: An ellipse with optional content.
pub fn ellipse(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let width = args.named("width")?;
    let height = args.named("height")?;
    let fill = args.named("fill")?;
    let body = args.find();
    Ok(shape_impl(ShapeKind::Ellipse, width, height, fill, body))
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
    let fill = args.named("fill")?;
    let body = args.find();
    Ok(shape_impl(ShapeKind::Circle, width, height, fill, body))
}

fn shape_impl(
    kind: ShapeKind,
    width: Option<Linear>,
    height: Option<Linear>,
    fill: Option<Color>,
    body: Option<Template>,
) -> Value {
    // Set default fill if there's no fill.
    let fill = fill.unwrap_or(Color::Rgba(RgbaColor::gray(175)));

    Value::Template(Template::from_inline(move |style| {
        ShapeNode {
            kind,
            fill: Some(Paint::Color(fill)),
            child: body.as_ref().map(|body| body.pack(style)),
        }
        .pack()
        .sized(width, height)
    }))
}

/// Places its child into a sizable and fillable shape.
#[derive(Debug, Hash)]
pub struct ShapeNode {
    /// Which shape to place the child into.
    pub kind: ShapeKind,
    /// How to fill the shape, if at all.
    pub fill: Option<Paint>,
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
                storage = PadNode {
                    padding: Sides::splat(Relative::new(0.5 - SQRT_2 / 4.0).into()),
                    child: child.clone(),
                };
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

            // Extract the frame.
            Rc::take(frames.into_iter().next().unwrap().item)
        } else {
            // When there's no child, fill the area if expansion is on,
            // otherwise fall back to a default size.
            let default = Length::pt(30.0);
            let size = Size::new(
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

            Frame::new(size, size.h)
        };

        // Add background fill if desired.
        if let Some(fill) = self.fill {
            let (pos, geometry) = match self.kind {
                ShapeKind::Square | ShapeKind::Rect => {
                    (Point::zero(), Geometry::Rect(frame.size))
                }
                ShapeKind::Circle | ShapeKind::Ellipse => {
                    (frame.size.to_point() / 2.0, Geometry::Ellipse(frame.size))
                }
            };

            frame.prepend(pos, Element::Geometry(geometry, fill));
        }

        // Return tight constraints for now.
        let mut cts = Constraints::new(regions.expand);
        cts.exact = regions.current.to_spec().map(Some);
        cts.base = regions.base.to_spec().map(Some);
        vec![frame.constrain(cts)]
    }
}
