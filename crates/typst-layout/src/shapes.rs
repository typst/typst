use std::f64::consts::SQRT_2;

use kurbo::{CubicBez, ParamCurveExtrema};
use typst_library::diag::{bail, SourceResult};
use typst_library::engine::Engine;
use typst_library::foundations::{Content, Packed, Resolve, Smart, StyleChain};
use typst_library::introspection::Locator;
use typst_library::layout::{
    Abs, Axes, Corner, Corners, Frame, FrameItem, Length, Point, Ratio, Region, Rel,
    Sides, Size,
};
use typst_library::visualize::{
    CircleElem, CloseMode, EllipseElem, FillRule, FixedStroke, Geometry, LineElem, Paint,
    Path, PathComponent, PathElem, PolygonElem, RectElem, Shape, SquareElem, Stroke,
};
use typst_syntax::Span;
use typst_utils::{Get, Numeric};

/// Layout the line.
#[typst_macros::time(span = elem.span())]
pub fn layout_line(
    elem: &Packed<LineElem>,
    _: &mut Engine,
    _: Locator,
    styles: StyleChain,
    region: Region,
) -> SourceResult<Frame> {
    let resolve = |axes: Axes<Rel<Abs>>| axes.zip_map(region.size, Rel::relative_to);
    let start = resolve(elem.start(styles));
    let delta = elem.end(styles).map(|end| resolve(end) - start).unwrap_or_else(|| {
        let length = elem.length(styles);
        let angle = elem.angle(styles);
        let x = angle.cos() * length;
        let y = angle.sin() * length;
        resolve(Axes::new(x, y))
    });

    let stroke = elem.stroke(styles).unwrap_or_default();
    let size = start.max(start + delta).max(Size::zero());

    if !size.is_finite() {
        bail!(elem.span(), "cannot create line with infinite length");
    }

    let mut frame = Frame::soft(size);
    let shape = Geometry::Line(delta.to_point()).stroked(stroke);
    frame.push(start.to_point(), FrameItem::Shape(shape, elem.span()));
    Ok(frame)
}

struct PathBuilder<'a> {
    path: Path,
    size: Size,
    close_mode: Option<CloseMode>,
    region: Region,
    styles: StyleChain<'a>,
    start_point: Point,
    start_control_into: Point, // cubic
    last_point: Point,
    last_control_from: Point, // cubic
    /// Has a new component be started?
    /// This does not mean that something has been added to self.path yet.
    is_started: bool,
    /// Has anything actually be added to self.path for the current component?
    is_empty: bool,
}

impl<'a> PathBuilder<'a> {
    fn new(
        region: Region,
        styles: StyleChain<'a>,
        close_mode: Option<CloseMode>,
    ) -> Self {
        Self {
            path: Path::new(),
            size: Size::zero(),
            close_mode,
            region,
            styles,
            start_point: Default::default(),
            start_control_into: Default::default(),
            last_point: Default::default(),
            last_control_from: Default::default(),
            is_started: false,
            is_empty: true,
        }
    }

    fn adjust_bounds(&mut self, point: Point) {
        self.size.x.set_max(point.x);
        self.size.y.set_max(point.y);
    }

    fn resolve_old_point(&self, axes: &Axes<Rel<Length>>) -> Point {
        axes.resolve(self.styles)
            .zip_map(self.region.size, Rel::relative_to)
            .to_point()
    }

    fn resolve_point(&self, point: Axes<Rel<Abs>>, relative: bool) -> Point {
        let p = point.zip_map(self.region.size, Rel::relative_to);
        let mut p = Point::new(p.x, p.y);
        if relative {
            p += self.last_point;
        }
        p
    }

    /// Push the initial move of a new component to self.path.
    fn start_component(&mut self) {
        self.path.move_to(self.start_point);
        self.is_empty = false;
    }

    fn vertex(&mut self, point: Point, cinto: Point, cfrom: Point) {
        if !self.is_started {
            self.move_to(point);
            self.start_control_into = point + cinto;
        } else {
            let old = self.start_control_into;
            self.cubic_to(self.last_control_from, point + cinto, point);
            self.start_control_into = old;
        }
        self.last_control_from = point + cfrom;
    }

    fn move_to(&mut self, point: Point) {
        self.close(self.close_mode);
        // Delay calling path.move_to in case there is another move_to element
        // before any actual drawing.
        self.adjust_bounds(point);
        self.start_point = point;
        self.start_control_into = point;
        self.last_point = point;
        self.last_control_from = point;
        self.is_started = true;
    }

    fn line_to(&mut self, point: Point) {
        if self.is_empty {
            self.start_component();
            self.start_control_into = self.start_point;
        }
        self.path.line_to(point);
        self.adjust_bounds(point);
        self.last_point = point;
        self.last_control_from = point;
    }

    fn quadratic_to(&mut self, control: Point, end: Point) {
        let c1 = control_q2c(self.last_point, control);
        let c2 = control_q2c(end, control);
        self.cubic_to(c1, c2, end);
    }

    fn cubic_to(&mut self, c1: Point, c2: Point, end: Point) {
        if self.is_empty {
            self.start_component();
            self.start_control_into = mirror_c(self.start_point, c1);
        }
        self.path.cubic_to(c1, c2, end);

        fn to_kurbo(point: Point) -> kurbo::Point {
            kurbo::Point::new(point.x.to_raw(), point.y.to_raw())
        }
        let p0 = to_kurbo(self.last_point);
        let p1 = to_kurbo(c1);
        let p2 = to_kurbo(c2);
        let p3 = to_kurbo(end);
        let extrema = CubicBez::new(p0, p1, p2, p3).bounding_box();
        self.size.x.set_max(Abs::raw(extrema.x1));
        self.size.y.set_max(Abs::raw(extrema.y1));

        self.last_point = end;
        self.last_control_from = mirror_c(end, c2);
    }

    fn close(&mut self, mode: Option<CloseMode>) {
        if self.is_started && !self.is_empty {
            if let Some(mode) = mode {
                if mode == CloseMode::Curve {
                    self.cubic_to(
                        self.last_control_from,
                        self.start_control_into,
                        self.start_point,
                    );
                }
                self.path.close_path();
                self.last_point = self.start_point;
                self.last_control_from = self.start_point;
            }
        }
        self.is_started = false;
        self.is_empty = true;
    }

    fn build(mut self) -> (Path, Size) {
        self.close(self.close_mode);
        (self.path, self.size)
    }
}

/// Convert a cubic control point into a quadratic one.
fn control_c2q(p: Point, c: Point) -> Point {
    1.5 * c - 0.5 * p
}

/// Convert a quadratic control point into a cubic one.
fn control_q2c(p: Point, c: Point) -> Point {
    (p + 2. * c) / 3.
}

/// Mirror a control point.
fn mirror_c(p: Point, c: Point) -> Point {
    2. * p - c
}

/// Layout the path.
#[typst_macros::time(span = elem.span())]
pub fn layout_path(
    elem: &Packed<PathElem>,
    _: &mut Engine,
    _: Locator,
    styles: StyleChain,
    region: Region,
) -> SourceResult<Frame> {
    let default_close_mode = elem.close_mode(styles);
    let close_mode = elem.closed(styles).then_some(default_close_mode).flatten();
    let mut builder = PathBuilder::new(region, styles, close_mode);

    for item in elem.vertices() {
        match item {
            PathComponent::SimplePoint(point) => {
                let point = builder.resolve_old_point(point);
                builder.vertex(point, Default::default(), Default::default());
            }
            PathComponent::MirroredControlPoint(point, cinto) => {
                let point = builder.resolve_old_point(point);
                let cinto = builder.resolve_old_point(cinto);
                builder.vertex(point, cinto, -cinto);
            }
            PathComponent::AllControlPoints(point, cinto, cfrom) => {
                let point = builder.resolve_old_point(point);
                let cinto = builder.resolve_old_point(cinto);
                let cfrom = builder.resolve_old_point(cfrom);
                builder.vertex(point, cinto, cfrom);
            }
            PathComponent::Vertex(element) => {
                let relative = element.relative(styles);
                let point = builder.resolve_point(element.point(styles), relative);
                let cinto = builder.resolve_point(element.control_into(styles), false);
                let cfrom = element
                    .control_from(styles)
                    .map(|p| builder.resolve_point(p, false))
                    .unwrap_or(-cinto);
                builder.vertex(point, cinto, cfrom);
            }
            PathComponent::MoveTo(element) => {
                let relative = element.relative(styles);
                let point = builder.resolve_point(element.start(styles), relative);
                builder.move_to(point);
            }
            PathComponent::LineTo(element) => {
                let relative = element.relative(styles);
                let point = builder.resolve_point(element.end(styles), relative);
                builder.line_to(point);
            }
            PathComponent::QuadraticTo(element) => {
                let relative = element.relative(styles);
                let control = match element.control(styles) {
                    Smart::Custom(p) => builder.resolve_point(p, relative),
                    Smart::Auto => {
                        control_c2q(builder.last_point, builder.last_control_from)
                    }
                };
                let end = builder.resolve_point(element.end(styles), relative);
                builder.quadratic_to(control, end);
            }
            PathComponent::CubicTo(element) => {
                let relative = element.relative(styles);
                let c1 = match element.cstart(styles) {
                    Smart::Custom(p) => builder.resolve_point(p, relative),
                    Smart::Auto => builder.last_control_from,
                };
                let c2 = builder.resolve_point(element.cend(styles), relative);
                let end = builder.resolve_point(element.end(styles), relative);
                builder.cubic_to(c1, c2, end);
            }
            PathComponent::ClosePath(element) => {
                let mode = element.mode(styles);
                builder.close(mode.unwrap_or(default_close_mode));
            }
        }
    }

    let (path, size) = builder.build();

    if path.is_empty() {
        return Ok(Frame::soft(size));
    }

    // Prepare fill and stroke.
    let fill = elem.fill(styles);
    let fill_rule = elem.fill_rule(styles);
    let stroke = match elem.stroke(styles) {
        Smart::Auto if fill.is_none() => Some(FixedStroke::default()),
        Smart::Auto => None,
        Smart::Custom(stroke) => stroke.map(Stroke::unwrap_or_default),
    };

    let mut frame = Frame::soft(size);
    let shape = Shape {
        geometry: Geometry::Path(path),
        stroke,
        fill,
        fill_rule,
    };
    frame.push(Point::zero(), FrameItem::Shape(shape, elem.span()));
    Ok(frame)
}

/// Layout the polygon.
#[typst_macros::time(span = elem.span())]
pub fn layout_polygon(
    elem: &Packed<PolygonElem>,
    _: &mut Engine,
    _: Locator,
    styles: StyleChain,
    region: Region,
) -> SourceResult<Frame> {
    let points: Vec<Point> = elem
        .vertices()
        .iter()
        .map(|c| c.resolve(styles).zip_map(region.size, Rel::relative_to).to_point())
        .collect();

    let size = points.iter().fold(Point::zero(), |max, c| c.max(max)).to_size();
    if !size.is_finite() {
        bail!(elem.span(), "cannot create polygon with infinite size");
    }

    let mut frame = Frame::hard(size);

    // Only create a path if there are more than zero points.
    if points.is_empty() {
        return Ok(frame);
    }

    // Prepare fill and stroke.
    let fill = elem.fill(styles);
    let fill_rule = elem.fill_rule(styles);
    let stroke = match elem.stroke(styles) {
        Smart::Auto if fill.is_none() => Some(FixedStroke::default()),
        Smart::Auto => None,
        Smart::Custom(stroke) => stroke.map(Stroke::unwrap_or_default),
    };

    // Construct a closed path given all points.
    let mut path = Path::new();
    path.move_to(points[0]);
    for &point in &points[1..] {
        path.line_to(point);
    }
    path.close_path();

    let shape = Shape {
        geometry: Geometry::Path(path),
        stroke,
        fill,
        fill_rule,
    };
    frame.push(Point::zero(), FrameItem::Shape(shape, elem.span()));
    Ok(frame)
}

/// Lay out the rectangle.
#[typst_macros::time(span = elem.span())]
pub fn layout_rect(
    elem: &Packed<RectElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    region: Region,
) -> SourceResult<Frame> {
    layout_shape(
        engine,
        locator,
        styles,
        region,
        ShapeKind::Rect,
        elem.body(styles),
        elem.fill(styles),
        elem.stroke(styles),
        elem.inset(styles),
        elem.outset(styles),
        elem.radius(styles),
        elem.span(),
    )
}

/// Lay out the square.
#[typst_macros::time(span = elem.span())]
pub fn layout_square(
    elem: &Packed<SquareElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    region: Region,
) -> SourceResult<Frame> {
    layout_shape(
        engine,
        locator,
        styles,
        region,
        ShapeKind::Square,
        elem.body(styles),
        elem.fill(styles),
        elem.stroke(styles),
        elem.inset(styles),
        elem.outset(styles),
        elem.radius(styles),
        elem.span(),
    )
}

/// Lay out the ellipse.
#[typst_macros::time(span = elem.span())]
pub fn layout_ellipse(
    elem: &Packed<EllipseElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    region: Region,
) -> SourceResult<Frame> {
    layout_shape(
        engine,
        locator,
        styles,
        region,
        ShapeKind::Ellipse,
        elem.body(styles),
        elem.fill(styles),
        elem.stroke(styles).map(|s| Sides::splat(Some(s))),
        elem.inset(styles),
        elem.outset(styles),
        Corners::splat(None),
        elem.span(),
    )
}

/// Lay out the circle.
#[typst_macros::time(span = elem.span())]
pub fn layout_circle(
    elem: &Packed<CircleElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    region: Region,
) -> SourceResult<Frame> {
    layout_shape(
        engine,
        locator,
        styles,
        region,
        ShapeKind::Circle,
        elem.body(styles),
        elem.fill(styles),
        elem.stroke(styles).map(|s| Sides::splat(Some(s))),
        elem.inset(styles),
        elem.outset(styles),
        Corners::splat(None),
        elem.span(),
    )
}

/// A category of shape.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
enum ShapeKind {
    /// A rectangle with equal side lengths.
    Square,
    /// A quadrilateral with four right angles.
    Rect,
    /// An ellipse with coinciding foci.
    Circle,
    /// A curve around two focal points.
    Ellipse,
}

impl ShapeKind {
    /// Whether this shape kind is curvy.
    fn is_round(self) -> bool {
        matches!(self, Self::Circle | Self::Ellipse)
    }

    /// Whether this shape kind has equal side length.
    fn is_quadratic(self) -> bool {
        matches!(self, Self::Square | Self::Circle)
    }
}

/// Layout a shape.
#[allow(clippy::too_many_arguments)]
fn layout_shape(
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    region: Region,
    kind: ShapeKind,
    body: &Option<Content>,
    fill: Option<Paint>,
    stroke: Smart<Sides<Option<Option<Stroke<Abs>>>>>,
    inset: Sides<Option<Rel<Abs>>>,
    outset: Sides<Option<Rel<Abs>>>,
    radius: Corners<Option<Rel<Abs>>>,
    span: Span,
) -> SourceResult<Frame> {
    let mut frame;
    if let Some(child) = body {
        let mut inset = inset.unwrap_or_default();
        if kind.is_round() {
            // Apply extra inset to round shapes.
            inset = inset.map(|v| v + Ratio::new(0.5 - SQRT_2 / 4.0));
        }
        let has_inset = !inset.is_zero();

        // Take the inset, if any, into account.
        let mut pod = region;
        if has_inset {
            pod.size = crate::pad::shrink(region.size, &inset);
        }

        // Layout the child.
        frame = crate::layout_frame(engine, child, locator.relayout(), styles, pod)?;

        // If the child is a square or circle, relayout with full expansion into
        // square region to make sure the result is really quadratic.
        if kind.is_quadratic() {
            let length = frame.size().max_by_side().min(pod.size.min_by_side());
            let quad_pod = Region::new(Size::splat(length), Axes::splat(true));
            frame = crate::layout_frame(engine, child, locator, styles, quad_pod)?;
        }

        // Apply the inset.
        if has_inset {
            crate::pad::grow(&mut frame, &inset);
        }
    } else {
        // The default size that a shape takes on if it has no child and
        // enough space.
        let default = Size::new(Abs::pt(45.0), Abs::pt(30.0));
        let mut size = region.expand.select(region.size, default.min(region.size));
        if kind.is_quadratic() {
            size = Size::splat(size.min_by_side());
        }
        frame = Frame::soft(size);
    }

    // Prepare stroke.
    let stroke = match stroke {
        Smart::Auto if fill.is_none() => Sides::splat(Some(FixedStroke::default())),
        Smart::Auto => Sides::splat(None),
        Smart::Custom(strokes) => {
            strokes.unwrap_or_default().map(|s| s.map(Stroke::unwrap_or_default))
        }
    };

    // Add fill and/or stroke.
    if fill.is_some() || stroke.iter().any(Option::is_some) {
        if kind.is_round() {
            let outset = outset.unwrap_or_default().relative_to(frame.size());
            let size = frame.size() + outset.sum_by_axis();
            let pos = Point::new(-outset.left, -outset.top);
            let shape = Shape {
                geometry: Geometry::Path(Path::ellipse(size)),
                fill,
                stroke: stroke.left,
                fill_rule: FillRule::default(),
            };
            frame.prepend(pos, FrameItem::Shape(shape, span));
        } else {
            fill_and_stroke(
                &mut frame,
                fill,
                &stroke,
                &outset.unwrap_or_default(),
                &radius.unwrap_or_default(),
                span,
            );
        }
    }

    Ok(frame)
}

/// Creates a new rectangle as a path.
pub fn clip_rect(
    size: Size,
    radius: &Corners<Rel<Abs>>,
    stroke: &Sides<Option<FixedStroke>>,
    outset: &Sides<Rel<Abs>>,
) -> Path {
    let outset = outset.relative_to(size);
    let size = size + outset.sum_by_axis();

    let stroke_widths = stroke
        .as_ref()
        .map(|s| s.as_ref().map_or(Abs::zero(), |s| s.thickness / 2.0));

    let max_radius = (size.x.min(size.y)) / 2.0
        + stroke_widths.iter().cloned().min().unwrap_or(Abs::zero());

    let radius = radius.map(|side| side.relative_to(max_radius * 2.0).min(max_radius));
    let corners = corners_control_points(size, &radius, stroke, &stroke_widths);

    let mut path = Path::new();
    if corners.top_left.arc_inner() {
        path.arc_move(
            corners.top_left.start_inner(),
            corners.top_left.center_inner(),
            corners.top_left.end_inner(),
        );
    } else {
        path.move_to(corners.top_left.center_inner());
    }
    for corner in [&corners.top_right, &corners.bottom_right, &corners.bottom_left] {
        if corner.arc_inner() {
            path.arc_line(corner.start_inner(), corner.center_inner(), corner.end_inner())
        } else {
            path.line_to(corner.center_inner());
        }
    }
    path.close_path();
    path.translate(Point::new(-outset.left, -outset.top));
    path
}

/// Add a fill and stroke with optional radius and outset to the frame.
pub fn fill_and_stroke(
    frame: &mut Frame,
    fill: Option<Paint>,
    stroke: &Sides<Option<FixedStroke>>,
    outset: &Sides<Rel<Abs>>,
    radius: &Corners<Rel<Abs>>,
    span: Span,
) {
    let outset = outset.relative_to(frame.size());
    let size = frame.size() + outset.sum_by_axis();
    let pos = Point::new(-outset.left, -outset.top);
    frame.prepend_multiple(
        styled_rect(size, radius, fill, stroke)
            .into_iter()
            .map(|x| (pos, FrameItem::Shape(x, span))),
    );
}

/// Create a styled rectangle with shapes.
/// - use rect primitive for simple rectangles
/// - stroke sides if possible
/// - use fill for sides for best looks
pub fn styled_rect(
    size: Size,
    radius: &Corners<Rel<Abs>>,
    fill: Option<Paint>,
    stroke: &Sides<Option<FixedStroke>>,
) -> Vec<Shape> {
    if stroke.is_uniform() && radius.iter().cloned().all(Rel::is_zero) {
        simple_rect(size, fill, stroke.top.clone())
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
    vec![Shape {
        geometry: Geometry::Rect(size),
        fill,
        stroke,
        fill_rule: FillRule::default(),
    }]
}

fn corners_control_points(
    size: Size,
    radius: &Corners<Abs>,
    strokes: &Sides<Option<FixedStroke>>,
    stroke_widths: &Sides<Abs>,
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
            (Some(a), Some(b)) => a.paint == b.paint && a.dash == b.dash,
            (None, None) => true,
            _ => false,
        },
    })
}

/// Use stroke and fill for the rectangle
fn segmented_rect(
    size: Size,
    radius: &Corners<Rel<Abs>>,
    fill: Option<Paint>,
    strokes: &Sides<Option<FixedStroke>>,
) -> Vec<Shape> {
    let mut res = vec![];
    let stroke_widths = strokes
        .as_ref()
        .map(|s| s.as_ref().map_or(Abs::zero(), |s| s.thickness / 2.0));

    let max_radius = (size.x.min(size.y)) / 2.0
        + stroke_widths.iter().cloned().min().unwrap_or(Abs::zero());

    let radius = radius.map(|side| side.relative_to(max_radius * 2.0).min(max_radius));
    let corners = corners_control_points(size, &radius, strokes, &stroke_widths);

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
            fill_rule: FillRule::default(),
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
            let Some(stroke) = strokes.get_ref(start.side_cw()) else { continue };
            let (shape, ontop) = segment(start, end, &corners, stroke);
            if ontop {
                res.push(shape);
            } else {
                res.insert(stroke_insert, shape);
                stroke_insert += 1;
            }
        }
    } else if let Some(stroke) = &strokes.top {
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
    stroke: &FixedStroke,
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
        .dash
        .as_ref()
        .map(|pattern| pattern.array.is_empty())
        .unwrap_or(true);

    let use_fill = solid && fill_corners(start, end, corners);
    let shape = if use_fill {
        fill_segment(start, end, corners, stroke)
    } else {
        stroke_segment(start, end, corners, stroke.clone())
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
    // Create start corner.
    let mut path = Path::new();
    path_segment(start, end, corners, &mut path);

    Shape {
        geometry: Geometry::Path(path),
        stroke: Some(stroke),
        fill: None,
        fill_rule: FillRule::default(),
    }
}

/// Fill the sides from `start` to `end` clockwise.
fn fill_segment(
    start: Corner,
    end: Corner,
    corners: &Corners<ControlPoints>,
    stroke: &FixedStroke,
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
        fill: Some(stroke.paint.clone()),
        fill_rule: FillRule::default(),
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
        let t = (-b + (b * b - 4.0 * a * c).max(0.0).sqrt()) / (2.0 * a);
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

/// Helper to draw arcs with bezier curves.
trait PathExt {
    fn arc(&mut self, start: Point, center: Point, end: Point);
    fn arc_move(&mut self, start: Point, center: Point, end: Point);
    fn arc_line(&mut self, start: Point, center: Point, end: Point);
}

impl PathExt for Path {
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
