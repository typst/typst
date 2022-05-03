use super::*;

use std::mem;

/// A rectangle with rounded corners.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Rect {
    size: Size,
    radius: Sides<Length>,
}

impl Rect {
    /// Create a new rectangle.
    pub fn new(size: Size, radius: Sides<Length>) -> Self {
        Self { size, radius }
    }

    /// Output all constituent shapes of the rectangle in order. The last one is
    /// in the foreground. The function will output multiple items if the stroke
    /// properties differ by side.
    pub fn shapes(
        self,
        fill: Option<Paint>,
        stroke: Sides<Option<Stroke>>,
    ) -> Vec<Shape> {
        let mut res = vec![];
        if fill.is_some() || (stroke.iter().any(Option::is_some) && stroke.is_uniform()) {
            res.push(Shape {
                geometry: self.fill_geometry(),
                fill,
                stroke: if stroke.is_uniform() { stroke.top } else { None },
            });
        }

        if !stroke.is_uniform() {
            for (path, stroke) in self.stroke_segments(stroke) {
                if stroke.is_some() {
                    res.push(Shape {
                        geometry: Geometry::Path(path),
                        fill: None,
                        stroke,
                    });
                }
            }
        }

        res
    }

    /// Output the shape of the rectangle as a path or primitive rectangle,
    /// depending on whether it is rounded.
    fn fill_geometry(self) -> Geometry {
        if self.radius.iter().copied().all(Length::is_zero) {
            Geometry::Rect(self.size)
        } else {
            let mut paths = self.stroke_segments(Sides::splat(None));
            assert_eq!(paths.len(), 1);

            Geometry::Path(paths.pop().unwrap().0)
        }
    }

    /// Output the minimum number of paths along the rectangles border.
    fn stroke_segments(
        self,
        strokes: Sides<Option<Stroke>>,
    ) -> Vec<(Path, Option<Stroke>)> {
        let mut res = vec![];

        let mut connection = Connection::default();
        let mut path = Path::new();
        let mut always_continuous = true;

        for side in [Side::Top, Side::Right, Side::Bottom, Side::Left] {
            let is_continuous = strokes.get(side) == strokes.get(side.next_cw());
            connection = connection.advance(is_continuous && side != Side::Left);
            always_continuous &= is_continuous;

            draw_side(
                &mut path,
                side,
                self.size,
                self.radius.get(side.next_ccw()),
                self.radius.get(side),
                connection,
            );

            if !is_continuous {
                res.push((mem::take(&mut path), strokes.get(side)));
            }
        }

        if always_continuous {
            path.close_path();
        }

        if !path.0.is_empty() {
            res.push((path, strokes.left));
        }

        res
    }
}

/// Draws one side of the rounded rectangle. Will always draw the left arc. The
/// right arc will be drawn halfway iff there is no connection.
fn draw_side(
    path: &mut Path,
    side: Side,
    size: Size,
    radius_left: Length,
    radius_right: Length,
    connection: Connection,
) {
    let reversed = |angle: Angle, radius, rotate, mirror_x, mirror_y| {
        let [a, b, c, d] = bezier_arc(angle, radius, rotate, mirror_x, mirror_y);
        [d, c, b, a]
    };

    let angle_left = Angle::deg(if connection.prev { 90.0 } else { 45.0 });
    let angle_right = Angle::deg(if connection.next { 90.0 } else { 45.0 });

    let (arc1, arc2) = match side {
        Side::Top => {
            let arc1 = reversed(angle_left, radius_left, true, true, false)
                .map(|x| x + Point::with_x(radius_left));
            let arc2 = bezier_arc(-angle_right, radius_right, true, true, false)
                .map(|x| x + Point::with_x(size.x - radius_right));

            (arc1, arc2)
        }
        Side::Right => {
            let arc1 = reversed(-angle_left, radius_left, false, false, false)
                .map(|x| x + Point::new(size.x, radius_left));

            let arc2 = bezier_arc(angle_right, radius_right, false, false, false)
                .map(|x| x + Point::new(size.x, size.y - radius_right));

            (arc1, arc2)
        }
        Side::Bottom => {
            let arc1 = reversed(-angle_left, radius_left, true, false, false)
                .map(|x| x + Point::new(size.x - radius_left, size.y));

            let arc2 = bezier_arc(angle_right, radius_right, true, false, false)
                .map(|x| x + Point::new(radius_right, size.y));

            (arc1, arc2)
        }
        Side::Left => {
            let arc1 = reversed(angle_left, radius_left, false, false, true)
                .map(|x| x + Point::with_y(size.y - radius_left));

            let arc2 = bezier_arc(-angle_right, radius_right, false, false, true)
                .map(|x| x + Point::with_y(radius_right));

            (arc1, arc2)
        }
    };

    if !connection.prev {
        path.move_to(if radius_left.is_zero() { arc1[3] } else { arc1[0] });
    }

    if !radius_left.is_zero() {
        path.cubic_to(arc1[1], arc1[2], arc1[3]);
    }

    path.line_to(arc2[0]);

    if !connection.next && !radius_right.is_zero() {
        path.cubic_to(arc2[1], arc2[2], arc2[3]);
    }
}

/// A state machine that indicates which sides of the border strokes in a 2D
/// polygon are connected to their neighboring sides.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
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
