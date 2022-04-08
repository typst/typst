use crate::library::prelude::*;

/// Display a line without affecting the layout.
#[derive(Debug, Hash)]
pub struct LineNode {
    /// Where the line starts.
    origin: Spec<Relative<RawLength>>,
    /// The offset from the `origin` where the line ends.
    delta: Spec<Relative<RawLength>>,
}

#[node]
impl LineNode {
    /// How to stroke the line.
    pub const STROKE: Paint = Color::BLACK.into();
    /// The line's thickness.
    #[property(resolve)]
    pub const THICKNESS: RawLength = Length::pt(1.0).into();

    fn construct(_: &mut Context, args: &mut Args) -> TypResult<Content> {
        let origin = args.named("origin")?.unwrap_or_default();

        let delta = match args.named::<Spec<Relative<RawLength>>>("to")? {
            Some(to) => to.zip(origin).map(|(to, from)| to - from),
            None => {
                let length = args
                    .named::<Relative<RawLength>>("length")?
                    .unwrap_or(Length::cm(1.0).into());

                let angle = args.named::<Angle>("angle")?.unwrap_or_default();
                let x = angle.cos() * length;
                let y = angle.sin() * length;

                Spec::new(x, y)
            }
        };

        Ok(Content::inline(Self { origin, delta }))
    }
}

impl Layout for LineNode {
    fn layout(
        &self,
        _: &mut Context,
        regions: &Regions,
        styles: StyleChain,
    ) -> TypResult<Vec<Arc<Frame>>> {
        let thickness = styles.get(Self::THICKNESS);
        let stroke = Some(Stroke {
            paint: styles.get(Self::STROKE),
            thickness,
        });

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

        let geometry = Geometry::Line(delta.to_point());
        let shape = Shape { geometry, fill: None, stroke };

        let target = regions.expand.select(regions.first, Size::zero());
        let mut frame = Frame::new(target);
        frame.push(origin.to_point(), Element::Shape(shape));

        Ok(vec![Arc::new(frame)])
    }
}

castable! {
    Spec<Relative<RawLength>>,
    Expected: "array of two relative lengths",
    Value::Array(array) => {
        let mut iter = array.into_iter();
        match (iter.next(), iter.next(), iter.next()) {
            (Some(a), Some(b), None) => Spec::new(a.cast()?, b.cast()?),
            _ => Err("point array must contain exactly two entries")?,
        }
    },
}
