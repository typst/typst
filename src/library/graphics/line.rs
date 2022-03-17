use crate::library::prelude::*;

/// Display a line without affecting the layout.
#[derive(Debug, Hash)]
pub struct LineNode {
    origin: Spec<Linear>,
    delta: Spec<Linear>,
}

#[node]
impl LineNode {
    /// How the stroke the line.
    pub const STROKE: Paint = Color::BLACK.into();
    /// The line's thickness.
    pub const THICKNESS: Length = Length::pt(1.0);

    fn construct(_: &mut Context, args: &mut Args) -> TypResult<Content> {
        let origin = args.named::<Spec<Linear>>("origin")?.unwrap_or_default();
        let delta = match args.named::<Spec<Linear>>("to")? {
            Some(to) => to.zip(origin).map(|(to, from)| to - from),
            None => {
                let length =
                    args.named::<Linear>("length")?.unwrap_or(Length::cm(1.0).into());
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

        let resolved_origin =
            self.origin.zip(regions.base).map(|(l, b)| Linear::resolve(l, b));
        let resolved_delta =
            self.delta.zip(regions.base).map(|(l, b)| Linear::resolve(l, b));

        let geometry = Geometry::Line(resolved_delta.to_point());
        let shape = Shape { geometry, fill: None, stroke };

        let target = regions.expand.select(regions.first, Size::zero());
        let mut frame = Frame::new(target);
        frame.push(resolved_origin.to_point(), Element::Shape(shape));

        Ok(vec![Arc::new(frame)])
    }
}
