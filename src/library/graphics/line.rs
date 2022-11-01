use crate::library::prelude::*;

/// Display a line without affecting the layout.
#[derive(Debug, Hash)]
pub struct LineNode {
    /// Where the line starts.
    origin: Axes<Rel<Length>>,
    /// The offset from the `origin` where the line ends.
    delta: Axes<Rel<Length>>,
}

#[node(Layout)]
impl LineNode {
    /// How to stroke the line.
    #[property(resolve, fold)]
    pub const STROKE: RawStroke = RawStroke::default();

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

impl Layout for LineNode {
    fn layout(
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

    fn level(&self) -> Level {
        Level::Inline
    }
}

castable! {
    Axes<Rel<Length>>,
    Expected: "array of two relative lengths",
    Value::Array(array) => {
        let mut iter = array.into_iter();
        match (iter.next(), iter.next(), iter.next()) {
            (Some(a), Some(b), None) => Axes::new(a.cast()?, b.cast()?),
            _ => Err("point array must contain exactly two entries")?,
        }
    },
}
