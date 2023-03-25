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

    /// The points of the polygon.
    /// A polygon needs to have atleast one point.
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
        // If there are no points in a polygon, we dont need to layout it.
        if self.vertices().len() == 0 {
            let target = regions.expand.select(regions.size, Size::zero());
            let frame = Frame::new(target);
            return Ok(Fragment::frame(frame));
        }

        let resolve_rel = |axes: Axes<Rel<Abs>>|
            axes.zip(regions.base()).map(|(l, b)| l.relative_to(b));
        
        let points: Vec<Point> = self.vertices().iter().map(
            |c| resolve_rel(c.resolve(styles)).to_point()).collect();
        let origin = Point::zero();

        let size = points.iter().fold(Point::zero(),
            |max, c| c.max(max)).to_size();

        let stroke = self.stroke(styles).map(|e| e.unwrap_or_default());

        let target = regions.expand.select(regions.size, size);
        let mut frame = Frame::new(target);

        let shape = polygon(points, self.fill(styles), stroke);
        frame.prepend(origin, FrameItem::Shape(shape, self.span()));
        Ok(Fragment::frame(frame))
    }
}
