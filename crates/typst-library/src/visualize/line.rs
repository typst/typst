use crate::prelude::*;

/// A line from one point to another.
///
/// ## Example { #example }
/// ```example
/// #set page(height: 100pt)
///
/// #line(length: 100%)
/// #line(end: (50%, 50%))
/// #line(
///   length: 4cm,
///   stroke: 2pt + maroon,
/// )
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
    /// - A stroke described by a dictionary with any of the following keys:
    ///   - `paint`: The [color]($type/color) to use for the stroke.
    ///   - `thickness`: The stroke's thickness as a [length]($type/length).
    ///   - `cap`: How the line terminates. One of `{"butt"}`, `{"round"}`, or
    ///     `{"square"}`.
    ///   - `join`: How sharp turns of a contour are rendered. One of
    ///     `{"miter"}`, `{"round"}`, or `{"bevel"}`. Not applicable to lines
    ///     but to [polygons]($func/polygon) or [paths]($func/path).
    ///   - `miter-limit`: Number at which protruding sharp angles are rendered
    ///     with a bevel instead. The higher the number, the sharper an angle
    ///     can be before it is bevelled. Only applicable if `join` is
    ///     `{"miter"}`. Defaults to `{4.0}`.
    ///   - `dash`: The dash pattern to use. Can be any of the following:
    ///     - One of the predefined patterns `{"solid"}`, `{"dotted"}`,
    ///       `{"densely-dotted"}`, `{"loosely-dotted"}`, `{"dashed"}`,
    ///       `{"densely-dashed"}`, `{"loosely-dashed"}`, `{"dash-dotted"}`,
    ///       `{"densely-dash-dotted"}` or `{"loosely-dash-dotted"}`
    ///     - An [array]($type/array) with alternating lengths for dashes and
    ///       gaps. You can also use the string `{"dot"}` for a length equal to
    ///       the line thickness.
    ///     - A [dictionary]($type/dictionary) with the keys `array` (same as
    ///       the array above), and `phase` (of type [length]($type/length)),
    ///       which defines where in the pattern to start drawing.
    ///
    /// ```example
    /// #set line(length: 100%)
    /// #stack(
    ///   spacing: 1em,
    ///   line(stroke: 2pt + red),
    ///   line(stroke: (paint: blue, thickness: 4pt, cap: "round")),
    ///   line(stroke: (paint: blue, thickness: 1pt, dash: "dashed")),
    ///   line(stroke: (paint: blue, thickness: 1pt, dash: ("dot", 2pt, 4pt, 2pt))),
    /// )
    /// ```
    #[resolve]
    #[fold]
    pub stroke: PartialStroke,
}

impl Layout for LineElem {
    #[tracing::instrument(name = "LineElem::layout", skip_all)]
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
