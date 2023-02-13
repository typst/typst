use super::*;

/// A bezier path.
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct Path(pub Vec<PathElement>);

/// An element in a bezier path.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
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
        let z = Abs::zero();
        let point = Point::new;
        let mut path = Self::new();
        path.move_to(point(z, z));
        path.line_to(point(size.x, z));
        path.line_to(point(size.x, size.y));
        path.line_to(point(z, size.y));
        path.close_path();
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
