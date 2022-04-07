use crate::geom::Transform;
use crate::library::prelude::*;

/// Transform a node without affecting layout.
#[derive(Debug, Hash)]
pub struct TransformNode<const T: TransformKind> {
    /// Transformation to apply to the contents.
    pub transform: Transform,
    /// The node whose contents should be transformed.
    pub child: LayoutNode,
}

/// Transform a node by translating it without affecting layout.
pub type MoveNode = TransformNode<MOVE>;

/// Transform a node by rotating it without affecting layout.
pub type RotateNode = TransformNode<ROTATE>;

/// Transform a node by scaling it without affecting layout.
pub type ScaleNode = TransformNode<SCALE>;

#[node]
impl<const T: TransformKind> TransformNode<T> {
    /// The origin of the transformation.
    pub const ORIGIN: Spec<Option<Align>> = Spec::default();

    fn construct(_: &mut Context, args: &mut Args) -> TypResult<Content> {
        let transform = match T {
            MOVE => {
                let tx = args.named("x")?.unwrap_or_default();
                let ty = args.named("y")?.unwrap_or_default();
                Transform::translate(tx, ty)
            }
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
        ctx: &mut Context,
        regions: &Regions,
        styles: StyleChain,
    ) -> TypResult<Vec<Arc<Frame>>> {
        let origin = styles.get(Self::ORIGIN).unwrap_or(Align::CENTER_HORIZON);
        let mut frames = self.child.layout(ctx, regions, styles)?;

        for frame in &mut frames {
            let Spec { x, y } = origin.zip(frame.size).map(|(o, s)| o.resolve(s));
            let transform = Transform::translate(x, y)
                .pre_concat(self.transform)
                .pre_concat(Transform::translate(-x, -y));

            Arc::make_mut(frame).transform(transform);
        }

        Ok(frames)
    }
}

/// Kinds of transformations.
pub type TransformKind = usize;

/// A translation on the X and Y axes.
const MOVE: TransformKind = 0;

/// A rotational transformation.
const ROTATE: TransformKind = 1;

/// A scale transformation.
const SCALE: TransformKind = 2;
