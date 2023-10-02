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

        let mut current = Point::zero();
        for item in self.0.iter() {
            match item {
                PathItem::MoveTo(item) => {
                    min_x = min_x.min(current.x);
                    min_y = min_y.min(current.y);
                    max_x = max_x.max(current.x);
                    max_y = max_y.max(current.y);

                    current = *item;
                }
                PathItem::LineTo(item) => {
                    min_x = min_x.min(current.x);
                    min_y = min_y.min(current.y);
                    max_x = max_x.max(current.x);
                    max_y = max_y.max(current.y);

                    current = *item;
                }
                PathItem::CubicTo(c0, c1, item) => {
                    // Compute the bounding box of the bezier curve.
                    let (xl, xh) = cubic_bezier_bounds_1d(current.x, c0.x, c1.x, item.x);
                    let (yl, yh) = cubic_bezier_bounds_1d(current.y, c0.y, c1.y, item.y);

                    min_x = min_x.min(xl).min(xh);
                    min_y = min_y.min(yl).min(yh);
                    max_x = max_x.max(xl).max(xh);
                    max_y = max_y.max(yl).max(yh);

                    current = *item;
                }
                PathItem::ClosePath => (),
            }
        }

        Size::new(max_x - min_x, max_y - min_y)
    }
}

/// Compute the 1D bound of a cubic bezier curve.
/// Returns:
/// - The lower bound.
/// - The upper bound.
///
/// Source: https://gist.github.com/steveruizok/1ef8a9e0257768c3f8e33c9904b38de5
fn cubic_bezier_bounds_1d(p0: Abs, c0: Abs, c1: Abs, p1: Abs) -> (Abs, Abs) {
    let a = 3.0 * p1.to_pt() - 9.0 * c1.to_pt() + 9.0 * c0.to_pt() - 3.0 * p0.to_pt();
    let b = 6.0 * p0.to_pt() - 12.0 * c0.to_pt() + 6.0 * c1.to_pt();
    let c = 3.0 * c0.to_pt() - 3.0 * p0.to_pt();
    let disc = b * b - 4.0 * a * c;

    let mut xl = p0.to_pt();
    let mut xh = p0.to_pt();
    if p1.to_pt() < xl {
        xl = p1.to_pt();
    }

    if p1.to_pt() > xh {
        xh = p1.to_pt();
    }

    if disc >= 0.0 {
        let t1 = (-b + disc.sqrt()) / (2.0 * a);
        if t1 > 0.0 && t1 < 1.0 {
            let x1 = bez1d(p0.to_pt(), c0.to_pt(), c1.to_pt(), p1.to_pt(), t1);
            if x1 < xl {
                xl = x1;
            }

            if x1 > xh {
                xh = x1;
            }
        }

        let t2 = (-b - disc.sqrt()) / (2.0 * a);
        if t2 > 0.0 && t2 < 1.0 {
            let x2 = bez1d(p0.to_pt(), c0.to_pt(), c1.to_pt(), p1.to_pt(), t2);
            if x2 < xl {
                xl = x2;
            }

            if x2 > xh {
                xh = x2;
            }
        }
    }

    (Abs::pt(xl), Abs::pt(xh))
}

fn bez1d(a: f64, b: f64, c: f64, d: f64, t: f64) -> f64 {
    a * (1.0 - t) * (1.0 - t) * (1.0 - t)
        + 3.0 * b * t * (1.0 - t) * (1.0 - t)
        + 3.0 * c * t * t * (1.0 - t)
        + d * t * t * t
}
