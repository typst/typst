use super::*;

/// A bezier path.
#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct Path(pub Vec<PathElement>);

/// An element in a bezier path.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PathElement {
    MoveTo(Point),
    LineTo(Point),
    CubicTo(Point, Point, Point),
    ClosePath,
}

impl Path {
    /// Create an empty path.
    pub const fn new() -> Self {
        Self(vec![])
    }

    /// Create a path that describes a rectangle.
    pub fn rect(size: Size) -> Self {
        let z = Length::zero();
        let point = Point::new;
        let mut path = Self::new();
        path.move_to(point(z, z));
        path.line_to(point(size.x, z));
        path.line_to(point(size.x, size.y));
        path.line_to(point(z, size.y));
        path.close_path();
        path
    }

    /// Create a path that approximates an axis-aligned ellipse.
    pub fn ellipse(size: Size) -> Self {
        // https://stackoverflow.com/a/2007782
        let z = Length::zero();
        let rx = size.x / 2.0;
        let ry = size.y / 2.0;
        let m = 0.551784;
        let mx = m * rx;
        let my = m * ry;
        let point = |x, y| Point::new(x + rx, y + ry);
        let mut path = Self::new();
        path.move_to(point(-rx, z));
        path.cubic_to(point(-rx, -my), point(-mx, -ry), point(z, -ry));
        path.cubic_to(point(mx, -ry), point(rx, -my), point(rx, z));
        path.cubic_to(point(rx, my), point(mx, ry), point(z, ry));
        path.cubic_to(point(-mx, ry), point(-rx, my), point(-rx, z));
        path
    }

    /// Push a [`MoveTo`](PathElement::MoveTo) element.
    pub fn move_to(&mut self, p: Point) {
        self.0.push(PathElement::MoveTo(p));
    }

    /// Push a [`LineTo`](PathElement::LineTo) element.
    pub fn line_to(&mut self, p: Point) {
        self.0.push(PathElement::LineTo(p));
    }

    /// Push a [`CubicTo`](PathElement::CubicTo) element.
    pub fn cubic_to(&mut self, p1: Point, p2: Point, p3: Point) {
        self.0.push(PathElement::CubicTo(p1, p2, p3));
    }

    /// Push a [`ClosePath`](PathElement::ClosePath) element.
    pub fn close_path(&mut self) {
        self.0.push(PathElement::ClosePath);
    }
}

/// Get the control points for a bezier curve that describes a circular arc
/// of this angle with the given radius.
pub fn bezier_arc(
    angle: Angle,
    radius: Length,
    rotate: bool,
    mirror_x: bool,
    mirror_y: bool,
) -> [Point; 4] {
    let end = Point::new(angle.cos() * radius - radius, angle.sin() * radius);
    let center = Point::new(-radius, Length::zero());

    let mut ts = if mirror_y {
        Transform::mirror_y()
    } else {
        Transform::identity()
    };

    if mirror_x {
        ts = ts.pre_concat(Transform::mirror_x());
    }

    if rotate {
        ts = ts.pre_concat(Transform::rotate(Angle::deg(90.0)));
    }

    let a = center * -1.0;
    let b = end - center;

    let q1 = a.x.to_raw() * a.x.to_raw() + a.y.to_raw() * a.y.to_raw();
    let q2 = q1 + a.x.to_raw() * b.x.to_raw() + a.y.to_raw() * b.y.to_raw();
    let k2 = (4.0 / 3.0) * ((2.0 * q1 * q2).sqrt() - q2)
        / (a.x.to_raw() * b.y.to_raw() - a.y.to_raw() * b.x.to_raw());

    let control_1 = Point::new(center.x + a.x - k2 * a.y, center.y + a.y + k2 * a.x);
    let control_2 = Point::new(center.x + b.x + k2 * b.y, center.y + b.y - k2 * b.x);

    [
        Point::zero(),
        control_1.transform(ts),
        control_2.transform(ts),
        end.transform(ts),
    ]
}
