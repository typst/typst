use std::f64::consts::SQRT_2;

use crate::library::prelude::*;
use crate::library::text::TextNode;

/// Place a node into a sizable and fillable shape.
#[derive(Debug, Hash)]
pub struct AngularNode<const S: ShapeKind>(pub Option<LayoutNode>);

/// Place a node into a square.
pub type SquareNode = AngularNode<SQUARE>;

/// Place a node into a rectangle.
pub type RectNode = AngularNode<RECT>;

// /// Place a node into a sizable and fillable shape.
// #[derive(Debug, Hash)]
// pub struct RoundNode<const S: ShapeKind>(pub Option<LayoutNode>);

// /// Place a node into a circle.
// pub type CircleNode = RoundNode<CIRCLE>;

// /// Place a node into an ellipse.
// pub type EllipseNode = RoundNode<ELLIPSE>;

#[node]
impl<const S: ShapeKind> AngularNode<S> {
    /// How to fill the shape.
    pub const FILL: Option<Paint> = None;
    /// How to stroke the shape.
    #[property(resolve, fold)]
    pub const STROKE: Smart<Sides<Option<RawStroke>>> = Smart::Auto;

    /// How much to pad the shape's content.
    #[property(resolve, fold)]
    pub const INSET: Sides<Option<Relative<RawLength>>> = Sides::splat(Relative::zero());

    /// How much to extend the shape's dimensions beyond the allocated space.
    #[property(resolve, fold)]
    pub const OUTSET: Sides<Option<Relative<RawLength>>> = Sides::splat(Relative::zero());

    /// How much to round the shape's corners.
    #[property(resolve, fold)]
    pub const RADIUS: Sides<Option<Relative<RawLength>>> = Sides::splat(Relative::zero());

    fn construct(_: &mut Context, args: &mut Args) -> TypResult<Content> {
        let size = args.named::<RawLength>("size")?.map(Relative::from);

        let width = match size {
            None => args.named("width")?,
            size => size,
        };

        let height = match size {
            None => args.named("height")?,
            size => size,
        };

        Ok(Content::inline(
            Self(args.find()?).pack().sized(Spec::new(width, height)),
        ))
    }
}

castable! {
    Sides<Option<RawStroke>>,
    Expected: "stroke, dictionary with strokes for each side",
    Value::None => {
        Sides::splat(None)
    },
    Value::Dict(values) => {
        let get = |name: &str| values.get(name.into()).and_then(|v| v.clone().cast()).unwrap_or(None);
        Sides {
            top: get("top"),
            right: get("right"),
            bottom: get("bottom"),
            left: get("left"),
        }
    },
    Value::Length(thickness) => Sides::splat(Some(RawStroke {
        paint: Smart::Auto,
        thickness: Smart::Custom(thickness),
    })),
    Value::Color(color) => Sides::splat(Some(RawStroke {
        paint: Smart::Custom(color.into()),
        thickness: Smart::Auto,
    })),
    @stroke: RawStroke => Sides::splat(Some(*stroke)),
}

castable! {
    Sides<Option<Relative<RawLength>>>,
    Expected: "length or dictionary of lengths for each side",
    Value::None => Sides::splat(None),
    Value::Dict(values) => {
        let get = |name: &str| values.get(name.into()).and_then(|v| v.clone().cast()).unwrap_or(None);
        Sides {
            top: get("top"),
            right: get("right"),
            bottom: get("bottom"),
            left: get("left"),
        }
    },
    Value::Length(l) => Sides::splat(Some(l.into())),
    Value::Relative(r) => Sides::splat(Some(r)),
}

impl<const S: ShapeKind> Layout for AngularNode<S> {
    fn layout(
        &self,
        ctx: &mut Context,
        regions: &Regions,
        styles: StyleChain,
    ) -> TypResult<Vec<Arc<Frame>>> {
        let mut frames;
        if let Some(child) = &self.0 {
            let inset = styles.get(Self::INSET);

            // Pad the child.
            let child = child
                .clone()
                .padded(inset.map(|side| side.map(|abs| RawLength::from(abs))));

            let mut pod = Regions::one(regions.first, regions.base, regions.expand);
            frames = child.layout(ctx, &pod, styles)?;

            // Relayout with full expansion into square region to make sure
            // the result is really a square or circle.
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
        } else {
            // The default size that a shape takes on if it has no child and
            // enough space.
            let mut size =
                Size::new(Length::pt(45.0), Length::pt(30.0)).min(regions.first);

            let length = if regions.expand.x || regions.expand.y {
                let target = regions.expand.select(regions.first, Size::zero());
                target.x.max(target.y)
            } else {
                size.x.min(size.y)
            };
            size = Size::splat(length);

            frames = vec![Arc::new(Frame::new(size))];
        }

        let frame = Arc::make_mut(&mut frames[0]);

        // Add fill and/or stroke.
        let fill = styles.get(Self::FILL);
        let stroke = match styles.get(Self::STROKE) {
            Smart::Auto if fill.is_none() => Sides::splat(Some(Stroke::default())),
            Smart::Auto => Sides::splat(None),
            Smart::Custom(strokes) => strokes.map(|s| Some(s.unwrap_or_default())),
        };

        let radius = {
            let radius = styles.get(Self::RADIUS);

            Sides {
                left: radius.left.relative_to(frame.size.x / 2.0),
                top: radius.top.relative_to(frame.size.y / 2.0),
                right: radius.right.relative_to(frame.size.x / 2.0),
                bottom: radius.bottom.relative_to(frame.size.y / 2.0),
            }
        };

        if fill.is_some() || stroke.iter().any(Option::is_some) {
            let shape = Shape {
                geometry: Geometry::Rect(frame.size, radius),
                fill,
                stroke,
            };
            frame.prepend(Point::zero(), Element::Shape(shape));
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
