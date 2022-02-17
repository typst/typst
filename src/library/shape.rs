//! Colorable geometrical shapes.

use std::f64::consts::SQRT_2;

use super::prelude::*;
use super::TextNode;

/// Place a node into a sizable and fillable shape.
#[derive(Debug, Hash)]
pub struct ShapeNode<const S: ShapeKind>(pub Option<LayoutNode>);

#[class]
impl<const S: ShapeKind> ShapeNode<S> {
    /// How to fill the shape.
    pub const FILL: Option<Paint> = None;
    /// How the stroke the shape.
    pub const STROKE: Smart<Option<Paint>> = Smart::Auto;
    /// The stroke's thickness.
    pub const THICKNESS: Length = Length::pt(1.0);
    /// How much to pad the shape's content.
    pub const PADDING: Linear = Linear::zero();

    fn construct(_: &mut Vm, args: &mut Args) -> TypResult<Template> {
        let size = match S {
            SQUARE => args.named::<Length>("size")?.map(Linear::from),
            CIRCLE => args.named::<Length>("radius")?.map(|r| 2.0 * Linear::from(r)),
            _ => None,
        };

        let width = match size {
            None => args.named("width")?,
            size => size,
        };

        let height = match size {
            None => args.named("height")?,
            size => size,
        };

        Ok(Template::inline(
            Self(args.find()?).pack().sized(Spec::new(width, height)),
        ))
    }
}

impl<const S: ShapeKind> Layout for ShapeNode<S> {
    fn layout(
        &self,
        vm: &mut Vm,
        regions: &Regions,
        styles: StyleChain,
    ) -> Vec<Constrained<Arc<Frame>>> {
        let mut frames;
        if let Some(child) = &self.0 {
            let mut padding = styles.get(Self::PADDING);
            if is_round(S) {
                padding.rel += Relative::new(0.5 - SQRT_2 / 4.0);
            }

            // Pad the child.
            let child = child.clone().padded(Sides::splat(padding));

            let mut pod = Regions::one(regions.current, regions.base, regions.expand);
            frames = child.layout(vm, &pod, styles);

            // Relayout with full expansion into square region to make sure
            // the result is really a square or circle.
            if is_quadratic(S) {
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
                frames = child.layout(vm, &pod, styles);
                frames[0].cts = Constraints::tight(regions);
            }
        } else {
            // The default size that a shape takes on if it has no child and
            // enough space.
            let mut size =
                Size::new(Length::pt(45.0), Length::pt(30.0)).min(regions.current);

            if is_quadratic(S) {
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

        let frame = Arc::make_mut(&mut frames[0].item);

        // Add fill and/or stroke.
        let fill = styles.get(Self::FILL);
        let thickness = styles.get(Self::THICKNESS);
        let stroke = styles
            .get(Self::STROKE)
            .unwrap_or(fill.is_none().then(|| Color::BLACK.into()))
            .map(|paint| Stroke { paint, thickness });

        if fill.is_some() || stroke.is_some() {
            let geometry = if is_round(S) {
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

/// A category of shape.
pub type ShapeKind = usize;

/// A rectangle with equal side lengths.
pub const SQUARE: ShapeKind = 0;

/// A quadrilateral with four right angles.
pub const RECT: ShapeKind = 1;

/// An ellipse with coinciding foci.
pub const CIRCLE: ShapeKind = 2;

/// A curve around two focal points.
pub const ELLIPSE: ShapeKind = 3;

/// Whether a shape kind is curvy.
fn is_round(kind: ShapeKind) -> bool {
    matches!(kind, CIRCLE | ELLIPSE)
}

/// Whether a shape kind has equal side length.
fn is_quadratic(kind: ShapeKind) -> bool {
    matches!(kind, SQUARE | CIRCLE)
}
