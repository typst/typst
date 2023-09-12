use super::*;

/// Produce a shape that approximates an axis-aligned ellipse.
pub fn ellipse(size: Size, fill: Option<Paint>, stroke: Option<FixedStroke>) -> Shape {
    // https://stackoverflow.com/a/2007782
    let z = Abs::zero();
    let rx = size.x / 2.0;
    let ry = size.y / 2.0;
    let m = 0.551784;
    let mx = m * rx;
    let my = m * ry;
    let point = |x, y| Point::new(x + rx, y + ry);

    let mut path = Path::new();
    path.move_to(point(-rx, z));
    path.cubic_to(point(-rx, -my), point(-mx, -ry), point(z, -ry));
    path.cubic_to(point(mx, -ry), point(rx, -my), point(rx, z));
    path.cubic_to(point(rx, my), point(mx, ry), point(z, ry));
    path.cubic_to(point(-mx, ry), point(-rx, my), point(-rx, z));

    Shape { geometry: Geometry::Path(path), stroke, fill }
}
