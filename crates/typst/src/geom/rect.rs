use super::*;

/// Helper to draw arcs with bezier curves.
trait PathExtension {
    fn arc(&mut self, start: Point, center: Point, end: Point);
    fn arc_move(&mut self, start: Point, center: Point, end: Point);
    fn arc_line(&mut self, start: Point, center: Point, end: Point);
}

/// Get the control points for a bezier curve that approximates a circular arc for
/// a start point, an end point and a center of the circle whose arc connects
/// the two.
fn bezier_arc_control(start: Point, center: Point, end: Point) -> [Point; 2] {
    // https://stackoverflow.com/a/44829356/1567835
    let a = start - center;
    let b = end - center;

    let q1 = a.x.to_raw() * a.x.to_raw() + a.y.to_raw() * a.y.to_raw();
    let q2 = q1 + a.x.to_raw() * b.x.to_raw() + a.y.to_raw() * b.y.to_raw();
    let k2 = (4.0 / 3.0) * ((2.0 * q1 * q2).sqrt() - q2)
        / (a.x.to_raw() * b.y.to_raw() - a.y.to_raw() * b.x.to_raw());

    let control_1 = Point::new(center.x + a.x - k2 * a.y, center.y + a.y + k2 * a.x);
    let control_2 = Point::new(center.x + b.x + k2 * b.y, center.y + b.y - k2 * b.x);

    [control_1, control_2]
}

impl PathExtension for Path {
    fn arc(&mut self, start: Point, center: Point, end: Point) {
        let arc = bezier_arc_control(start, center, end);
        self.cubic_to(arc[0], arc[1], end);
    }

    fn arc_move(&mut self, start: Point, center: Point, end: Point) {
        self.move_to(start);
        self.arc(start, center, end);
    }
    fn arc_line(&mut self, start: Point, center: Point, end: Point) {
        self.line_to(start);
        self.arc(start, center, end);
    }
}

/// Creates a new rectangle as a path.
pub fn path_rect(
    size: Size,
    radius: Corners<Rel<Abs>>,
    stroke: &Sides<Option<FixedStroke>>,
) -> Path {
    if stroke.is_uniform() && radius.iter().cloned().all(Rel::is_zero) {
        Path::rect(size)
    } else {
        segmented_path_rect(size, radius, stroke)
    }
}

/// Create a styled rectangle with shapes.
/// - use rect primitive for simple rectangles
/// - stroke sides if possible
/// - use fill for sides for best looks
pub fn styled_rect(
    size: Size,
    radius: Corners<Rel<Abs>>,
    fill: Option<Paint>,
    stroke: Sides<Option<FixedStroke>>,
) -> Vec<Shape> {
    if stroke.is_uniform() && radius.iter().cloned().all(Rel::is_zero) {
        simple_rect(size, fill, stroke.top)
    } else {
        segmented_rect(size, radius, fill, stroke)
    }
}

/// Use rect primitive for the rectangle
fn simple_rect(
    size: Size,
    fill: Option<Paint>,
    stroke: Option<FixedStroke>,
) -> Vec<Shape> {
    vec![Shape { geometry: Geometry::Rect(size), fill, stroke }]
}

fn corners_control_points(
    size: Size,
    radius: Corners<Abs>,
    strokes: &Sides<Option<FixedStroke>>,
    stroke_widths: Sides<Abs>,
) -> Corners<ControlPoints> {
    Corners {
        top_left: Corner::TopLeft,
        top_right: Corner::TopRight,
        bottom_right: Corner::BottomRight,
        bottom_left: Corner::BottomLeft,
    }
    .map(|corner| ControlPoints {
        radius: radius.get(corner),
        stroke_before: stroke_widths.get(corner.side_ccw()),
        stroke_after: stroke_widths.get(corner.side_cw()),
        corner,
        size,
        same: match (
            strokes.get_ref(corner.side_ccw()),
            strokes.get_ref(corner.side_cw()),
        ) {
            (Some(a), Some(b)) => a.paint == b.paint && a.dash_pattern == b.dash_pattern,
            (None, None) => true,
            _ => false,
        },
    })
}

fn segmented_path_rect(
    size: Size,
    radius: Corners<Rel<Abs>>,
    strokes: &Sides<Option<FixedStroke>>,
) -> Path {
    let stroke_widths = strokes
        .as_ref()
        .map(|s| s.as_ref().map_or(Abs::zero(), |s| s.thickness / 2.0));

    let max_radius = (size.x.min(size.y)) / 2.0
        + stroke_widths.iter().cloned().min().unwrap_or(Abs::zero());

    let radius = radius.map(|side| side.relative_to(max_radius * 2.0).min(max_radius));

    // insert stroked sides below filled sides
    let mut path = Path::new();
    let corners = corners_control_points(size, radius, strokes, stroke_widths);
    let current = corners.iter().find(|c| !c.same).map(|c| c.corner);
    if let Some(mut current) = current {
        // multiple segments
        // start at a corner with a change between sides and iterate clockwise all other corners
        let mut last = current;
        for _ in 0..4 {
            current = current.next_cw();
            if corners.get_ref(current).same {
                continue;
            }
            // create segment
            let start = last;
            let end = current;
            last = current;
            path_segment(start, end, &corners, &mut path);
        }
    } else if strokes.top.is_some() {
        // single segment
        path_segment(Corner::TopLeft, Corner::TopLeft, &corners, &mut path);
    }
    path
}

/// Use stroke and fill for the rectangle
fn segmented_rect(
    size: Size,
    radius: Corners<Rel<Abs>>,
    fill: Option<Paint>,
    strokes: Sides<Option<FixedStroke>>,
) -> Vec<Shape> {
    let mut res = vec![];
    let stroke_widths = strokes
        .as_ref()
        .map(|s| s.as_ref().map_or(Abs::zero(), |s| s.thickness / 2.0));

    let max_radius = (size.x.min(size.y)) / 2.0
        + stroke_widths.iter().cloned().min().unwrap_or(Abs::zero());

    let radius = radius.map(|side| side.relative_to(max_radius * 2.0).min(max_radius));

    let corners = corners_control_points(size, radius, &strokes, stroke_widths);

    // insert stroked sides below filled sides
    let mut stroke_insert = 0;

    // fill shape with inner curve
    if let Some(fill) = fill {
        let mut path = Path::new();
        let c = corners.get_ref(Corner::TopLeft);
        if c.arc() {
            path.arc_move(c.start(), c.center(), c.end());
        } else {
            path.move_to(c.center());
        };

        for corner in [Corner::TopRight, Corner::BottomRight, Corner::BottomLeft] {
            let c = corners.get_ref(corner);
            if c.arc() {
                path.arc_line(c.start(), c.center(), c.end());
            } else {
                path.line_to(c.center());
            }
        }
        path.close_path();
        res.push(Shape {
            geometry: Geometry::Path(path),
            fill: Some(fill),
            stroke: None,
        });
        stroke_insert += 1;
    }

    let current = corners.iter().find(|c| !c.same).map(|c| c.corner);
    if let Some(mut current) = current {
        // multiple segments
        // start at a corner with a change between sides and iterate clockwise all other corners
        let mut last = current;
        for _ in 0..4 {
            current = current.next_cw();
            if corners.get_ref(current).same {
                continue;
            }
            // create segment
            let start = last;
            let end = current;
            last = current;
            let stroke = match strokes.get_ref(start.side_cw()) {
                None => continue,
                Some(stroke) => stroke.clone(),
            };
            let (shape, ontop) = segment(start, end, &corners, stroke);
            if ontop {
                res.push(shape);
            } else {
                res.insert(stroke_insert, shape);
                stroke_insert += 1;
            }
        }
    } else if let Some(stroke) = strokes.top {
        // single segment
        let (shape, _) = segment(Corner::TopLeft, Corner::TopLeft, &corners, stroke);
        res.push(shape);
    }
    res
}

fn path_segment(
    start: Corner,
    end: Corner,
    corners: &Corners<ControlPoints>,
    path: &mut Path,
) {
    // create start corner
    let c = corners.get_ref(start);
    if start == end || !c.arc() {
        path.move_to(c.end());
    } else {
        path.arc_move(c.mid(), c.center(), c.end());
    }

    // create corners between start and end
    let mut current = start.next_cw();
    while current != end {
        let c = corners.get_ref(current);
        if c.arc() {
            path.arc_line(c.start(), c.center(), c.end());
        } else {
            path.line_to(c.end());
        }
        current = current.next_cw();
    }

    // create end corner
    let c = corners.get_ref(end);
    if !c.arc() {
        path.line_to(c.start());
    } else if start == end {
        path.arc_line(c.start(), c.center(), c.end());
    } else {
        path.arc_line(c.start(), c.center(), c.mid());
    }
}

/// Returns the shape for the segment and whether the shape should be drawn on top.
fn segment(
    start: Corner,
    end: Corner,
    corners: &Corners<ControlPoints>,
    stroke: FixedStroke,
) -> (Shape, bool) {
    fn fill_corner(corner: &ControlPoints) -> bool {
        corner.stroke_before != corner.stroke_after
            || corner.radius() < corner.stroke_before
    }

    fn fill_corners(
        start: Corner,
        end: Corner,
        corners: &Corners<ControlPoints>,
    ) -> bool {
        if fill_corner(corners.get_ref(start)) {
            return true;
        }
        if fill_corner(corners.get_ref(end)) {
            return true;
        }
        let mut current = start.next_cw();
        while current != end {
            if fill_corner(corners.get_ref(current)) {
                return true;
            }
            current = current.next_cw();
        }
        false
    }

    let solid = stroke
        .dash_pattern
        .as_ref()
        .map(|pattern| pattern.array.is_empty())
        .unwrap_or(true);

    let use_fill = solid && fill_corners(start, end, corners);

    let shape = if use_fill {
        fill_segment(start, end, corners, stroke)
    } else {
        stroke_segment(start, end, corners, stroke)
    };
    (shape, use_fill)
}

/// Stroke the sides from `start` to `end` clockwise.
fn stroke_segment(
    start: Corner,
    end: Corner,
    corners: &Corners<ControlPoints>,
    stroke: FixedStroke,
) -> Shape {
    // create start corner
    let mut path = Path::new();
    path_segment(start, end, corners, &mut path);

    Shape {
        geometry: Geometry::Path(path),
        stroke: Some(stroke),
        fill: None,
    }
}

/// Fill the sides from `start` to `end` clockwise.
fn fill_segment(
    start: Corner,
    end: Corner,
    corners: &Corners<ControlPoints>,
    stroke: FixedStroke,
) -> Shape {
    let mut path = Path::new();

    // create the start corner
    // begin on the inside and finish on the outside
    // no corner if start and end are equal
    // half corner if different
    if start == end {
        let c = corners.get_ref(start);
        path.move_to(c.end_inner());
        path.line_to(c.end_outer());
    } else {
        let c = corners.get_ref(start);

        if c.arc_inner() {
            path.arc_move(c.end_inner(), c.center_inner(), c.mid_inner());
        } else {
            path.move_to(c.end_inner());
        }

        if c.arc_outer() {
            path.arc_line(c.mid_outer(), c.center_outer(), c.end_outer());
        } else {
            path.line_to(c.outer());
            path.line_to(c.end_outer());
        }
    }

    // create the clockwise outside path for the corners between start and end
    let mut current = start.next_cw();
    while current != end {
        let c = corners.get_ref(current);
        if c.arc_outer() {
            path.arc_line(c.start_outer(), c.center_outer(), c.end_outer());
        } else {
            path.line_to(c.outer());
        }
        current = current.next_cw();
    }

    // create the end corner
    // begin on the outside and finish on the inside
    // full corner if start and end are equal
    // half corner if different
    if start == end {
        let c = corners.get_ref(end);
        if c.arc_outer() {
            path.arc_line(c.start_outer(), c.center_outer(), c.end_outer());
        } else {
            path.line_to(c.outer());
            path.line_to(c.end_outer());
        }
        if c.arc_inner() {
            path.arc_line(c.end_inner(), c.center_inner(), c.start_inner());
        } else {
            path.line_to(c.center_inner());
        }
    } else {
        let c = corners.get_ref(end);
        if c.arc_outer() {
            path.arc_line(c.start_outer(), c.center_outer(), c.mid_outer());
        } else {
            path.line_to(c.outer());
        }
        if c.arc_inner() {
            path.arc_line(c.mid_inner(), c.center_inner(), c.start_inner());
        } else {
            path.line_to(c.center_inner());
        }
    }

    // create the counterclockwise inside path for the corners between start and end
    let mut current = end.next_ccw();
    while current != start {
        let c = corners.get_ref(current);
        if c.arc_inner() {
            path.arc_line(c.end_inner(), c.center_inner(), c.start_inner());
        } else {
            path.line_to(c.center_inner());
        }
        current = current.next_ccw();
    }

    path.close_path();

    Shape {
        geometry: Geometry::Path(path),
        stroke: None,
        fill: Some(stroke.paint),
    }
}

/// Helper to calculate different control points for the corners.
/// Clockwise orientation from start to end.
/// ```text
/// O-------------------EO  ---   - Z: Zero/Origin ({x: 0, y: 0} for top left corner)
/// |\   ___----'''     |    |    - O: Outer: intersection between the straight outer lines
/// | \ /               |    |    - S_: start
/// |  MO               |    |    - M_: midpoint
/// | /Z\  __-----------E    |    - E_: end
/// |/   \M             |    ro   - r_: radius
/// |    /\             |    |    - middle of the stroke
/// |   /  \            |    |      - arc from S through M to E with center C and radius r
/// |  |    MI--EI-------    |    - outer curve
/// |  |  /  \               |      - arc from SO through MO to EO with center CO and radius ro
/// SO | |    \         CO  ---   - inner curve
/// |  | |     \                    - arc from SI through MI to EI with center CI and radius ri
/// |--S-SI-----CI      C
///      |--ri--|
///    |-------r--------|
/// ```
struct ControlPoints {
    radius: Abs,
    stroke_after: Abs,
    stroke_before: Abs,
    corner: Corner,
    size: Size,
    same: bool,
}

impl ControlPoints {
    /// Move and rotate the point from top-left to the required corner.
    fn rotate(&self, point: Point) -> Point {
        match self.corner {
            Corner::TopLeft => point,
            Corner::TopRight => Point { x: self.size.x - point.y, y: point.x },
            Corner::BottomRight => {
                Point { x: self.size.x - point.x, y: self.size.y - point.y }
            }
            Corner::BottomLeft => Point { x: point.y, y: self.size.y - point.x },
        }
    }

    /// Outside intersection of the sides.
    pub fn outer(&self) -> Point {
        self.rotate(Point { x: -self.stroke_before, y: -self.stroke_after })
    }

    /// Center for the outer arc.
    pub fn center_outer(&self) -> Point {
        let r = self.radius_outer();
        self.rotate(Point {
            x: r - self.stroke_before,
            y: r - self.stroke_after,
        })
    }

    /// Center for the middle arc.
    pub fn center(&self) -> Point {
        let r = self.radius();
        self.rotate(Point { x: r, y: r })
    }

    /// Center for the inner arc.
    pub fn center_inner(&self) -> Point {
        let r = self.radius_inner();

        self.rotate(Point {
            x: self.stroke_before + r,
            y: self.stroke_after + r,
        })
    }

    /// Radius of the outer arc.
    pub fn radius_outer(&self) -> Abs {
        self.radius
    }

    /// Radius of the middle arc.
    pub fn radius(&self) -> Abs {
        (self.radius - self.stroke_before.min(self.stroke_after)).max(Abs::zero())
    }

    /// Radius of the inner arc.
    pub fn radius_inner(&self) -> Abs {
        (self.radius - 2.0 * self.stroke_before.max(self.stroke_after)).max(Abs::zero())
    }

    /// Middle of the corner on the outside of the stroke.
    pub fn mid_outer(&self) -> Point {
        let c_i = self.center_inner();
        let c_o = self.center_outer();
        let o = self.outer();
        let r = self.radius_outer();

        // https://math.stackexchange.com/a/311956
        // intersection between the line from inner center to outside and the outer arc
        let a = (o.x - c_i.x).to_raw().powi(2) + (o.y - c_i.y).to_raw().powi(2);
        let b = 2.0 * (o.x - c_i.x).to_raw() * (c_i.x - c_o.x).to_raw()
            + 2.0 * (o.y - c_i.y).to_raw() * (c_i.y - c_o.y).to_raw();
        let c = (c_i.x - c_o.x).to_raw().powi(2) + (c_i.y - c_o.y).to_raw().powi(2)
            - r.to_raw().powi(2);
        let t = (-b + (b * b - 4.0 * a * c).sqrt()) / (2.0 * a);
        c_i + t * (o - c_i)
    }

    /// Middle of the corner in the middle of the stroke.
    pub fn mid(&self) -> Point {
        let center = self.center_outer();
        let outer = self.outer();
        let diff = outer - center;
        center + diff / diff.hypot().to_raw() * self.radius().to_raw()
    }

    /// Middle of the corner on the inside of the stroke.
    pub fn mid_inner(&self) -> Point {
        let center = self.center_inner();
        let outer = self.outer();
        let diff = outer - center;
        center + diff / diff.hypot().to_raw() * self.radius_inner().to_raw()
    }

    /// If an outer arc is required.
    pub fn arc_outer(&self) -> bool {
        self.radius_outer() > Abs::zero()
    }

    pub fn arc(&self) -> bool {
        self.radius() > Abs::zero()
    }

    /// If an inner arc is required.
    pub fn arc_inner(&self) -> bool {
        self.radius_inner() > Abs::zero()
    }

    /// Start of the corner on the outside of the stroke.
    pub fn start_outer(&self) -> Point {
        self.rotate(Point {
            x: -self.stroke_before,
            y: self.radius_outer() - self.stroke_after,
        })
    }

    /// Start of the corner in the center of the stroke.
    pub fn start(&self) -> Point {
        self.rotate(Point::with_y(self.radius()))
    }

    /// Start of the corner on the inside of the stroke.
    pub fn start_inner(&self) -> Point {
        self.rotate(Point {
            x: self.stroke_before,
            y: self.stroke_after + self.radius_inner(),
        })
    }

    /// End of the corner on the outside of the stroke.
    pub fn end_outer(&self) -> Point {
        self.rotate(Point {
            x: self.radius_outer() - self.stroke_before,
            y: -self.stroke_after,
        })
    }

    /// End of the corner in the center of the stroke.
    pub fn end(&self) -> Point {
        self.rotate(Point::with_x(self.radius()))
    }

    /// End of the corner on the inside of the stroke.
    pub fn end_inner(&self) -> Point {
        self.rotate(Point {
            x: self.stroke_before + self.radius_inner(),
            y: self.stroke_after,
        })
    }
}
