//! Colorable geometrical shapes.

use std::f64::consts::SQRT_2;

use super::prelude::*;
use super::LinkNode;

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
    let mut padding = args.named::<Linear>("padding")?.unwrap_or_default();

    // Padding with this ratio ensures that a rectangular child fits
    // perfectly into a circle / an ellipse.
    if kind.is_round() {
        padding.rel += Relative::new(0.5 - SQRT_2 / 4.0);
    }

    // The shape's contents.
    let child = args.find().map(|body: PackedNode| body.padded(Sides::splat(padding)));

    Ok(Value::inline(
        ShapeNode { kind, fill, stroke, child }
            .pack()
            .sized(Spec::new(width, height)),
    ))
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

impl Layout for ShapeNode {
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
        styles: StyleChain,
    ) -> Vec<Constrained<Rc<Frame>>> {
        let mut frames;
        if let Some(child) = &self.child {
            let mut pod = Regions::one(regions.current, regions.base, regions.expand);
            frames = child.layout(ctx, &pod, styles);

            // Relayout with full expansion into square region to make sure
            // the result is really a square or circle.
            if self.kind.is_quadratic() {
                let length = if regions.expand.x || regions.expand.y {
                    let target = regions.expand.select(regions.current, Size::zero());
                    target.x.max(target.y)
                } else {
                    let size = frames[0].item.size;
                    let desired = size.x.max(size.y);
                    desired.min(regions.current.x).min(regions.current.y)
                };

                pod.current = Size::splat(length);
                pod.expand = Spec::splat(true);
                frames = child.layout(ctx, &pod, styles);
                frames[0].cts = Constraints::tight(regions);
            }
        } else {
            // The default size that a shape takes on if it has no child and
            // enough space.
            let mut size =
                Size::new(Length::pt(45.0), Length::pt(30.0)).min(regions.current);

            if self.kind.is_quadratic() {
                let length = if regions.expand.x || regions.expand.y {
                    let target = regions.expand.select(regions.current, Size::zero());
                    target.x.max(target.y)
                } else {
                    size.x.min(size.y)
                };
                size = Size::splat(length);
            } else {
                size = regions.expand.select(regions.current, size);
            }

            frames = vec![Frame::new(size).constrain(Constraints::tight(regions))];
        }

        let frame = Rc::make_mut(&mut frames[0].item);

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

        // Apply link if it exists.
        if let Some(url) = styles.get_ref(LinkNode::URL) {
            frame.link(url);
        }

        frames
    }
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

impl ShapeKind {
    /// Whether the shape is curved.
    pub fn is_round(self) -> bool {
        matches!(self, Self::Circle | Self::Ellipse)
    }

    /// Whether the shape has a fixed 1-1 aspect ratio.
    pub fn is_quadratic(self) -> bool {
        matches!(self, Self::Square | Self::Circle)
    }
}
