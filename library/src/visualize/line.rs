use crate::prelude::*;

/// # Line
/// Display a line without affecting the layout.
///
/// You should only provide either an endpoint or an angle and a length.
///
/// ## Parameters
/// - origin: Axes<Rel<Length>> (named)
///   The start point of the line.
///
/// - to: Axes<Rel<Length>> (named)
///   The end point of the line.
///
/// - length: Rel<Length> (named)
///   The line's length.
///
/// - angle: Angle (named)
///   The angle at which the line points away from the origin.
///
/// ## Category
/// visualize
#[func]
#[capable(Layout, Inline)]
#[derive(Debug, Hash)]
pub struct LineNode {
    /// Where the line starts.
    pub origin: Axes<Rel<Length>>,
    /// The offset from the `origin` where the line ends.
    pub delta: Axes<Rel<Length>>,
}

#[node]
impl LineNode {
    /// How to stroke the line.
    #[property(resolve, fold)]
    pub const STROKE: PartialStroke = PartialStroke::default();

    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let origin = args.named("origin")?.unwrap_or_default();

        let delta = match args.named::<Axes<Rel<Length>>>("to")? {
            Some(to) => to.zip(origin).map(|(to, from)| to - from),
            None => {
                let length =
                    args.named::<Rel<Length>>("length")?.unwrap_or(Abs::cm(1.0).into());

                let angle = args.named::<Angle>("angle")?.unwrap_or_default();
                let x = angle.cos() * length;
                let y = angle.sin() * length;

                Axes::new(x, y)
            }
        };

        Ok(Self { origin, delta }.pack())
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
            .origin
            .resolve(styles)
            .zip(regions.base)
            .map(|(l, b)| l.relative_to(b));

        let delta = self
            .delta
            .resolve(styles)
            .zip(regions.base)
            .map(|(l, b)| l.relative_to(b));

        let target = regions.expand.select(regions.first, Size::zero());

        let mut frame = Frame::new(target);
        let shape = Geometry::Line(delta.to_point()).stroked(stroke);
        frame.push(origin.to_point(), Element::Shape(shape));

        Ok(Fragment::frame(frame))
    }
}

impl Inline for LineNode {}
