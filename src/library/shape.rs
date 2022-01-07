//! Colorable geometrical shapes.

use std::f64::consts::SQRT_2;

use super::prelude::*;
use super::TextNode;

/// Places its child into a sizable and fillable shape.
#[derive(Debug, Hash)]
pub struct ShapeNode<S: ShapeKind> {
    /// Which shape to place the child into.
    pub kind: S,
    /// The child node to place into the shape, if any.
    pub child: Option<PackedNode>,
}

#[class]
impl<S: ShapeKind> ShapeNode<S> {
    /// How to fill the shape.
    pub const FILL: Option<Paint> = None;
    /// How the stroke the shape.
    pub const STROKE: Smart<Option<Paint>> = Smart::Auto;
    /// The stroke's thickness.
    pub const THICKNESS: Length = Length::pt(1.0);
    /// The How much to pad the shape's content.
    pub const PADDING: Linear = Linear::zero();

    fn construct(_: &mut EvalContext, args: &mut Args) -> TypResult<Node> {
        let size = if !S::ROUND && S::QUADRATIC {
            args.named::<Length>("size")?.map(Linear::from)
        } else if S::ROUND && S::QUADRATIC {
            args.named("radius")?.map(|r: Length| 2.0 * Linear::from(r))
        } else {
            None
        };

        let width = match size {
            None => args.named("width")?,
            size => size,
        };

        let height = match size {
            None => args.named("height")?,
            size => size,
        };

        Ok(Node::inline(
            ShapeNode { kind: S::default(), child: args.find() }
                .pack()
                .sized(Spec::new(width, height)),
        ))
    }

    fn set(args: &mut Args, styles: &mut StyleMap) -> TypResult<()> {
        styles.set_opt(Self::FILL, args.named("fill")?);
        styles.set_opt(Self::STROKE, args.named("stroke")?);
        styles.set_opt(Self::THICKNESS, args.named("thickness")?);
        styles.set_opt(Self::PADDING, args.named("padding")?);
        Ok(())
    }
}

impl<S: ShapeKind> Layout for ShapeNode<S> {
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
        styles: StyleChain,
    ) -> Vec<Constrained<Rc<Frame>>> {
        let mut frames;
        if let Some(child) = &self.child {
            let mut padding = styles.get(Self::PADDING);
            if S::ROUND {
                padding.rel += Relative::new(0.5 - SQRT_2 / 4.0);
            }

            // Pad the child.
            let child = child.clone().padded(Sides::splat(padding));

            let mut pod = Regions::one(regions.current, regions.base, regions.expand);
            frames = child.layout(ctx, &pod, styles);

            // Relayout with full expansion into square region to make sure
            // the result is really a square or circle.
            if S::QUADRATIC {
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

            if S::QUADRATIC {
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
        let fill = styles.get(Self::FILL);
        let thickness = styles.get(Self::THICKNESS);
        let stroke = styles
            .get(Self::STROKE)
            .unwrap_or(fill.is_none().then(|| RgbaColor::BLACK.into()))
            .map(|paint| Stroke { paint, thickness });

        if fill.is_some() || stroke.is_some() {
            let geometry = if S::ROUND {
                Geometry::Ellipse(frame.size)
            } else {
                Geometry::Rect(frame.size)
            };

            let shape = Shape { geometry, fill, stroke };
            frame.prepend(Point::zero(), Element::Shape(shape));
        }

        // Apply link if it exists.
        if let Some(url) = styles.get_ref(TextNode::LINK) {
            frame.link(url);
        }

        frames
    }
}

/// Categorizes shapes.
pub trait ShapeKind: Debug + Default + Hash + 'static {
    const ROUND: bool;
    const QUADRATIC: bool;
}

/// A rectangle with equal side lengths.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Square;

impl ShapeKind for Square {
    const ROUND: bool = false;
    const QUADRATIC: bool = true;
}

/// A quadrilateral with four right angles.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Rect;

impl ShapeKind for Rect {
    const ROUND: bool = false;
    const QUADRATIC: bool = false;
}

/// An ellipse with coinciding foci.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Circle;

impl ShapeKind for Circle {
    const ROUND: bool = true;
    const QUADRATIC: bool = true;
}

/// A curve around two focal points.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Ellipse;

impl ShapeKind for Ellipse {
    const ROUND: bool = true;
    const QUADRATIC: bool = false;
}
