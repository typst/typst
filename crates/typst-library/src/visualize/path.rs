use kurbo::ParamCurveExtrema;
use typst_macros::{scope, Cast};
use typst_utils::Numeric;

use crate::diag::{bail, HintedStrResult, HintedString, SourceResult};
use crate::engine::Engine;
use crate::foundations::{
    array, cast, elem, Array, Content, NativeElement, Packed, Reflect, Show, Smart,
    StyleChain,
};
use crate::layout::{Abs, Axes, BlockElem, Length, Point, Rel, Size};
use crate::visualize::{FillRule, Paint, Stroke};
use PathComponent::*;

/// A path through a list of points, connected by Bezier curves.
///
/// # Example
/// ```example
/// #path(
///   fill: blue.lighten(80%),
///   stroke: blue,
///   closed: true,
///   (0pt, 50pt),
///   (100%, 50pt),
///   ((50%, 0pt), (40pt, 0pt)),
/// )
/// ```
#[elem(scope, Show)]
pub struct PathElem {
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
    /// #let star = path.with(
    ///   fill: red,
    ///   closed: true,
    ///   (25pt, 0pt),
    ///   (10pt, 50pt),
    ///   (50pt, 20pt),
    ///   (0pt, 20pt),
    ///   (40pt, 50pt),
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

    /// Whether to close this path with one last bezier curve. This curve will
    /// take into account the adjacent control points. If you want to close
    /// with a straight line, simply add one last point that's the same as the
    /// start point.
    #[default(false)]
    pub closed: bool,

    /// How to close the path.
    #[default(Some(CloseMode::Curve))]
    pub close_mode: Option<CloseMode>,

    /// The vertices of the path.
    ///
    /// Each vertex can be defined in 3 ways:
    ///
    /// - A regular point, as given to the [`line`] or [`polygon`] function.
    /// - An array of two points, the first being the vertex and the second
    ///   being the control point. The control point is expressed relative to
    ///   the vertex and is mirrored to get the second control point. The given
    ///   control point is the one that affects the curve coming _into_ this
    ///   vertex (even for the first point). The mirrored control point affects
    ///   the curve going out of this vertex.
    /// - An array of three points, the first being the vertex and the next
    ///   being the control points (control point for curves coming in and out,
    ///   respectively).
    #[variadic]
    pub vertices: Vec<PathComponent>,
}

impl Show for Packed<PathElem> {
    fn show(&self, engine: &mut Engine, _: StyleChain) -> SourceResult<Content> {
        Ok(BlockElem::single_layouter(self.clone(), engine.routines.layout_path)
            .pack()
            .spanned(self.span()))
    }
}

#[scope]
impl PathElem {
    #[elem]
    type PathVertex;

    #[elem]
    type PathMoveTo;

    #[elem]
    type PathLineTo;

    #[elem]
    type PathQuadraticTo;

    #[elem]
    type PathCubicTo;

    #[elem]
    type PathClose;
}

/// A component used for path creation.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum PathComponent {
    /// Old style syntax.
    SimplePoint(Axes<Rel<Length>>),
    MirroredControlPoint(Axes<Rel<Length>>, Axes<Rel<Length>>),
    AllControlPoints(Axes<Rel<Length>>, Axes<Rel<Length>>, Axes<Rel<Length>>),
    /// New style syntax.
    Vertex(Packed<PathVertex>),
    MoveTo(Packed<PathMoveTo>),
    LineTo(Packed<PathLineTo>),
    QuadraticTo(Packed<PathQuadraticTo>),
    CubicTo(Packed<PathCubicTo>),
    ClosePath(Packed<PathClose>),
}

cast! {
    PathComponent,
    self => match self {
        SimplePoint(x) => x.into_value(),
        MirroredControlPoint(x, c) => array![x, c].into_value(),
        AllControlPoints(x, c1, c2) => array![x, c1, c2].into_value(),
        Vertex(element) => element.into_value(),
        MoveTo(element) => element.into_value(),
        LineTo(element) => element.into_value(),
        QuadraticTo(element) => element.into_value(),
        CubicTo(element) => element.into_value(),
        ClosePath(element) => element.into_value(),
    },
    array: Array => {
        let mut iter = array.into_iter();
        match (iter.next(), iter.next(), iter.next(), iter.next()) {
            (Some(a), None, None, None) => {
                SimplePoint(a.cast()?)
            },
            (Some(a), Some(b), None, None) => {
                if Axes::<Rel<Length>>::castable(&a) {
                    MirroredControlPoint(a.cast()?, b.cast()?)
                } else {
                    SimplePoint(Axes::new(a.cast()?, b.cast()?))
                }
            },
            (Some(a), Some(b), Some(c), None) => {
                AllControlPoints(a.cast()?, b.cast()?, c.cast()?)
            },
            _ => bail!("path vertex must have 1, 2, or 3 points"),
        }
    },
    v: Content => {
        v.try_into()?
    }
}

impl TryFrom<Content> for PathComponent {
    type Error = HintedString;
    fn try_from(value: Content) -> HintedStrResult<Self> {
        value
            .into_packed::<PathVertex>()
            .map(Self::Vertex)
            .or_else(|value| value.into_packed::<PathMoveTo>().map(Self::MoveTo))
            .or_else(|value| value.into_packed::<PathLineTo>().map(Self::LineTo))
            .or_else(|value| {
                value.into_packed::<PathQuadraticTo>().map(Self::QuadraticTo)
            })
            .or_else(|value| value.into_packed::<PathCubicTo>().map(Self::CubicTo))
            .or_else(|value| value.into_packed::<PathClose>().map(Self::ClosePath))
            .or_else(|_| bail!("expecting a path element"))
    }
}

/// An element used to define a vertex and its control points.
///
/// - `point`
/// - `control-into` controls the curve coming into this vertex.
/// - `control-from` controls the curve coming out of this vertex. If set
///    to `auto`, the `control-into` point is mirrored.
/// - If `relative` is set, the vertex
/// Control points are defined relative to the vertex.
///
#[elem(name = "vertex", title = "Vertex with control points")]
pub struct PathVertex {
    /// Position of the vertex.
    #[resolve]
    pub point: Axes<Rel<Length>>,

    /// Control point affecting the curve coming into the vertex.
    /// Relative to the vertex.
    #[resolve]
    pub control_into: Axes<Rel<Length>>,

    /// Control point affecting the curve coming from the vertex.
    /// Relative to the vertex.
    /// If set to `auto`, the other control point is mirrored.
    #[resolve]
    pub control_from: Smart<Axes<Rel<Length>>>,

    /// Is the point relative to the previous vertex?
    #[default(false)]
    pub relative: bool,
}

/// An element used to start a new path component.
///
/// If no `path.moveto` element is provided, the component will
/// start at `(0pt, 0pt)`.
///
/// If `closed` is `true` in the containing path, previous components
/// will be closed.
#[elem(name = "move", title = "Path Move To")]
pub struct PathMoveTo {
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
pub struct PathLineTo {
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
pub struct PathQuadraticTo {
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
pub struct PathCubicTo {
    /// The first control point.
    ///
    /// If set to `auto` and this element follows another `path.cubicto` element,
    /// the last control point will be mirrored.
    ///
    /// Defaults to the last used point.
    #[resolve]
    pub cstart: Smart<Axes<Rel<Length>>>,

    /// The second control point.
    ///
    /// Defaults to the end point.
    #[resolve]
    pub cend: Axes<Rel<Length>>,

    /// The end point.
    #[resolve]
    pub end: Axes<Rel<Length>>,

    /// Are the coordinates of the `end`and `control` points relative to the previous point?
    pub relative: bool,
}

/// An element used to close a component. A segment from last point to the last `path.moveto()`
/// point will be added.
///
/// If the containing path has the `closed` attribute set, all components will
/// be closed anyway.
#[elem(name = "close", title = "Path Close")]
pub struct PathClose {
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
