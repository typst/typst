use kurbo::ParamCurveExtrema;
use typst_macros::{scope, Cast};
use typst_utils::Numeric;

use crate::diag::{bail, HintedStrResult, HintedString, SourceResult};
use crate::engine::Engine;
use crate::foundations::{
    cast, elem, Content, NativeElement, Packed, Show, Smart, StyleChain,
};
use crate::layout::{Abs, Axes, BlockElem, Length, Point, Rel, Size};
use crate::visualize::{FillRule, Paint, Stroke};

/// A curve consisting of movements, lines, and Bezier segments.
///
/// # Example
/// ```example
/// #curve(
///   fill: blue.lighten(80%),
///   stroke: blue,
///   curve.move((0pt, 50pt)),
///   curve.line((100%, 50pt)),
///   curve.cubic(none, (50%+40pt, 0pt), (50%, 0pt)),
///   curve.close(mode: "curve")
/// )
/// ```
#[elem(scope, Show)]
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
    ///   curve.close()
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
    #[resolve]
    #[fold]
    pub stroke: Smart<Option<Stroke>>,

    /// The components of the curve.
    #[variadic]
    pub components: Vec<CurveComponent>,
}

impl Show for Packed<CurveElem> {
    fn show(&self, engine: &mut Engine, _: StyleChain) -> SourceResult<Content> {
        Ok(BlockElem::single_layouter(self.clone(), engine.routines.layout_curve)
            .pack()
            .spanned(self.span()))
    }
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
#[elem(name = "line", title = "Curve Line")]
pub struct CurveLine {
    /// The point at which the line shall end.
    #[required]
    pub end: Axes<Rel<Length>>,

    /// Whether the coordinates are relative to the previous point.
    #[default(false)]
    pub relative: bool,
}

/// Add a quadratic Bezier curve segment from the last point to `end`, using
/// `control` as the control point.
#[elem(name = "quad", title = "Curve Quadratic Segment")]
pub struct CurveQuad {
    /// The control point of the quadratic Bezier curve.
    ///
    /// - If `{auto}` and this segment follows another quadratic Bezier curve,
    ///   the previous control point will be mirrored.
    /// - If `{none}`, the control point defaults to `end`, and the curve will
    ///   be a straight line.
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

/// Adds a cubic Bezier curve segment from the last point to `end`, using
/// `control-start` and `control-end` as the control points.
#[elem(name = "cubic", title = "Curve Cubic Segment")]
pub struct CurveCubic {
    /// The first control point.
    ///
    /// - If `{auto}` and this element follows another `curve.cubic` element,
    ///   the last control point will be mirrored.
    /// - If `{none}`, defaults to the curve's starting point.
    #[required]
    pub control_start: Option<Smart<Axes<Rel<Length>>>>,

    /// The second control point.
    ///
    /// If set to `{none}`, defaults to the curve's end point.
    #[required]
    pub control_end: Option<Axes<Rel<Length>>>,

    /// The point at which the segment shall end.
    #[required]
    pub end: Axes<Rel<Length>>,

    /// Whether the `control-start`, `control-end`, and `end` coordinates are
    /// relative to the previous point.
    #[default(false)]
    pub relative: bool,
}

/// Closes the curve by adding a segment from the last point to the start of the
/// curve (or the last preceding `curve.move` point).
#[elem(name = "close", title = "Curve Close")]
pub struct CurveClose {
    /// How to close the curve.
    pub mode: CloseMode,
}

/// How to close a curve.
#[derive(Debug, Copy, Clone, Default, Eq, PartialEq, Hash, Cast)]
pub enum CloseMode {
    /// Close the curve with a curved line that takes into account the control
    /// points at the start point.
    #[default]
    Curve,
    /// Close the curve with a straight line.
    Line,
}

/// A curve consisting of movements, lines, and Bezier segments.
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
        let mut min_x = Abs::inf();
        let mut min_y = Abs::inf();
        let mut max_x = -Abs::inf();
        let mut max_y = -Abs::inf();

        let mut cursor = Point::zero();
        for item in self.0.iter() {
            match item {
                CurveItem::Move(to) => {
                    min_x = min_x.min(cursor.x);
                    min_y = min_y.min(cursor.y);
                    max_x = max_x.max(cursor.x);
                    max_y = max_y.max(cursor.y);
                    cursor = *to;
                }
                CurveItem::Line(to) => {
                    min_x = min_x.min(cursor.x);
                    min_y = min_y.min(cursor.y);
                    max_x = max_x.max(cursor.x);
                    max_y = max_y.max(cursor.y);
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
                    min_x = min_x.min(Abs::pt(bbox.x0)).min(Abs::pt(bbox.x1));
                    min_y = min_y.min(Abs::pt(bbox.y0)).min(Abs::pt(bbox.y1));
                    max_x = max_x.max(Abs::pt(bbox.x0)).max(Abs::pt(bbox.x1));
                    max_y = max_y.max(Abs::pt(bbox.y0)).max(Abs::pt(bbox.y1));
                    cursor = *end;
                }
                CurveItem::Close => (),
            }
        }

        Size::new(max_x - min_x, max_y - min_y)
    }
}
