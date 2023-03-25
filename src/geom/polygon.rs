use super::*;

// Produce a shape of a polygon. Needs to have at-least one point.
pub fn polygon(points: Vec<Point>, fill: Option<Paint>, stroke: Option<Stroke>) -> Shape {
    let mut path = Path::new();
    
    path.move_to(points[0]);
    for point in &points[1..] {
        path.line_to(*point);
    }
    path.close_path();

    Shape { geometry: Geometry::Path(path), stroke, fill }
}