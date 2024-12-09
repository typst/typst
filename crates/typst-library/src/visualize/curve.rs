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
use CurveComponent::*;

/// A path through a list of points, connected by Bezier curves.
///
/// # Example
/// ```example
/// #curve(
///   fill: blue.lighten(80%),
///   stroke: blue,
///   curve.move(start: (0pt, 50pt)),
///   curve.line(end: (100%, 50pt)),
///   curve.cubic(control-end: (50%+40pt, 0pt), end: (50%, 0pt)),
///   curve.close(mode: "curve")
/// )
/// ```
#[elem(scope, Show)]
pub struct CurveElem {
    /// How to fill the path.
    ///
    /// When setting a fill, the default stroke disappears. To create a
    /// rectangle with both fill and stroke, you have to configure both.
    pub fill: Option<Paint>,

    /// The drawing rule used to fill the path.
    ///
    /// ```example
    /// // We use `.with` to get a new
    /// // function that has the common
    /// // arguments pre-applied.
    /// #let star = curve.with(
    ///   fill: red,
    ///   curve.move(start: (25pt, 0pt)),
    ///   curve.line(end: (10pt, 50pt)),
    ///   curve.line(end: (50pt, 20pt)),
    ///   curve.line(end: (0pt, 20pt)),
    ///   curve.line(end: (40pt, 50pt)),
    ///   curve.close()
    /// )
    ///
    /// #star(fill-rule: "non-zero")
    /// #star(fill-rule: "even-odd")
    /// ```
    #[default]
    pub fill_rule: FillRule,

    /// How to [stroke] the path. This can be:
    ///
    /// Can be set to  `{none}` to disable the stroke or to `{auto}` for a
    /// stroke of `{1pt}` black if and if only if no fill is given.
    #[resolve]
    #[fold]
    pub stroke: Smart<Option<Stroke>>,

    /// How to close the path.
    #[default(Some(CloseMode::Curve))]
    pub close_mode: Option<CloseMode>,

    /// The components of the path.
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
    type CurveMoveTo;

    #[elem]
    type CurveLineTo;

    #[elem]
    type CurveQuadraticTo;

    #[elem]
    type CurveCubicTo;

    #[elem]
    type CurveClose;
}

/// A component used for path creation.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum CurveComponent {
    MoveTo(Packed<CurveMoveTo>),
    LineTo(Packed<CurveLineTo>),
    QuadraticTo(Packed<CurveQuadraticTo>),
    CubicTo(Packed<CurveCubicTo>),
    Close(Packed<CurveClose>),
}

cast! {
    CurveComponent,
    self => match self {
        MoveTo(element) => element.into_value(),
        LineTo(element) => element.into_value(),
        QuadraticTo(element) => element.into_value(),
        CubicTo(element) => element.into_value(),
        Close(element) => element.into_value(),
    },
    v: Content => {
        v.try_into()?
    }
}

impl TryFrom<Content> for CurveComponent {
    type Error = HintedString;
    fn try_from(value: Content) -> HintedStrResult<Self> {
        value
            .into_packed::<CurveMoveTo>()
            .map(Self::MoveTo)
            .or_else(|value| value.into_packed::<CurveLineTo>().map(Self::LineTo))
            .or_else(|value| {
                value.into_packed::<CurveQuadraticTo>().map(Self::QuadraticTo)
            })
            .or_else(|value| value.into_packed::<CurveCubicTo>().map(Self::CubicTo))
            .or_else(|value| value.into_packed::<CurveClose>().map(Self::Close))
            .or_else(|_| bail!("expecting a curve element"))
    }
}

/// An element used to start a new path component.
///
/// If no `path.moveto` element is provided, the component will
/// start at `(0pt, 0pt)`.
///
/// If `closed` is `true` in the containing path, previous components
/// will be closed.
#[elem(name = "move", title = "Path Move To")]
pub struct CurveMoveTo {
    /// The starting point for the new component.
    #[resolve]
    pub start: Axes<Rel<Length>>,

    /// Are the coordinates relative to the previous point?
    #[default(false)]
    pub relative: bool,
}

/// An element used to add a segment from the last point to
/// the `end`point.
#[elem(name = "line", title = "Path Line To")]
pub struct CurveLineTo {
    #[resolve]
    pub end: Axes<Rel<Length>>,

    /// Are the coordinates relative to the previous point?
    #[default(false)]
    pub relative: bool,
}

/// An element used to add a quadratic Bezier curve from the last
/// point to `end`, using `control` as the control point.
///
/// If no control point is specified, it defaults to `end`, and
/// the curve will be a straight line.
///
/// If set to `auto` and this curve follows an other quadratic Bezier curve,
/// the previous control point will be mirrored.
#[elem(name = "quadratic", title = "Path Quadratic Curve To")]
pub struct CurveQuadraticTo {
    /// The control point of the Bezier curve.
    #[resolve]
    pub control: Smart<Axes<Rel<Length>>>,

    /// The end point.
    #[resolve]
    pub end: Axes<Rel<Length>>,

    /// Are the coordinates of the `end`and `control` points relative to the previous point?
    #[default(false)]
    pub relative: bool,
}

/// An element used to add a cubic Bezier curve from the last
/// point to `end`, using `cstart` and 'cend' as the control points.
#[elem(name = "cubic", title = "Path Cubic Curve To")]
pub struct CurveCubicTo {
    /// The first control point.
    ///
    /// If set to `auto` and this element follows another `curve.cubic` element,
    /// the last control point will be mirrored.
    ///
    /// Defaults to the last used point.
    #[resolve]
    pub control_start: Smart<Axes<Rel<Length>>>,

    /// The second control point.
    ///
    /// Defaults to the end point.
    #[resolve]
    pub control_end: Axes<Rel<Length>>,

    /// The end point.
    #[resolve]
    pub end: Axes<Rel<Length>>,

    /// Are the coordinates of the `end`and `control` points relative to the previous point?
    pub relative: bool,
}

/// An element used to close a component. A segment from last point to the last `curve.move()`
/// point will be added.
///
/// If the containing path has the `closed` attribute set, all components will
/// be closed anyway.
#[elem(name = "close", title = "Path Close")]
pub struct CurveClose {
    /// How to close the path. If set to `auto`, use the `close-mode` parameter
    /// of the path.
    pub mode: Smart<Option<CloseMode>>,
}

#[derive(Debug, Copy, Clone, Default, Eq, PartialEq, Hash, Cast)]
pub enum CloseMode {
    Line,
    #[default]
    Curve,
}

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

    /// Create a path that describes an axis-aligned ellipse.
    pub fn ellipse(size: Size) -> Self {
        // https://stackoverflow.com/a/2007782
        let z = Abs::zero();
        let rx = size.x / 2.0;
        let ry = size.y / 2.0;
        let m = 0.551784;
        let mx = m * rx;
        let my = m * ry;
        let point = |x, y| Point::new(x + rx, y + ry);

        let mut path = Path::new();
        path.move_to(point(-rx, z));
        path.cubic_to(point(-rx, -my), point(-mx, -ry), point(z, -ry));
        path.cubic_to(point(mx, -ry), point(rx, -my), point(rx, z));
        path.cubic_to(point(rx, my), point(mx, ry), point(z, ry));
        path.cubic_to(point(-mx, ry), point(-rx, my), point(-rx, z));
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

    /// Check if the path is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Translate all points in this path by the given offset.
    pub fn translate(&mut self, offset: Point) {
        if offset.is_zero() {
            return;
        }
        for item in self.0.iter_mut() {
            match item {
                PathItem::MoveTo(p) => *p += offset,
                PathItem::LineTo(p) => *p += offset,
                PathItem::CubicTo(p1, p2, p3) => {
                    *p1 += offset;
                    *p2 += offset;
                    *p3 += offset;
                }
                PathItem::ClosePath => (),
            }
        }
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
