use typst::geom::Transform;

use crate::prelude::*;

/// Moves content without affecting layout.
///
/// The `move` function allows you to move content while the layout still 'sees'
/// it at the original positions. Containers will still be sized as if the
/// content was not moved.
///
/// ## Example { #example }
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
#[element(Layout)]
pub struct MoveElem {
    /// The horizontal displacement of the content.
    pub dx: Rel<Length>,

    /// The vertical displacement of the content.
    pub dy: Rel<Length>,

    /// The content to move.
    #[required]
    pub body: Content,
}

impl Layout for MoveElem {
    #[tracing::instrument(name = "MoveElem::layout", skip_all)]
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let pod = Regions::one(regions.base(), Axes::splat(false));
        let mut frame = self.body().layout(vt, styles, pod)?.into_frame();
        let delta = Axes::new(self.dx(styles), self.dy(styles)).resolve(styles);
        let delta = delta.zip(regions.base()).map(|(d, s)| d.relative_to(s));
        frame.translate(delta.to_point());
        Ok(Fragment::frame(frame))
    }
}

/// Rotates content without affecting layout.
///
/// Rotates an element by a given angle. The layout will act as if the element
/// was not rotated.
///
/// ## Example { #example }
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
#[element(Layout)]
pub struct RotateElem {
    /// The amount of rotation.
    ///
    /// ```example
    /// #rotate(-1.571rad)[Space!]
    /// ```
    ///
    #[positional]
    pub angle: Angle,

    /// The origin of the rotation.
    ///
    /// If, for instance, you wanted the bottom left corner of the rotated
    /// element to stay aligned with the baseline, you would set it to `bottom +
    /// left` instead.
    ///
    /// ```example
    /// #set text(spacing: 8pt)
    /// #let square = square.with(width: 8pt)
    ///
    /// #box(square())
    /// #box(rotate(30deg, origin: center, square()))
    /// #box(rotate(30deg, origin: top + left, square()))
    /// #box(rotate(30deg, origin: bottom + right, square()))
    /// ```
    #[resolve]
    #[fold]
    #[default(Align::CENTER_HORIZON)]
    pub origin: Axes<Option<GenAlign>>,

    /// The content to rotate.
    #[required]
    pub body: Content,
}

impl Layout for RotateElem {
    #[tracing::instrument(name = "RotateElem::layout", skip_all)]
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let pod = Regions::one(regions.base(), Axes::splat(false));
        let mut frame = self.body().layout(vt, styles, pod)?.into_frame();
        let Axes { x, y } =
            self.origin(styles).zip(frame.size()).map(|(o, s)| o.position(s));
        let ts = Transform::translate(x, y)
            .pre_concat(Transform::rotate(self.angle(styles)))
            .pre_concat(Transform::translate(-x, -y));
        frame.transform(ts);
        Ok(Fragment::frame(frame))
    }
}

/// Scales content without affecting layout.
///
/// Lets you mirror content by specifying a negative scale on a single axis.
///
/// ## Example { #example }
/// ```example
/// #set align(center)
/// #scale(x: -100%)[This is mirrored.]
/// ```
///
/// Display: Scale
/// Category: layout
#[element(Layout)]
pub struct ScaleElem {
    /// The horizontal scaling factor.
    ///
    /// The body will be mirrored horizontally if the parameter is negative.
    #[parse(
        let all = args.find()?;
        args.named("x")?.or(all)
    )]
    #[default(Ratio::one())]
    pub x: Ratio,

    /// The vertical scaling factor.
    ///
    /// The body will be mirrored vertically if the parameter is negative.
    #[parse(args.named("y")?.or(all))]
    #[default(Ratio::one())]
    pub y: Ratio,

    /// The origin of the transformation.
    ///
    /// ```example
    /// A#box(scale(75%)[A])A \
    /// B#box(scale(75%, origin: bottom + left)[B])B
    /// ```
    #[resolve]
    #[fold]
    #[default(Align::CENTER_HORIZON)]
    pub origin: Axes<Option<GenAlign>>,

    /// The content to scale.
    #[required]
    pub body: Content,
}

impl Layout for ScaleElem {
    #[tracing::instrument(name = "ScaleElem::layout", skip_all)]
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let pod = Regions::one(regions.base(), Axes::splat(false));
        let mut frame = self.body().layout(vt, styles, pod)?.into_frame();
        let Axes { x, y } =
            self.origin(styles).zip(frame.size()).map(|(o, s)| o.position(s));
        let transform = Transform::translate(x, y)
            .pre_concat(Transform::scale(self.x(styles), self.y(styles)))
            .pre_concat(Transform::translate(-x, -y));
        frame.transform(transform);
        Ok(Fragment::frame(frame))
    }
}
