use typst::geom::Transform;

use crate::prelude::*;

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
/// Display: Move
/// Category: layout
#[node(Layout)]
pub struct MoveNode {
    /// The content to move.
    #[positional]
    #[required]
    pub body: Content,

    /// The horizontal displacement of the content.
    #[named]
    #[default]
    pub dx: Rel<Length>,

    /// The vertical displacement of the content.
    #[named]
    #[default]
    pub dy: Rel<Length>,
}

impl Layout for MoveNode {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let pod = Regions::one(regions.base(), Axes::splat(false));
        let mut frame = self.body().layout(vt, styles, pod)?.into_frame();
        let delta = Axes::new(self.dx(), self.dy()).resolve(styles);
        let delta = delta.zip(regions.base()).map(|(d, s)| d.relative_to(s));
        frame.translate(delta.to_point());
        Ok(Fragment::frame(frame))
    }
}

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
/// Display: Rotate
/// Category: layout
#[node(Layout)]
pub struct RotateNode {
    /// The amount of rotation.
    ///
    /// ```example
    /// #rotate(angle: -1.571rad)[Space!]
    /// ```
    ///
    #[named]
    #[shorthand]
    #[default]
    pub angle: Angle,

    /// The content to rotate.
    #[positional]
    #[required]
    pub body: Content,

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
    #[settable]
    #[resolve]
    #[default]
    pub origin: Axes<Option<GenAlign>>,
}

impl Layout for RotateNode {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let pod = Regions::one(regions.base(), Axes::splat(false));
        let mut frame = self.body().layout(vt, styles, pod)?.into_frame();
        let origin = styles.get(Self::ORIGIN).unwrap_or(Align::CENTER_HORIZON);
        let Axes { x, y } = origin.zip(frame.size()).map(|(o, s)| o.position(s));
        let ts = Transform::translate(x, y)
            .pre_concat(Transform::rotate(self.angle()))
            .pre_concat(Transform::translate(-x, -y));
        frame.transform(ts);
        Ok(Fragment::frame(frame))
    }
}

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
/// Display: Scale
/// Category: layout
#[node(Construct, Layout)]
pub struct ScaleNode {
    /// The content to scale.
    #[positional]
    #[required]
    pub body: Content,

    /// The horizontal scaling factor.
    ///
    /// The body will be mirrored horizontally if the parameter is negative.
    #[named]
    #[default(Ratio::one())]
    pub x: Ratio,

    /// The vertical scaling factor.
    ///
    /// The body will be mirrored vertically if the parameter is negative.
    #[named]
    #[default(Ratio::one())]
    pub y: Ratio,

    /// The origin of the transformation.
    ///
    /// By default, the origin is the center of the scaled element.
    ///
    /// ```example
    /// A#box(scale(75%)[A])A \
    /// B#box(scale(75%, origin: bottom + left)[B])B
    /// ```
    #[settable]
    #[resolve]
    #[default]
    pub origin: Axes<Option<GenAlign>>,
}

impl Construct for ScaleNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let all = args.find()?;
        let x = args.named("x")?.or(all).unwrap_or(Ratio::one());
        let y = args.named("y")?.or(all).unwrap_or(Ratio::one());
        Ok(Self::new(args.expect::<Content>("body")?).with_x(x).with_y(y).pack())
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
        let mut frame = self.body().layout(vt, styles, pod)?.into_frame();
        let origin = styles.get(Self::ORIGIN).unwrap_or(Align::CENTER_HORIZON);
        let Axes { x, y } = origin.zip(frame.size()).map(|(o, s)| o.position(s));
        let transform = Transform::translate(x, y)
            .pre_concat(Transform::scale(self.x(), self.y()))
            .pre_concat(Transform::translate(-x, -y));
        frame.transform(transform);
        Ok(Fragment::frame(frame))
    }
}
