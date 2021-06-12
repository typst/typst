use super::*;

use serde::{Deserialize, Serialize};

/// A bezier path.
#[derive(Default, Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Path(pub Vec<PathElement>);

/// An element in a bezier path.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum PathElement {
    MoveTo(Point),
    LineTo(Point),
    CubicTo(Point, Point, Point),
    ClosePath,
}

impl Path {
    /// Create an empty path.
    pub fn new() -> Self {
        Self(vec![])
    }

    /// Create a path that approximates an axis-aligned ellipse.
    pub fn ellipse(size: Size) -> Self {
        // https://stackoverflow.com/a/2007782
        let rx = size.width / 2.0;
        let ry = size.height / 2.0;
        let m = 0.551784;
        let mx = m * rx;
        let my = m * ry;
        let z = Length::zero();
        let point = Point::new;
        let mut path = Self::new();
        path.move_to(point(-rx, z));
        path.cubic_to(point(-rx, my), point(-mx, ry), point(z, ry));
        path.cubic_to(point(mx, ry), point(rx, my), point(rx, z));
        path.cubic_to(point(rx, -my), point(mx, -ry), point(z, -ry));
        path.cubic_to(point(-mx, -ry), point(-rx, -my), point(z - rx, z));
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
