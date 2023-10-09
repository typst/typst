use kurbo::Shape;

use super::*;

/// A bezier path.
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct Path(pub Vec<PathItem>);

/// An item in a bezier path.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum PathItem {
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

    /// Push a [`MoveTo`](PathItem::MoveTo) item.
    pub fn move_to(&mut self, p: Point) {
        self.0.push(PathItem::MoveTo(p));
    }

    /// Push a [`LineTo`](PathItem::LineTo) item.
    pub fn line_to(&mut self, p: Point) {
        self.0.push(PathItem::LineTo(p));
    }

    /// Push a [`CubicTo`](PathItem::CubicTo) item.
    pub fn cubic_to(&mut self, p1: Point, p2: Point, p3: Point) {
        self.0.push(PathItem::CubicTo(p1, p2, p3));
    }

    /// Push a [`ClosePath`](PathItem::ClosePath) item.
    pub fn close_path(&mut self) {
        self.0.push(PathItem::ClosePath);
    }

    /// Computes the size of bounding box of this path.
    pub fn bbox_size(&self) -> Size {
        let mut min_x = Abs::inf();
        let mut min_y = Abs::inf();
        let mut max_x = -Abs::inf();
        let mut max_y = -Abs::inf();

        let mut cursor = Point::zero();
        for item in self.0.iter() {
            match item {
                PathItem::MoveTo(to) => {
                    min_x = min_x.min(cursor.x);
                    min_y = min_y.min(cursor.y);
                    max_x = max_x.max(cursor.x);
                    max_y = max_y.max(cursor.y);
                    cursor = *to;
                }
                PathItem::LineTo(to) => {
                    min_x = min_x.min(cursor.x);
                    min_y = min_y.min(cursor.y);
                    max_x = max_x.max(cursor.x);
                    max_y = max_y.max(cursor.y);
                    cursor = *to;
                }
                PathItem::CubicTo(c0, c1, end) => {
                    let cubic = kurbo::CubicBez::new(
                        kurbo::Point::new(cursor.x.to_pt(), cursor.y.to_pt()),
                        kurbo::Point::new(c0.x.to_pt(), c0.y.to_pt()),
                        kurbo::Point::new(c1.x.to_pt(), c1.y.to_pt()),
                        kurbo::Point::new(end.x.to_pt(), end.y.to_pt()),
                    );

                    let bbox = cubic.bounding_box();
                    min_x = min_x.min(Abs::pt(bbox.x0)).min(Abs::pt(bbox.x1));
                    min_y = min_y.min(Abs::pt(bbox.y0)).min(Abs::pt(bbox.y1));
                    max_x = max_x.max(Abs::pt(bbox.x0)).max(Abs::pt(bbox.x1));
                    max_y = max_y.max(Abs::pt(bbox.y0)).max(Abs::pt(bbox.y1));
                    cursor = *end;
                }
                PathItem::ClosePath => (),
            }
        }

        Size::new(max_x - min_x, max_y - min_y)
    }
}
