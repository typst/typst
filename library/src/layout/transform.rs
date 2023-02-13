use typst::geom::Transform;

use crate::prelude::*;

/// # Move
/// Move content without affecting layout.
///
/// The `move` function allows you to move content while the layout still 'sees'
/// it at the original positions. Containers will still be sized as if the content
/// was not moved.
///
/// ## Example
/// ```example
/// #rect(inset: 0pt, move(
///   dx: 6pt, dy: 6pt,
///   rect(
///     inset: 8pt,
///     fill: white,
///     stroke: black,
///     [Abra cadabra]
///   )
/// ))
/// ```
///
/// ## Parameters
/// - body: `Content` (positional, required)
///   The content to move.
///
/// - dx: `Rel<Length>` (named)
///   The horizontal displacement of the content.
///
/// - dy: `Rel<Length>` (named)
///   The vertical displacement of the content.
///
/// ## Category
/// layout
#[func]
#[capable(Layout)]
#[derive(Debug, Hash)]
pub struct MoveNode {
    /// The offset by which to move the content.
    pub delta: Axes<Rel<Length>>,
    /// The content that should be moved.
    pub body: Content,
}

#[node]
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
        let pod = Regions::one(regions.base(), Axes::splat(false));
        let mut frame = self.body.layout(vt, styles, pod)?.into_frame();
        let delta = self.delta.resolve(styles);
        let delta = delta.zip(regions.base()).map(|(d, s)| d.relative_to(s));
        frame.translate(delta.to_point());
        Ok(Fragment::frame(frame))
    }
}

/// # Rotate
/// Rotate content with affecting layout.
///
/// Rotate an element by a given angle. The layout will act as if the element
/// was not rotated.
///
/// ## Example
/// ```example
/// #stack(
///   dir: ltr,
///   spacing: 1fr,
///   ..range(16)
///     .map(i => rotate(24deg * i)[X]),
/// )
/// ```
///
/// ## Parameters
/// - body: `Content` (positional, required)
///   The content to rotate.
///
/// - angle: `Angle` (named)
///   The amount of rotation.
///
///   ```example
///   #rotate(angle: -1.571rad)[Space!]
///   ```
///
/// ## Category
/// layout
#[func]
#[capable(Layout)]
#[derive(Debug, Hash)]
pub struct RotateNode {
    /// The angle by which to rotate the node.
    pub angle: Angle,
    /// The content that should be rotated.
    pub body: Content,
}

#[node]
impl RotateNode {
    /// The origin of the rotation.
    ///
    /// By default, the origin is the center of the rotated element. If,
    /// however, you wanted the bottom left corner of the rotated element to
    /// stay aligned with the baseline, you would set the origin to `bottom +
    /// left`.
    ///
    /// ```example
    /// #set text(spacing: 8pt)
    /// #let square = square.with(width: 8pt)
    ///
    /// #box(square())
    /// #box(rotate(angle: 30deg, origin: center, square()))
    /// #box(rotate(angle: 30deg, origin: top + left, square()))
    /// #box(rotate(angle: 30deg, origin: bottom + right, square()))
    /// ```
    #[property(resolve)]
    pub const ORIGIN: Axes<Option<GenAlign>> = Axes::default();

    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self {
            angle: args.named_or_find("angle")?.unwrap_or_default(),
            body: args.expect("body")?,
        }
        .pack())
    }
}

impl Layout for RotateNode {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let pod = Regions::one(regions.base(), Axes::splat(false));
        let mut frame = self.body.layout(vt, styles, pod)?.into_frame();
        let origin = styles.get(Self::ORIGIN).unwrap_or(Align::CENTER_HORIZON);
        let Axes { x, y } = origin.zip(frame.size()).map(|(o, s)| o.position(s));
        let ts = Transform::translate(x, y)
            .pre_concat(Transform::rotate(self.angle))
            .pre_concat(Transform::translate(-x, -y));
        frame.transform(ts);
        Ok(Fragment::frame(frame))
    }
}

/// # Scale
/// Scale content without affecting layout.
///
/// The `scale` function allows you to scale and mirror content without
/// affecting the layout.
///
///
/// ## Example
/// ```example
/// #set align(center)
/// #scale(x: -100%)[This is mirrored.]
/// ```
///
/// ## Parameters
/// - body: `Content` (positional, required)
///   The content to scale.
///
/// - x: `Ratio` (named)
///   The horizontal scaling factor.
///
///   The body will be mirrored horizontally if the parameter is negative.
///
/// - y: `Ratio` (named)
///   The vertical scaling factor.
///
///   The body will be mirrored vertically if the parameter is negative.
///
/// ## Category
/// layout
#[func]
#[capable(Layout)]
#[derive(Debug, Hash)]
pub struct ScaleNode {
    /// Scaling factor.
    pub factor: Axes<Ratio>,
    /// The content that should be scaled.
    pub body: Content,
}

#[node]
impl ScaleNode {
    /// The origin of the transformation.
    ///
    /// By default, the origin is the center of the scaled element.
    ///
    /// ```example
    /// A#box(scale(75%)[A])A \
    /// B#box(scale(75%, origin: bottom + left)[B])B
    /// ```
    #[property(resolve)]
    pub const ORIGIN: Axes<Option<GenAlign>> = Axes::default();

    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let all = args.find()?;
        let x = args.named("x")?.or(all).unwrap_or(Ratio::one());
        let y = args.named("y")?.or(all).unwrap_or(Ratio::one());
        Ok(Self {
            factor: Axes::new(x, y),
            body: args.expect("body")?,
        }
        .pack())
    }
}

impl Layout for ScaleNode {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let pod = Regions::one(regions.base(), Axes::splat(false));
        let mut frame = self.body.layout(vt, styles, pod)?.into_frame();
        let origin = styles.get(Self::ORIGIN).unwrap_or(Align::CENTER_HORIZON);
        let Axes { x, y } = origin.zip(frame.size()).map(|(o, s)| o.position(s));
        let transform = Transform::translate(x, y)
            .pre_concat(Transform::scale(self.factor.x, self.factor.y))
            .pre_concat(Transform::translate(-x, -y));
        frame.transform(transform);
        Ok(Fragment::frame(frame))
    }
}
