use super::*;
use kurbo::ParamCurveExtrema;

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

impl PathItem {
    ///// The endpoint of the path operation.
    pub fn endpoint(&self) -> Option<Point> {
        match self {
            PathItem::MoveTo(p) => Some(*p),
            PathItem::LineTo(p) => Some(*p),
            PathItem::CubicTo(_, _, p) => Some(*p),
            PathItem::ClosePath => None,
        }
    }

    /// Extreme point of drawing operation
    pub fn extrema(&self, start: Point) -> Option<Point> {
        match self {
            PathItem::MoveTo(_) => None,
            PathItem::LineTo(p) => Some(p.max(start)),
            PathItem::CubicTo(p1, p2, p3) => {
                let bbox = kurbo::CubicBez::new(start, *p1, *p2, *p3).bounding_box();
                Some(Point::new(Abs::raw(bbox.x1), Abs::raw(bbox.y1)))
            }
            PathItem::ClosePath => None,
        }
    }

    pub fn transform(self, ts: Transform) -> Self {
        match self {
            PathItem::MoveTo(p) => PathItem::MoveTo(p.transform(ts)),
            PathItem::LineTo(p) => PathItem::LineTo(p.transform(ts)),
            PathItem::CubicTo(p1, p2, p3) => {
                PathItem::CubicTo(p1.transform(ts), p2.transform(ts), p3.transform(ts))
            }
            PathItem::ClosePath => PathItem::ClosePath,
        }
    }
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

    /// The size of a path.
    pub fn size(&self) -> Size {
        let mut endpoint = Point::zero();
        self.0
            .iter()
            .flat_map(|p| {
                let val = p.extrema(endpoint);
                if let Some(ep) = p.endpoint() {
                    endpoint = ep;
                }
                val
            })
            .map(|p| Axes::new(p.x, p.y))
            .fold(Size::zero(), Size::max)
    }
}
