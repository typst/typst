//! Affine transformations on nodes.

use super::prelude::*;
use crate::geom::Transform;

/// `move`: Move content without affecting layout.
pub fn move_(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let tx = args.named("x")?.unwrap_or_default();
    let ty = args.named("y")?.unwrap_or_default();
    let transform = Transform::translation(tx, ty);
    transform_impl(args, transform)
}

/// `scale`: Scale content without affecting layout.
pub fn scale(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let all = args.find();
    let sx = args.named("x")?.or(all).unwrap_or(Relative::one());
    let sy = args.named("y")?.or(all).unwrap_or(Relative::one());
    let transform = Transform::scaling(sx, sy);
    transform_impl(args, transform)
}

/// `rotate`: Rotate content without affecting layout.
pub fn rotate(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let angle = args.named("angle")?.or_else(|| args.find()).unwrap_or_default();
    let transform = Transform::rotation(angle);
    transform_impl(args, transform)
}

fn transform_impl(args: &mut Args, transform: Transform) -> TypResult<Value> {
    let body: Node = args.expect("body")?;
    let origin = args
        .named("origin")?
        .unwrap_or(Spec::splat(None))
        .unwrap_or(Align::CENTER_HORIZON);

    Ok(Value::inline(
        body.into_block().transformed(transform, origin),
    ))
}

/// A node that transforms its child without affecting layout.
#[derive(Debug, Hash)]
pub struct TransformNode {
    /// Transformation to apply to the contents.
    pub transform: Transform,
    /// The origin of the transformation.
    pub origin: Spec<Align>,
    /// The node whose contents should be transformed.
    pub child: PackedNode,
}

impl Layout for TransformNode {
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
    ) -> Vec<Constrained<Rc<Frame>>> {
        let mut frames = self.child.layout(ctx, regions);

        for Constrained { item: frame, .. } in &mut frames {
            let Spec { x, y } = self.origin.zip(frame.size).map(|(o, s)| o.resolve(s));
            let transform = Transform::translation(x, y)
                .pre_concat(self.transform)
                .pre_concat(Transform::translation(-x, -y));

            Rc::make_mut(frame).transform(transform);
        }

        frames
    }
}
