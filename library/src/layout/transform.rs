use typst::geom::Transform;

use crate::prelude::*;

/// Move content without affecting layout.
#[derive(Debug, Hash)]
pub struct MoveNode {
    /// The offset by which to move the content.
    pub delta: Axes<Rel<Length>>,
    /// The content that should be moved.
    pub child: Content,
}

#[node(LayoutInline)]
impl MoveNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let dx = args.named("dx")?.unwrap_or_default();
        let dy = args.named("dy")?.unwrap_or_default();
        Ok(Self {
            delta: Axes::new(dx, dy),
            child: args.expect("body")?,
        }
        .pack())
    }
}

impl LayoutInline for MoveNode {
    fn layout_inline(
        &self,
        world: Tracked<dyn World>,
        regions: &Regions,
        styles: StyleChain,
    ) -> SourceResult<Frame> {
        let mut frame = self.child.layout_inline(world, regions, styles)?;
        let delta = self.delta.resolve(styles);
        let delta = delta.zip(frame.size()).map(|(d, s)| d.relative_to(s));
        frame.translate(delta.to_point());
        Ok(frame)
    }
}

/// Transform content without affecting layout.
#[derive(Debug, Hash)]
pub struct TransformNode<const T: TransformKind> {
    /// Transformation to apply to the content.
    pub transform: Transform,
    /// The content that should be transformed.
    pub child: Content,
}

/// Rotate content without affecting layout.
pub type RotateNode = TransformNode<ROTATE>;

/// Scale content without affecting layout.
pub type ScaleNode = TransformNode<SCALE>;

#[node(LayoutInline)]
impl<const T: TransformKind> TransformNode<T> {
    /// The origin of the transformation.
    #[property(resolve)]
    pub const ORIGIN: Axes<Option<GenAlign>> = Axes::default();

    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
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

        Ok(Self { transform, child: args.expect("body")? }.pack())
    }
}

impl<const T: TransformKind> LayoutInline for TransformNode<T> {
    fn layout_inline(
        &self,
        world: Tracked<dyn World>,
        regions: &Regions,
        styles: StyleChain,
    ) -> SourceResult<Frame> {
        let mut frame = self.child.layout_inline(world, regions, styles)?;

        let origin = styles.get(Self::ORIGIN).unwrap_or(Align::CENTER_HORIZON);
        let Axes { x, y } = origin.zip(frame.size()).map(|(o, s)| o.position(s));
        let transform = Transform::translate(x, y)
            .pre_concat(self.transform)
            .pre_concat(Transform::translate(-x, -y));
        frame.transform(transform);

        Ok(frame)
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
