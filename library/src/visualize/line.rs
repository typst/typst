use crate::prelude::*;

/// A line from one point to another.
///
/// ## Example
/// ```example
/// #set page(height: 100pt)
/// #line(length: 100%)
/// #line(end: (50%, 50%))
/// ```
///
/// Display: Line
/// Category: visualize
#[element(Layout)]
pub struct LineElem {
    /// The start point of the line.
    ///
    /// Must be an array of exactly two relative lengths.
    #[resolve]
    pub start: Axes<Rel<Length>>,

    /// The offset from `start` where the line ends.
    #[resolve]
    pub end: Option<Axes<Rel<Length>>>,

    /// The line's length. This is only respected if `end` is `none`.
    #[resolve]
    #[default(Abs::pt(30.0).into())]
    pub length: Rel<Length>,

    /// The angle at which the line points away from the origin. This is only
    /// respected if `end` is `none`.
    pub angle: Angle,

    /// How to stroke the line. This can be:
    ///
    /// - A length specifying the stroke's thickness. The color is inherited,
    ///   defaulting to black.
    /// - A color to use for the stroke. The thickness is inherited, defaulting
    ///   to `{1pt}`.
    /// - A stroke combined from color and thickness using the `+` operator as
    ///   in `{2pt + red}`.
    ///
    /// ```example
    /// #line(length: 100%, stroke: 2pt + red)
    /// ```
    #[resolve]
    #[fold]
    pub stroke: PartialStroke,
}

impl Layout for LineElem {
    fn layout(
        &self,
        _: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let resolve = |axes: Axes<Rel<Abs>>| {
            axes.zip(regions.base()).map(|(l, b)| l.relative_to(b))
        };

        let start = resolve(self.start(styles));
        let delta =
            self.end(styles).map(|end| resolve(end) - start).unwrap_or_else(|| {
                let length = self.length(styles);
                let angle = self.angle(styles);
                let x = angle.cos() * length;
                let y = angle.sin() * length;
                resolve(Axes::new(x, y))
            });

        let stroke = self.stroke(styles).unwrap_or_default();
        let size = start.max(start + delta).max(Size::zero());
        let target = regions.expand.select(regions.size, size);

        let mut frame = Frame::new(target);
        let shape = Geometry::Line(delta.to_point()).stroked(stroke);
        frame.push(start.to_point(), FrameItem::Shape(shape, self.span()));
        Ok(Fragment::frame(frame))
    }
}

/// A cubic spline (bezier curve) from one point to another.
///
/// ## Example
/// ```example
/// #set page(height: 100pt)
/// #bezier(end: (50%, 50%), start-control-point: (10%, 10%), end-control-point: (60%, 60%))
/// ```
///
/// Display: Bezier
/// Category: visualize
#[element(Layout)]
pub struct BezierCurve {
    /// The start point of the curve.
    ///
    /// Must be an array of exactly two relative lengths.
    #[resolve]
    pub start: Axes<Rel<Length>>,


    /// The end point of the curve.
    ///
    /// Must be an array of exactly two relative lengths.
    #[resolve]
    pub end: Axes<Rel<Length>>,

    /// The first control point, associated to the start point. This is an offset to the start point, and therefore defaults to it when not specified.
    ///
    /// Must be an array of exactly two relative lengths.
    #[resolve]
    pub start_control_point: Axes<Rel<Length>>,

    /// The second control point, associated to the end point. This is an offset to the end point, and therefore defaults to it when not specified.
    ///
    /// Must be an array of exactly two relative lengths.
    #[resolve]
    pub end_control_point: Axes<Rel<Length>>,


    /// How to stroke the curve. This can be:
    ///
    /// - A length specifying the stroke's thickness. The color is inherited,
    ///   defaulting to black.
    /// - A color to use for the stroke. The thickness is inherited, defaulting
    ///   to `{1pt}`.
    /// - A stroke combined from color and thickness using the `+` operator as
    ///   in `{2pt + red}`.
    ///
    /// ```example
    /// #bezier(end: (100%, 100%), stroke: 2pt + red)
    /// ```
    #[resolve]
    #[fold]
    pub stroke: PartialStroke,
}

impl Layout for BezierCurve {
    fn layout(
        &self,
        _: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let resolve = |axes: Axes<Rel<Abs>>| {
            axes.zip(regions.base()).map(|(l, b)| l.relative_to(b))
        };

        let start = resolve(self.start(styles));
        let delta = resolve(self.end(styles)) - start;

        let start_control_point = resolve(self.start_control_point(styles)).to_point();
        let end_control_point = resolve(self.end_control_point(styles)).to_point() + delta.to_point();

        let stroke = self.stroke(styles).unwrap_or_default();
        let size = start.max(delta + start).max(Size::zero());
        let target = regions.expand.select(regions.size, size);

        let mut frame = Frame::new(target);

        let mut path = Path::new();
        path.move_to(Point::zero());
        path.cubic_to(start_control_point, end_control_point, delta.to_point());
        let shape = Geometry::Path(path).stroked(stroke);
        frame.push(start.to_point(), FrameItem::Shape(shape, self.span()));
        Ok(Fragment::frame(frame))
    }
}