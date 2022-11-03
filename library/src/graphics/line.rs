use crate::prelude::*;

/// Display a line without affecting the layout.
#[derive(Debug, Hash)]
pub struct LineNode {
    /// Where the line starts.
    origin: Axes<Rel<Length>>,
    /// The offset from the `origin` where the line ends.
    delta: Axes<Rel<Length>>,
}

#[node(LayoutInline)]
impl LineNode {
    /// How to stroke the line.
    #[property(resolve, fold)]
    pub const STROKE: PartialStroke = PartialStroke::default();

    fn construct(_: &mut Vm, args: &mut Args) -> SourceResult<Content> {
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

impl LayoutInline for LineNode {
    fn layout_inline(
        &self,
        _: Tracked<dyn World>,
        regions: &Regions,
        styles: StyleChain,
    ) -> SourceResult<Vec<Frame>> {
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

        Ok(vec![frame])
    }
}
