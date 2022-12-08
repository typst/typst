use typst::geom::Transform;

use crate::prelude::*;

/// Move content without affecting layout.
#[derive(Debug, Hash)]
pub struct MoveNode {
    /// The offset by which to move the content.
    pub delta: Axes<Rel<Length>>,
    /// The content that should be moved.
    pub body: Content,
}

#[node(Layout, Inline)]
impl MoveNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let dx = args.named("dx")?.unwrap_or_default();
        let dy = args.named("dy")?.unwrap_or_default();
        Ok(Self {
            delta: Axes::new(dx, dy),
            body: args.expect("body")?,
        }
        .pack())
    }
}

impl Layout for MoveNode {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let mut fragment = self.body.layout(vt, styles, regions)?;
        for frame in &mut fragment {
            let delta = self.delta.resolve(styles);
            let delta = delta.zip(frame.size()).map(|(d, s)| d.relative_to(s));
            frame.translate(delta.to_point());
        }
        Ok(fragment)
    }
}

impl Inline for MoveNode {}

/// Transform content without affecting layout.
#[derive(Debug, Hash)]
pub struct TransformNode<const T: TransformKind> {
    /// Transformation to apply to the content.
    pub transform: Transform,
    /// The content that should be transformed.
    pub body: Content,
}

/// Rotate content without affecting layout.
pub type RotateNode = TransformNode<ROTATE>;

/// Scale content without affecting layout.
pub type ScaleNode = TransformNode<SCALE>;

#[node(Layout, Inline)]
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

        Ok(Self { transform, body: args.expect("body")? }.pack())
    }
}

impl<const T: TransformKind> Layout for TransformNode<T> {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let mut fragment = self.body.layout(vt, styles, regions)?;
        for frame in &mut fragment {
            let origin = styles.get(Self::ORIGIN).unwrap_or(Align::CENTER_HORIZON);
            let Axes { x, y } = origin.zip(frame.size()).map(|(o, s)| o.position(s));
            let transform = Transform::translate(x, y)
                .pre_concat(self.transform)
                .pre_concat(Transform::translate(-x, -y));
            frame.transform(transform);
        }
        Ok(fragment)
    }
}

impl<const T: TransformKind> Inline for TransformNode<T> {}

/// Kinds of transformations.
///
/// The move transformation is handled separately.
pub type TransformKind = usize;

/// A rotational transformation.
const ROTATE: TransformKind = 1;

/// A scale transformation.
const SCALE: TransformKind = 2;
