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
/// ## Parameters
/// - end: `Axes<Rel<Length>>` (named)
///   The end point of the line.
///   Must be an array of exactly two relative lengths.
///
/// - length: `Rel<Length>` (named)
///   The line's length. Mutually exclusive with `end`.
///
/// - angle: `Angle` (named)
///   The angle at which the line points away from the origin. Mutually
///   exclusive with `end`.
///
/// Display: Line
/// Category: visualize
#[node(Construct, Layout)]
pub struct LineNode {
    /// The start point of the line.
    ///
    /// Must be an array of exactly two relative lengths.
    #[named]
    #[default]
    pub start: Axes<Rel<Length>>,

    /// The offset from `start` where the line ends.
    #[named]
    #[default]
    #[skip]
    pub delta: Axes<Rel<Length>>,

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
    #[settable]
    #[resolve]
    #[fold]
    #[default]
    pub stroke: PartialStroke,
}

impl Construct for LineNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let start = args.named("start")?.unwrap_or_default();
        let delta = match args.named::<Axes<Rel<Length>>>("end")? {
            Some(end) => end.zip(start).map(|(to, from)| to - from),
            None => {
                let length =
                    args.named::<Rel<Length>>("length")?.unwrap_or(Abs::pt(30.0).into());

                let angle = args.named::<Angle>("angle")?.unwrap_or_default();
                let x = angle.cos() * length;
                let y = angle.sin() * length;

                Axes::new(x, y)
            }
        };
        Ok(Self::new().with_start(start).with_delta(delta).pack())
    }
}

impl Layout for LineNode {
    fn layout(
        &self,
        _: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let stroke = styles.get(Self::STROKE).unwrap_or_default();

        let origin = self
            .start()
            .resolve(styles)
            .zip(regions.base())
            .map(|(l, b)| l.relative_to(b));

        let delta = self
            .delta()
            .resolve(styles)
            .zip(regions.base())
            .map(|(l, b)| l.relative_to(b));

        let size = origin.max(origin + delta).max(Size::zero());
        let target = regions.expand.select(regions.size, size);

        let mut frame = Frame::new(target);
        let shape = Geometry::Line(delta.to_point()).stroked(stroke);
        frame.push(origin.to_point(), Element::Shape(shape));

        Ok(Fragment::frame(frame))
    }
}
