use kurbo::ParamCurveExtrema;
use typst_macros::{Cast, scope};
use typst_utils::Numeric;

use crate::diag::{HintedStrResult, HintedString, bail};
use crate::foundations::{Content, Packed, Smart, cast, elem};
use crate::layout::{Abs, Axes, Length, Point, Rel, Size};
use crate::visualize::{FillRule, Paint, Stroke};

use super::FixedStroke;

/// A curve consisting of movements, lines, and Bézier segments.
///
/// At any point in time, there is a conceptual pen or cursor.
/// - Move elements move the cursor without drawing.
/// - Line/Quadratic/Cubic elements draw a segment from the cursor to a new
///   position, potentially with control point for a Bézier curve.
/// - Close elements draw a straight or smooth line back to the start of the
///   curve or the latest preceding move segment.
///
/// For layout purposes, the bounding box of the curve is a tight rectangle
/// containing all segments as well as the point `{(0pt, 0pt)}`.
///
/// Positions may be specified absolutely (i.e. relatively to `{(0pt, 0pt)}`),
/// or relative to the current pen/cursor position, that is, the position where
/// the previous segment ended.
///
/// Bézier curve control points can be skipped by passing `{none}` or
/// automatically mirrored from the preceding segment by passing `{auto}`.
///
/// # Example
/// ```example
/// #curve(
///   fill: blue.lighten(80%),
///   stroke: blue,
///   curve.move((0pt, 50pt)),
///   curve.line((100pt, 50pt)),
///   curve.cubic(none, (90pt, 0pt), (50pt, 0pt)),
///   curve.close(),
/// )
/// ```
#[elem(scope)]
pub struct CurveElem {
    /// How to fill the curve.
    ///
    /// When setting a fill, the default stroke disappears. To create a
    /// rectangle with both fill and stroke, you have to configure both.
    pub fill: Option<Paint>,

    /// The drawing rule used to fill the curve.
    ///
    /// ```example
    /// // We use `.with` to get a new
    /// // function that has the common
    /// // arguments pre-applied.
    /// #let star = curve.with(
    ///   fill: red,
    ///   curve.move((25pt, 0pt)),
    ///   curve.line((10pt, 50pt)),
    ///   curve.line((50pt, 20pt)),
    ///   curve.line((0pt, 20pt)),
    ///   curve.line((40pt, 50pt)),
    ///   curve.close(),
    /// )
    ///
    /// #star(fill-rule: "non-zero")
    /// #star(fill-rule: "even-odd")
    /// ```
    #[default]
    pub fill_rule: FillRule,

    /// How to [stroke] the curve. This can be:
    ///
    /// Can be set to `{none}` to disable the stroke or to `{auto}` for a
    /// stroke of `{1pt}` black if and if only if no fill is given.
    ///
    /// ```example
    /// #let down = curve.line((40pt, 40pt), relative: true)
    /// #let up = curve.line((40pt, -40pt), relative: true)
    ///
    /// #curve(
    ///   stroke: 4pt + gradient.linear(red, blue),
    ///   down, up, down, up, down,
    /// )
    /// ```
    #[fold]
    pub stroke: Smart<Option<Stroke>>,

    /// The components of the curve, in the form of moves, line and Bézier
    /// segment, and closes.
    #[variadic]
    pub components: Vec<CurveComponent>,
}

#[scope]
impl CurveElem {
    #[elem]
    type CurveMove;

    #[elem]
    type CurveLine;

    #[elem]
    type CurveQuad;

    #[elem]
    type CurveCubic;

    #[elem]
    type CurveClose;
}

/// A component used for curve creation.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum CurveComponent {
    Move(Packed<CurveMove>),
    Line(Packed<CurveLine>),
    Quad(Packed<CurveQuad>),
    Cubic(Packed<CurveCubic>),
    Close(Packed<CurveClose>),
}

cast! {
    CurveComponent,
    self => match self {
        Self::Move(element) => element.into_value(),
        Self::Line(element) => element.into_value(),
        Self::Quad(element) => element.into_value(),
        Self::Cubic(element) => element.into_value(),
        Self::Close(element) => element.into_value(),
    },
    v: Content => {
        v.try_into()?
    }
}

impl TryFrom<Content> for CurveComponent {
    type Error = HintedString;

    fn try_from(value: Content) -> HintedStrResult<Self> {
        value
            .into_packed::<CurveMove>()
            .map(Self::Move)
            .or_else(|value| value.into_packed::<CurveLine>().map(Self::Line))
            .or_else(|value| value.into_packed::<CurveQuad>().map(Self::Quad))
            .or_else(|value| value.into_packed::<CurveCubic>().map(Self::Cubic))
            .or_else(|value| value.into_packed::<CurveClose>().map(Self::Close))
            .or_else(|_| bail!("expecting a curve element"))
    }
}

/// Starts a new curve component.
///
/// If no `curve.move` element is passed, the curve will start at
/// `{(0pt, 0pt)}`.
///
/// ```example
/// #curve(
///   fill: blue.lighten(80%),
///   fill-rule: "even-odd",
///   stroke: blue,
///   curve.line((50pt, 0pt)),
///   curve.line((50pt, 50pt)),
///   curve.line((0pt, 50pt)),
///   curve.close(),
///   curve.move((10pt, 10pt)),
///   curve.line((40pt, 10pt)),
///   curve.line((40pt, 40pt)),
///   curve.line((10pt, 40pt)),
///   curve.close(),
/// )
/// ```
#[elem(name = "move", title = "Curve Move")]
pub struct CurveMove {
    /// The starting point for the new component.
    #[required]
    pub start: Axes<Rel<Length>>,

    /// Whether the coordinates are relative to the previous point.
    #[default(false)]
    pub relative: bool,
}

/// Adds a straight line from the current point to a following one.
///
/// ```example
/// #curve(
///   stroke: blue,
///   curve.line((50pt, 0pt)),
///   curve.line((50pt, 50pt)),
///   curve.line((100pt, 50pt)),
///   curve.line((100pt, 0pt)),
///   curve.line((150pt, 0pt)),
/// )
/// ```
#[elem(name = "line", title = "Curve Line")]
pub struct CurveLine {
    /// The point at which the line shall end.
    #[required]
    pub end: Axes<Rel<Length>>,

    /// Whether the coordinates are relative to the previous point.
    ///
    /// ```example
    /// #curve(
    ///   stroke: blue,
    ///   curve.line((50pt, 0pt), relative: true),
    ///   curve.line((0pt, 50pt), relative: true),
    ///   curve.line((50pt, 0pt), relative: true),
    ///   curve.line((0pt, -50pt), relative: true),
    ///   curve.line((50pt, 0pt), relative: true),
    /// )
    /// ```
    #[default(false)]
    pub relative: bool,
}

/// Adds a quadratic Bézier curve segment from the last point to `end`, using
/// `control` as the control point.
///
/// ```example
/// // Function to illustrate where the control point is.
/// #let mark((x, y)) = place(
///   dx: x - 1pt, dy: y - 1pt,
///   circle(fill: aqua, radius: 2pt),
/// )
///
/// #mark((20pt, 20pt))
///
/// #curve(
///   stroke: blue,
///   curve.move((0pt, 100pt)),
///   curve.quad((20pt, 20pt), (100pt, 0pt)),
/// )
/// ```
#[elem(name = "quad", title = "Curve Quadratic Segment")]
pub struct CurveQuad {
    /// The control point of the quadratic Bézier curve.
    ///
    /// - If `{auto}` and this segment follows another quadratic Bézier curve,
    ///   the previous control point will be mirrored.
    /// - If `{none}`, the control point defaults to `end`, and the curve will
    ///   be a straight line.
    ///
    /// ```example
    /// #curve(
    ///   stroke: 2pt,
    ///   curve.quad((20pt, 40pt), (40pt, 40pt), relative: true),
    ///   curve.quad(auto, (40pt, -40pt), relative: true),
    /// )
    /// ```
    #[required]
    pub control: Smart<Option<Axes<Rel<Length>>>>,

    /// The point at which the segment shall end.
    #[required]
    pub end: Axes<Rel<Length>>,

    /// Whether the `control` and `end` coordinates are relative to the previous
    /// point.
    #[default(false)]
    pub relative: bool,
}

/// Adds a cubic Bézier curve segment from the last point to `end`, using
/// `control-start` and `control-end` as the control points.
///
/// ```example
/// // Function to illustrate where the control points are.
/// #let handle(start, end) = place(
///   line(stroke: red, start: start, end: end)
/// )
///
/// #handle((0pt, 80pt), (10pt, 20pt))
/// #handle((90pt, 60pt), (100pt, 0pt))
///
/// #curve(
///   stroke: blue,
///   curve.move((0pt, 80pt)),
///   curve.cubic((10pt, 20pt), (90pt, 60pt), (100pt, 0pt)),
/// )
/// ```
#[elem(name = "cubic", title = "Curve Cubic Segment")]
pub struct CurveCubic {
    /// The control point going out from the start of the curve segment.
    ///
    /// - If `{auto}` and this element follows another `curve.cubic` element,
    ///   the last control point will be mirrored. In SVG terms, this makes
    ///   `curve.cubic` behave like the `S` operator instead of the `C` operator.
    ///
    /// - If `{none}`, the curve has no first control point, or equivalently,
    ///   the control point defaults to the curve's starting point.
    ///
    /// ```example
    /// #curve(
    ///   stroke: blue,
    ///   curve.move((0pt, 50pt)),
    ///   // - No start control point
    ///   // - End control point at `(20pt, 0pt)`
    ///   // - End point at `(50pt, 0pt)`
    ///   curve.cubic(none, (20pt, 0pt), (50pt, 0pt)),
    ///   // - No start control point
    ///   // - No end control point
    ///   // - End point at `(50pt, 0pt)`
    ///   curve.cubic(none, none, (100pt, 50pt)),
    /// )
    ///
    /// #curve(
    ///   stroke: blue,
    ///   curve.move((0pt, 50pt)),
    ///   curve.cubic(none, (20pt, 0pt), (50pt, 0pt)),
    ///   // Passing `auto` instead of `none` means the start control point
    ///   // mirrors the end control point of the previous curve. Mirror of
    ///   // `(20pt, 0pt)` w.r.t `(50pt, 0pt)` is `(80pt, 0pt)`.
    ///   curve.cubic(auto, none, (100pt, 50pt)),
    /// )
    ///
    /// #curve(
    ///   stroke: blue,
    ///   curve.move((0pt, 50pt)),
    ///   curve.cubic(none, (20pt, 0pt), (50pt, 0pt)),
    ///   // `(80pt, 0pt)` is the same as `auto` in this case.
    ///   curve.cubic((80pt, 0pt), none, (100pt, 50pt)),
    /// )
    /// ```
    #[required]
    pub control_start: Option<Smart<Axes<Rel<Length>>>>,

    /// The control point going into the end point of the curve segment.
    ///
    /// If set to `{none}`, the curve has no end control point, or equivalently,
    /// the control point defaults to the curve's end point.
    #[required]
    pub control_end: Option<Axes<Rel<Length>>>,

    /// The point at which the curve segment shall end.
    #[required]
    pub end: Axes<Rel<Length>>,

    /// Whether the `control-start`, `control-end`, and `end` coordinates are
    /// relative to the previous point.
    #[default(false)]
    pub relative: bool,
}

/// Closes the curve by adding a segment from the last point to the start of the
/// curve (or the last preceding `curve.move` point).
///
/// ```example
/// // We define a function to show the same shape with
/// // both closing modes.
/// #let shape(mode: "smooth") = curve(
///   fill: blue.lighten(80%),
///   stroke: blue,
///   curve.move((0pt, 50pt)),
///   curve.line((100pt, 50pt)),
///   curve.cubic(auto, (90pt, 0pt), (50pt, 0pt)),
///   curve.close(mode: mode),
/// )
///
/// #shape(mode: "smooth")
/// #shape(mode: "straight")
/// ```
#[elem(name = "close", title = "Curve Close")]
pub struct CurveClose {
    /// How to close the curve.
    pub mode: CloseMode,
}

/// How to close a curve.
#[derive(Debug, Copy, Clone, Default, Eq, PartialEq, Hash, Cast)]
pub enum CloseMode {
    /// Closes the curve with a smooth segment that takes into account the
    /// control point opposite the start point.
    #[default]
    Smooth,
    /// Closes the curve with a straight line.
    Straight,
}

/// A curve consisting of movements, lines, and Bézier segments.
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct Curve(pub Vec<CurveItem>);

/// An item in a curve.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum CurveItem {
    Move(Point),
    Line(Point),
    Cubic(Point, Point, Point),
    Close,
}

impl Curve {
    /// Creates an empty curve.
    pub const fn new() -> Self {
        Self(vec![])
    }

    /// Creates a curve that describes a rectangle.
    pub fn rect(size: Size) -> Self {
        let z = Abs::zero();
        let point = Point::new;
        let mut curve = Self::new();
        curve.move_(point(z, z));
        curve.line(point(size.x, z));
        curve.line(point(size.x, size.y));
        curve.line(point(z, size.y));
        curve.close();
        curve
    }

    /// Creates a curve that describes an axis-aligned ellipse.
    pub fn ellipse(size: Size) -> Self {
        // https://stackoverflow.com/a/2007782
        let z = Abs::zero();
        let rx = size.x / 2.0;
        let ry = size.y / 2.0;
        let m = 0.551784;
        let mx = m * rx;
        let my = m * ry;
        let point = |x, y| Point::new(x + rx, y + ry);

        let mut curve = Curve::new();
        curve.move_(point(-rx, z));
        curve.cubic(point(-rx, -my), point(-mx, -ry), point(z, -ry));
        curve.cubic(point(mx, -ry), point(rx, -my), point(rx, z));
        curve.cubic(point(rx, my), point(mx, ry), point(z, ry));
        curve.cubic(point(-mx, ry), point(-rx, my), point(-rx, z));
        curve
    }

    /// Push a [`Move`](CurveItem::Move) item.
    pub fn move_(&mut self, p: Point) {
        self.0.push(CurveItem::Move(p));
    }

    /// Push a [`Line`](CurveItem::Line) item.
    pub fn line(&mut self, p: Point) {
        self.0.push(CurveItem::Line(p));
    }

    /// Push a [`Cubic`](CurveItem::Cubic) item.
    pub fn cubic(&mut self, p1: Point, p2: Point, p3: Point) {
        self.0.push(CurveItem::Cubic(p1, p2, p3));
    }

    /// Push a [`Close`](CurveItem::Close) item.
    pub fn close(&mut self) {
        self.0.push(CurveItem::Close);
    }

    /// Check if the curve is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Translate all points in this curve by the given offset.
    pub fn translate(&mut self, offset: Point) {
        if offset.is_zero() {
            return;
        }
        for item in self.0.iter_mut() {
            match item {
                CurveItem::Move(p) => *p += offset,
                CurveItem::Line(p) => *p += offset,
                CurveItem::Cubic(p1, p2, p3) => {
                    *p1 += offset;
                    *p2 += offset;
                    *p3 += offset;
                }
                CurveItem::Close => (),
            }
        }
    }

    /// Computes the size of the bounding box of this curve.
    pub fn bbox_size(&self) -> Size {
        let mut min = Point::splat(Abs::inf());
        let mut max = Point::splat(-Abs::inf());

        let mut cursor = Point::zero();
        for item in self.0.iter() {
            match item {
                CurveItem::Move(to) => {
                    cursor = *to;
                }
                CurveItem::Line(to) => {
                    min = min.min(cursor);
                    max = max.max(cursor);
                    min = min.min(*to);
                    max = max.max(*to);
                    cursor = *to;
                }
                CurveItem::Cubic(c0, c1, end) => {
                    let cubic = kurbo::CubicBez::new(
                        kurbo::Point::new(cursor.x.to_pt(), cursor.y.to_pt()),
                        kurbo::Point::new(c0.x.to_pt(), c0.y.to_pt()),
                        kurbo::Point::new(c1.x.to_pt(), c1.y.to_pt()),
                        kurbo::Point::new(end.x.to_pt(), end.y.to_pt()),
                    );

                    let bbox = cubic.bounding_box();
                    min.x = min.x.min(Abs::pt(bbox.x0)).min(Abs::pt(bbox.x1));
                    min.y = min.y.min(Abs::pt(bbox.y0)).min(Abs::pt(bbox.y1));
                    max.x = max.x.max(Abs::pt(bbox.x0)).max(Abs::pt(bbox.x1));
                    max.y = max.y.max(Abs::pt(bbox.y0)).max(Abs::pt(bbox.y1));
                    cursor = *end;
                }
                CurveItem::Close => (),
            }
        }

        Size::new(max.x - min.x, max.y - min.y)
    }
}

impl Curve {
    fn to_kurbo(&self) -> impl Iterator<Item = kurbo::PathEl> + '_ {
        use kurbo::PathEl;

        self.0.iter().map(|item| match *item {
            CurveItem::Move(point) => PathEl::MoveTo(point_to_kurbo(point)),
            CurveItem::Line(point) => PathEl::LineTo(point_to_kurbo(point)),
            CurveItem::Cubic(point, point1, point2) => PathEl::CurveTo(
                point_to_kurbo(point),
                point_to_kurbo(point1),
                point_to_kurbo(point2),
            ),
            CurveItem::Close => PathEl::ClosePath,
        })
    }

    /// When this curve is interpreted as a clip mask, would it contain `point`?
    pub fn contains(&self, fill_rule: FillRule, needle: Point) -> bool {
        let kurbo = kurbo::BezPath::from_vec(self.to_kurbo().collect());
        let windings = kurbo::Shape::winding(&kurbo, point_to_kurbo(needle));
        match fill_rule {
            FillRule::NonZero => windings != 0,
            FillRule::EvenOdd => windings % 2 != 0,
        }
    }

    /// When this curve is stroked with `stroke`, would the stroke contain
    /// `point`?
    pub fn stroke_contains(&self, stroke: &FixedStroke, needle: Point) -> bool {
        let width = stroke.thickness.to_raw();
        let cap = match stroke.cap {
            super::LineCap::Butt => kurbo::Cap::Butt,
            super::LineCap::Round => kurbo::Cap::Round,
            super::LineCap::Square => kurbo::Cap::Square,
        };
        let join = match stroke.join {
            super::LineJoin::Miter => kurbo::Join::Miter,
            super::LineJoin::Round => kurbo::Join::Round,
            super::LineJoin::Bevel => kurbo::Join::Bevel,
        };
        let miter_limit = stroke.miter_limit.get();
        let mut style = kurbo::Stroke::new(width)
            .with_caps(cap)
            .with_join(join)
            .with_miter_limit(miter_limit);
        if let Some(dash) = &stroke.dash {
            style = style.with_dashes(
                dash.phase.to_raw(),
                dash.array.iter().copied().map(Abs::to_raw),
            );
        }
        let opts = kurbo::StrokeOpts::default();
        let tolerance = 0.01;
        let expanded = kurbo::stroke(self.to_kurbo(), &style, &opts, tolerance);
        kurbo::Shape::contains(&expanded, point_to_kurbo(needle))
    }
}

fn point_to_kurbo(point: Point) -> kurbo::Point {
    kurbo::Point::new(point.x.to_raw(), point.y.to_raw())
}
