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
    /// - A stroke described by a dict with any of the following keys: 
    ///     - `color`: the color to use for the stroke
    ///     - `thickness`: the stroke's thickness
    ///     - `line_cap`: one of `"butt"`, `"round"` or `"square"`, the line cap of the stroke
    ///     - `line_join`: one of `"miter"`, `"round"` or `"bevel"`, the line join of the stroke
    ///     - `dash`: the dash pattern to use. Can be any of the following:
    ///         - One of the strings `"solid"`, `"dotted"`, `"densely dotted"`, `"loosely dotted"`,
    ///           `"dashed"`, `"densely dashed"`, `"loosely dashed"`, `"dashdotted"`, 
    ///           `"densely dashdotted"` or `"loosely dashdotted"`
    ///         - An array with elements that specify the lengths of dashes and gaps, alternating.
    ///           Elements can also be the string `"dot"` for a length equal to the line thickness.
    ///         - A dict with the keys `array`, same as the array above, and `phase`, the offset to
    ///           the start of the first dash.
    ///     
    ///
    /// ```example
    /// #stack(
    ///   line(length: 100%, stroke: 2pt + red),
    ///   v(1em),
    ///   line(length: 100%, stroke: (color: blue, thickness: 4pt, line_cap: "round")),
    ///   v(1em),
    ///   line(length: 100%, stroke: (color: blue, thickness: 1pt, dash: "dashed")),
    ///   v(1em),
    ///   line(length: 100%, stroke: (color: blue, thickness: 1pt, dash: ("dot", 2pt, 4pt, 2pt))),
    /// )
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
