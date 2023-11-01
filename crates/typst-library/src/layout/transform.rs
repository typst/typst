use typst::geom::Transform;

use crate::prelude::*;

/// Moves content without affecting layout.
///
/// The `move` function allows you to move content while the layout still 'sees'
/// it at the original positions. Containers will still be sized as if the
/// content was not moved, unless you specify `{layout: true}` in which case
/// the rest of the layout will be adjusted to account for the movement.
///
/// # Example
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
#[elem(Layout)]
pub struct MoveElem {
    /// The horizontal displacement of the content.
    pub dx: Rel<Length>,

    /// The vertical displacement of the content.
    pub dy: Rel<Length>,

    /// Whether the movement impacts the layout.
    #[default(false)]
    pub layout: bool,

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
        let delta = delta.zip_map(regions.base(), Rel::relative_to);
        frame.translate(delta.to_point());

        if !self.layout(styles) {
            return Ok(Fragment::frame(frame));
        }

        // If we impact layout we wrap into a frame of the correct size
        let ts = Transform::translate(delta.x, delta.y);
        let (_, size) = compute_bounding_box(&frame, ts);
        let mut out: Frame = Frame::soft(size);
        out.push(Point::zero(), FrameItem::Group(GroupItem::new(frame)));
        Ok(Fragment::frame(out))
    }
}

/// Rotates content without affecting layout.
///
/// Rotates an element by a given angle. The layout will act as if the element
/// was not rotated unless you specify `{layout: true}`.
///
/// # Example
/// ```example
/// #stack(
///   dir: ltr,
///   spacing: 1fr,
///   ..range(16)
///     .map(i => rotate(24deg * i)[X]),
/// )
/// ```
#[elem(Layout)]
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
    #[fold]
    #[default(HAlign::Center + VAlign::Horizon)]
    pub origin: Align,

    /// Whether the rotation impacts the layout.
    ///
    /// If set to `{false}`, the rotated content will be allowed to overlap
    /// other content. However, when set to `{true}`, it will be compute the
    /// new size of the rotated content and adjust the layout accordingly.
    ///
    /// ```example
    /// #let rotated(body) = rotate(90deg, layout: true, body)
    ///
    /// Hello #rotated[World]!
    /// ```
    #[default(false)]
    pub layout: bool,

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
        let align = self.origin(styles).resolve(styles);
        let Axes { x, y } = align.zip_map(frame.size(), FixedAlign::position);

        let ts = Transform::translate(x, y)
            .pre_concat(Transform::rotate(self.angle(styles)))
            .pre_concat(Transform::translate(-x, -y));
        frame.transform(ts);

        // If we don't impact layout we exit early.
        if !self.layout(styles) {
            return Ok(Fragment::frame(frame));
        }

        // If we impact layout we wrap into a frame of the correct size
        let (offset, size) = compute_bounding_box(&frame, ts);
        let mut out = Frame::soft(size);
        out.push(offset, FrameItem::Group(GroupItem::new(frame)));
        Ok(Fragment::frame(out))
    }
}

/// Scales content without affecting layout.
///
/// Lets you mirror content by specifying a negative scale on a single axis.
///
/// # Example
/// ```example
/// #set align(center)
/// #scale(x: -100%)[This is mirrored.]
/// ```
#[elem(Layout)]
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
    #[fold]
    #[default(HAlign::Center + VAlign::Horizon)]
    pub origin: Align,

    /// Whether the scaling impacts the layout.
    ///
    /// If set to `{false}`, the scaled content will be allowed to overlap
    /// other content. However, when set to `{true}`, it will be compute the
    /// new size of the scaled content and adjust the layout accordingly.
    ///
    /// ```example
    /// #let scaled(body) = scale(x: 20%, y: 40%, layout: true, body)
    ///
    /// Hello #scaled[World]!
    /// ```
    #[default(false)]
    pub layout: bool,

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
        let Axes { x, y } = self
            .origin(styles)
            .resolve(styles)
            .zip_map(frame.size(), FixedAlign::position);
        let ts = Transform::translate(x, y)
            .pre_concat(Transform::scale(self.x(styles), self.y(styles)))
            .pre_concat(Transform::translate(-x, -y));
        frame.transform(ts);

        // If we don't impact layout we exit early.
        if !self.layout(styles) {
            return Ok(Fragment::frame(frame));
        }

        // If we impact layout we wrap into a frame of the correct size
        let (offset, size) = compute_bounding_box(&frame, ts);
        let mut out = Frame::soft(size);
        out.push(offset, FrameItem::Group(GroupItem::new(frame)));
        Ok(Fragment::frame(out))
    }
}

/// Computes the bounding box and offset of a transformed frame.
fn compute_bounding_box(frame: &Frame, ts: Transform) -> (Point, Size) {
    let top_left = ts.transform_point(Point::zero());
    let top_right = ts.transform_point(Point::new(frame.width(), Abs::zero()));
    let bottom_left = ts.transform_point(Point::new(Abs::zero(), frame.height()));
    let bottom_right = ts.transform_point(Point::new(frame.width(), frame.height()));

    // We first compute the new bounding box of the rotated frame.
    let min_x = top_left.x.min(top_right.x).min(bottom_left.x).min(bottom_right.x);
    let min_y = top_left.y.min(top_right.y).min(bottom_left.y).min(bottom_right.y);
    let max_x = top_left.x.max(top_right.x).max(bottom_left.x).max(bottom_right.x);
    let max_y = top_left.y.max(top_right.y).max(bottom_left.y).max(bottom_right.y);

    // Then we compute the new size of the frame.
    let width = max_x - min_x;
    let height = max_y - min_y;

    (Point::new(-min_x, -min_y), Size::new(width, height))
}
