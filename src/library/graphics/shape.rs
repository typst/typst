use std::f64::consts::SQRT_2;

use crate::library::prelude::*;
use crate::library::text::TextNode;

/// Place a node into a sizable and fillable shape.
#[derive(Debug, Hash)]
pub struct ShapeNode<const S: ShapeKind>(pub Option<LayoutNode>);

/// Place a node into a square.
pub type SquareNode = ShapeNode<SQUARE>;

/// Place a node into a rectangle.
pub type RectNode = ShapeNode<RECT>;

/// Place a node into a circle.
pub type CircleNode = ShapeNode<CIRCLE>;

/// Place a node into an ellipse.
pub type EllipseNode = ShapeNode<ELLIPSE>;

#[node]
impl<const S: ShapeKind> ShapeNode<S> {
    /// How to fill the shape.
    pub const FILL: Option<Paint> = None;
    /// How to stroke the shape.
    #[property(skip, resolve, fold)]
    pub const STROKE: Smart<Sides<Option<RawStroke>>> = Smart::Auto;

    /// How much to pad the shape's content.
    #[property(resolve, fold)]
    pub const INSET: Sides<Option<Relative<RawLength>>> = Sides::splat(Relative::zero());
    /// How much to extend the shape's dimensions beyond the allocated space.
    #[property(resolve, fold)]
    pub const OUTSET: Sides<Option<Relative<RawLength>>> = Sides::splat(Relative::zero());
    /// How much to round the shape's corners.
    #[property(skip, resolve, fold)]
    pub const RADIUS: Sides<Option<Relative<RawLength>>> = Sides::splat(Relative::zero());

    fn construct(_: &mut Machine, args: &mut Args) -> TypResult<Content> {
        let size = match S {
            SQUARE => args.named::<RawLength>("size")?.map(Relative::from),
            CIRCLE => args.named::<RawLength>("radius")?.map(|r| 2.0 * Relative::from(r)),
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

        Ok(Content::inline(
            Self(args.eat()?).pack().sized(Spec::new(width, height)),
        ))
    }

    fn set(...) {
        if is_round(S) {
            styles.set_opt(
                Self::STROKE,
                args.named::<Smart<Option<RawStroke>>>("stroke")?
                    .map(|some| some.map(Sides::splat)),
            );
        } else {
            styles.set_opt(Self::STROKE, args.named("stroke")?);
            styles.set_opt(Self::RADIUS, args.named("radius")?);
        }
    }
}

impl<const S: ShapeKind> Layout for ShapeNode<S> {
    fn layout(
        &self,
        ctx: &mut Context,
        regions: &Regions,
        styles: StyleChain,
    ) -> TypResult<Vec<Arc<Frame>>> {
        let mut frames;
        if let Some(child) = &self.0 {
            let mut inset = styles.get(Self::INSET);
            if is_round(S) {
                inset = inset.map(|side| side + Ratio::new(0.5 - SQRT_2 / 4.0));
            }

            // Pad the child.
            let child = child.clone().padded(inset.map(|side| side.map(RawLength::from)));

            let mut pod = Regions::one(regions.first, regions.base, regions.expand);
            let role_map = StyleMap::with_role(Role::GenericBlock);
            let styles = role_map.chain(&styles);

            frames = child.layout(ctx, &pod, styles)?;

            // Relayout with full expansion into square region to make sure
            // the result is really a square or circle.
            if is_quadratic(S) {
                let length = if regions.expand.x || regions.expand.y {
                    let target = regions.expand.select(regions.first, Size::zero());
                    target.x.max(target.y)
                } else {
                    let size = frames[0].size;
                    let desired = size.x.max(size.y);
                    desired.min(regions.first.x).min(regions.first.y)
                };

                pod.first = Size::splat(length);
                pod.expand = Spec::splat(true);
                frames = child.layout(ctx, &pod, styles)?;
            }
        } else {
            // The default size that a shape takes on if it has no child and
            // enough space.
            let mut size =
                Size::new(Length::pt(45.0), Length::pt(30.0)).min(regions.first);

            if is_quadratic(S) {
                let length = if regions.expand.x || regions.expand.y {
                    let target = regions.expand.select(regions.first, Size::zero());
                    target.x.max(target.y)
                } else {
                    size.x.min(size.y)
                };
                size = Size::splat(length);
            } else {
                size = regions.expand.select(regions.first, size);
            }

            frames = vec![Arc::new(Frame::new(size))];
        }

        let frame = Arc::make_mut(&mut frames[0]);

        // Add fill and/or stroke.
        let fill = styles.get(Self::FILL);
        let stroke = match styles.get(Self::STROKE) {
            Smart::Auto if fill.is_none() => Sides::splat(Some(Stroke::default())),
            Smart::Auto => Sides::splat(None),
            Smart::Custom(strokes) => {
                strokes.map(|s| s.map(RawStroke::unwrap_or_default))
            }
        };

        let outset = styles.get(Self::OUTSET).relative_to(frame.size);
        let size = frame.size + outset.sum_by_axis();

        let radius = styles
            .get(Self::RADIUS)
            .map(|side| side.relative_to(size.x.min(size.y) / 2.0));

        let pos = Point::new(-outset.left, -outset.top);

        if fill.is_some() || stroke.iter().any(Option::is_some) {
            if is_round(S) {
                let shape = Shape {
                    geometry: Geometry::Ellipse(size),
                    fill,
                    stroke: stroke.left,
                };
                frame.prepend(pos, Element::Shape(shape));
            } else {
                frame.prepend_multiple(
                    Rect::new(size, radius)
                        .shapes(fill, stroke)
                        .into_iter()
                        .map(|x| (pos, Element::Shape(x))),
                )
            }
        }

        // Apply link if it exists.
        if let Some(url) = styles.get(TextNode::LINK) {
            frame.link(url.clone());
        }

        Ok(frames)
    }
}

/// A category of shape.
pub type ShapeKind = usize;

/// A rectangle with equal side lengths.
const SQUARE: ShapeKind = 0;

/// A quadrilateral with four right angles.
const RECT: ShapeKind = 1;

/// An ellipse with coinciding foci.
const CIRCLE: ShapeKind = 2;

/// A curve around two focal points.
const ELLIPSE: ShapeKind = 3;

/// Whether a shape kind is curvy.
fn is_round(kind: ShapeKind) -> bool {
    matches!(kind, CIRCLE | ELLIPSE)
}

/// Whether a shape kind has equal side length.
fn is_quadratic(kind: ShapeKind) -> bool {
    matches!(kind, SQUARE | CIRCLE)
}
