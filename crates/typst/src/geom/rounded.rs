use super::*;

/// Produce shapes that together make up a rounded rectangle.
pub fn rounded_rect(
    size: Size,
    radius: Corners<Abs>,
    fill: Option<Paint>,
    stroke: Sides<Option<Stroke>>,
) -> Vec<Shape> {
    let mut res = vec![];
    if fill.is_some() || (stroke.iter().any(Option::is_some) && stroke.is_uniform()) {
        res.push(Shape {
            geometry: fill_geometry(size, radius),
            fill,
            stroke: if stroke.is_uniform() { stroke.top.clone() } else { None },
        });
    }

    if !stroke.is_uniform() {
        for (path, stroke) in stroke_segments(size, radius, stroke) {
            if stroke.is_some() {
                res.push(Shape { geometry: Geometry::Path(path), fill: None, stroke });
            }
        }
    }

    res
}

/// Output the shape of the rectangle as a path or primitive rectangle,
/// depending on whether it is rounded.
fn fill_geometry(size: Size, radius: Corners<Abs>) -> Geometry {
    if radius.iter().copied().all(Abs::is_zero) {
        Geometry::Rect(size)
    } else {
        let mut paths = stroke_segments(size, radius, Sides::splat(None));
        assert_eq!(paths.len(), 1);
        Geometry::Path(paths.pop().unwrap().0)
    }
}

/// Output the minimum number of paths along the rectangles border.
fn stroke_segments(
    size: Size,
    radius: Corners<Abs>,
    stroke: Sides<Option<Stroke>>,
) -> Vec<(Path, Option<Stroke>)> {
    let mut res = vec![];

    let mut connection = Connection::default();
    let mut path = Path::new();
    let mut always_continuous = true;
    let max_radius = size.x.min(size.y).max(Abs::zero()) / 2.0;

    for side in [Side::Top, Side::Right, Side::Bottom, Side::Left] {
        let continuous = stroke.get_ref(side) == stroke.get_ref(side.next_cw());
        connection = connection.advance(continuous && side != Side::Left);
        always_continuous &= continuous;

        draw_side(
            &mut path,
            side,
            size,
            radius.get(side.start_corner()).clamp(Abs::zero(), max_radius),
            radius.get(side.end_corner()).clamp(Abs::zero(), max_radius),
            connection,
        );

        if !continuous {
            res.push((std::mem::take(&mut path), stroke.get_ref(side).clone()));
        }
    }

    if always_continuous {
        path.close_path();
    }

    if !path.0.is_empty() {
        res.push((path, stroke.left));
    }

    res
}

/// Draws one side of the rounded rectangle. Will always draw the left arc. The
/// right arc will be drawn halfway if and only if there is no connection.
fn draw_side(
    path: &mut Path,
    side: Side,
    size: Size,
    start_radius: Abs,
    end_radius: Abs,
    connection: Connection,
) {
    let angle_left = Angle::deg(if connection.prev { 90.0 } else { 45.0 });
    let angle_right = Angle::deg(if connection.next { 90.0 } else { 45.0 });
    let length = size.get(side.axis());

    // The arcs for a border of the rectangle along the x-axis, starting at (0,0).
    let p1 = Point::with_x(start_radius);
    let mut arc1 = bezier_arc(
        p1 + Point::new(
            -angle_left.sin() * start_radius,
            (1.0 - angle_left.cos()) * start_radius,
        ),
        Point::new(start_radius, start_radius),
        p1,
    );

    let p2 = Point::with_x(length - end_radius);
    let mut arc2 = bezier_arc(
        p2,
        Point::new(length - end_radius, end_radius),
        p2 + Point::new(
            angle_right.sin() * end_radius,
            (1.0 - angle_right.cos()) * end_radius,
        ),
    );

    let transform = match side {
        Side::Left => Transform::rotate(Angle::deg(-90.0))
            .post_concat(Transform::translate(Abs::zero(), size.y)),
        Side::Bottom => Transform::rotate(Angle::deg(180.0))
            .post_concat(Transform::translate(size.x, size.y)),
        Side::Right => Transform::rotate(Angle::deg(90.0))
            .post_concat(Transform::translate(size.x, Abs::zero())),
        _ => Transform::identity(),
    };

    arc1 = arc1.map(|x| x.transform(transform));
    arc2 = arc2.map(|x| x.transform(transform));

    if !connection.prev {
        path.move_to(if start_radius.is_zero() { arc1[3] } else { arc1[0] });
    }

    if !start_radius.is_zero() {
        path.cubic_to(arc1[1], arc1[2], arc1[3]);
    }

    path.line_to(arc2[0]);

    if !connection.next && !end_radius.is_zero() {
        path.cubic_to(arc2[1], arc2[2], arc2[3]);
    }
}

/// Get the control points for a bezier curve that describes a circular arc for
/// a start point, an end point and a center of the circle whose arc connects
/// the two.
fn bezier_arc(start: Point, center: Point, end: Point) -> [Point; 4] {
    // https://stackoverflow.com/a/44829356/1567835
    let a = start - center;
    let b = end - center;

    let q1 = a.x.to_raw() * a.x.to_raw() + a.y.to_raw() * a.y.to_raw();
    let q2 = q1 + a.x.to_raw() * b.x.to_raw() + a.y.to_raw() * b.y.to_raw();
    let k2 = (4.0 / 3.0) * ((2.0 * q1 * q2).sqrt() - q2)
        / (a.x.to_raw() * b.y.to_raw() - a.y.to_raw() * b.x.to_raw());

    let control_1 = Point::new(center.x + a.x - k2 * a.y, center.y + a.y + k2 * a.x);
    let control_2 = Point::new(center.x + b.x + k2 * b.y, center.y + b.y - k2 * b.x);

    [start, control_1, control_2, end]
}

/// Indicates which sides of the border strokes in a 2D polygon are connected to
/// their neighboring sides.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
struct Connection {
    prev: bool,
    next: bool,
}

impl Connection {
    /// Advance to the next clockwise side of the polygon. The argument
    /// indicates whether the border is connected on the right side of the next
    /// edge.
    pub fn advance(self, next: bool) -> Self {
        Self { prev: self.next, next }
    }
}
