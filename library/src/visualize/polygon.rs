use crate::prelude::*;

/// A closed-path polygon.
///
/// ## Example
/// ```example
/// #polygon(fill: blue, (0pt, 0pt), (10pt, 0pt), (10pt, 10pt))
/// ```
///
/// Display: Polygon
/// Category: visualize
#[element(Layout)]
pub struct PolygonElem {
    /// How to fill the polygon. See the
    /// [rectangle's documentation]($func/rect.fill) for more details.
    pub fill: Option<Paint>,

    /// How to stroke the polygon. See the [lines's
    /// documentation]($func/line.stroke) for more details.
    #[resolve]
    #[fold]
    pub stroke: Option<PartialStroke>,

    /// The vertices of the polygon. The polygon automatically closes itself.
    #[variadic]
    pub vertices: Vec<Axes<Rel<Length>>>,
}

impl Layout for PolygonElem {
    fn layout(
        &self,
        _: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let points: Vec<Point> = self
            .vertices()
            .iter()
            .map(|c| {
                c.resolve(styles)
                    .zip(regions.base())
                    .map(|(l, b)| l.relative_to(b))
                    .to_point()
            })
            .collect();

        let size = points.iter().fold(Point::zero(), |max, c| c.max(max)).to_size();

        let target = regions.expand.select(regions.size, size);
        let mut frame = Frame::new(target);

        // only create a path if there is more than zero points.
        if points.len() > 0 {
            let stroke = self.stroke(styles).map(|e| e.unwrap_or_default());
            let fill = self.fill(styles);

            // construct a closed path given all points.
            let mut path = Path::new();
            path.move_to(points[0]);
            for point in &points[1..] {
                path.line_to(*point);
            }
            path.close_path();

            let shape = Shape { geometry: Geometry::Path(path), stroke, fill };
            frame.push(Point::zero(), FrameItem::Shape(shape, self.span()));
        }

        Ok(Fragment::frame(frame))
    }
}
