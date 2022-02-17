//! Affine transformations on nodes.

use super::prelude::*;
use crate::geom::Transform;

/// Transform a node without affecting layout.
#[derive(Debug, Hash)]
pub struct TransformNode<const T: TransformKind> {
    /// Transformation to apply to the contents.
    pub transform: Transform,
    /// The node whose contents should be transformed.
    pub child: LayoutNode,
}

#[class]
impl<const T: TransformKind> TransformNode<T> {
    /// The origin of the transformation.
    pub const ORIGIN: Spec<Option<Align>> = Spec::default();

    fn construct(_: &mut Vm, args: &mut Args) -> TypResult<Template> {
        let transform = match T {
            MOVE => {
                let tx = args.named("x")?.unwrap_or_default();
                let ty = args.named("y")?.unwrap_or_default();
                Transform::translation(tx, ty)
            }
            ROTATE => {
                let angle = args.named_or_find("angle")?.unwrap_or_default();
                Transform::rotation(angle)
            }
            SCALE | _ => {
                let all = args.find()?;
                let sx = args.named("x")?.or(all).unwrap_or(Relative::one());
                let sy = args.named("y")?.or(all).unwrap_or(Relative::one());
                Transform::scale(sx, sy)
            }
        };

        Ok(Template::inline(Self {
            transform,
            child: args.expect("body")?,
        }))
    }
}

impl<const T: TransformKind> Layout for TransformNode<T> {
    fn layout(
        &self,
        vm: &mut Vm,
        regions: &Regions,
        styles: StyleChain,
    ) -> TypResult<Vec<Constrained<Arc<Frame>>>> {
        let origin = styles.get(Self::ORIGIN).unwrap_or(Align::CENTER_HORIZON);
        let mut frames = self.child.layout(vm, regions, styles)?;

        for Constrained { item: frame, .. } in &mut frames {
            let Spec { x, y } = origin.zip(frame.size).map(|(o, s)| o.resolve(s));
            let transform = Transform::translation(x, y)
                .pre_concat(self.transform)
                .pre_concat(Transform::translation(-x, -y));

            Arc::make_mut(frame).transform(transform);
        }

        Ok(frames)
    }
}

/// Kinds of transformations.
pub type TransformKind = usize;

/// A translation on the X and Y axes.
pub const MOVE: TransformKind = 0;

/// A rotational transformation.
pub const ROTATE: TransformKind = 1;

/// A scale transformation.
pub const SCALE: TransformKind = 2;
