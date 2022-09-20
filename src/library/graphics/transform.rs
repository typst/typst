use crate::geom::Transform;
use crate::library::prelude::*;

/// Move a node without affecting layout.
#[derive(Debug, Hash)]
pub struct MoveNode {
    /// The offset by which to move the node.
    pub delta: Spec<Relative<RawLength>>,
    /// The node whose contents should be moved.
    pub child: LayoutNode,
}

#[node]
impl MoveNode {
    fn construct(_: &mut Vm, args: &mut Args) -> TypResult<Content> {
        let dx = args.named("dx")?.unwrap_or_default();
        let dy = args.named("dy")?.unwrap_or_default();
        Ok(Content::inline(Self {
            delta: Spec::new(dx, dy),
            child: args.expect("body")?,
        }))
    }
}

impl Layout for MoveNode {
    fn layout(
        &self,
        world: &dyn World,
        regions: &Regions,
        styles: StyleChain,
    ) -> TypResult<Vec<Frame>> {
        let mut frames = self.child.layout(world, regions, styles)?;

        let delta = self.delta.resolve(styles);
        for frame in &mut frames {
            let delta = delta.zip(frame.size()).map(|(d, s)| d.relative_to(s));
            frame.translate(delta.to_point());
        }

        Ok(frames)
    }
}

/// Transform a node without affecting layout.
#[derive(Debug, Hash)]
pub struct TransformNode<const T: TransformKind> {
    /// Transformation to apply to the contents.
    pub transform: Transform,
    /// The node whose contents should be transformed.
    pub child: LayoutNode,
}

/// Rotate a node without affecting layout.
pub type RotateNode = TransformNode<ROTATE>;

/// Scale a node without affecting layout.
pub type ScaleNode = TransformNode<SCALE>;

#[node]
impl<const T: TransformKind> TransformNode<T> {
    /// The origin of the transformation.
    #[property(resolve)]
    pub const ORIGIN: Spec<Option<RawAlign>> = Spec::default();

    fn construct(_: &mut Vm, args: &mut Args) -> TypResult<Content> {
        let transform = match T {
            ROTATE => {
                let angle = args.named_or_find("angle")?.unwrap_or_default();
                Transform::rotate(angle)
            }
            SCALE | _ => {
                let all = args.find()?;
                let sx = args.named("x")?.or(all).unwrap_or(Ratio::one());
                let sy = args.named("y")?.or(all).unwrap_or(Ratio::one());
                Transform::scale(sx, sy)
            }
        };

        Ok(Content::inline(Self {
            transform,
            child: args.expect("body")?,
        }))
    }
}

impl<const T: TransformKind> Layout for TransformNode<T> {
    fn layout(
        &self,
        world: &dyn World,
        regions: &Regions,
        styles: StyleChain,
    ) -> TypResult<Vec<Frame>> {
        let origin = styles.get(Self::ORIGIN).unwrap_or(Align::CENTER_HORIZON);
        let mut frames = self.child.layout(world, regions, styles)?;

        for frame in &mut frames {
            let Spec { x, y } = origin.zip(frame.size()).map(|(o, s)| o.position(s));
            let transform = Transform::translate(x, y)
                .pre_concat(self.transform)
                .pre_concat(Transform::translate(-x, -y));

            frame.transform(transform);
        }

        Ok(frames)
    }
}

/// Kinds of transformations.
///
/// The move transformation is handled separately.
pub type TransformKind = usize;

/// A rotational transformation.
const ROTATE: TransformKind = 1;

/// A scale transformation.
const SCALE: TransformKind = 2;
