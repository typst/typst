use kurbo::ParamCurveExtrema;

use self::PathVertex::{AllControlPoints, MirroredControlPoint, Vertex};
use crate::diag::{bail, SourceResult};
use crate::engine::Engine;
use crate::foundations::{
    array, cast, elem, Array, Content, NativeElement, Packed, Reflect, Show, Smart,
    StyleChain,
};
use crate::layout::{Abs, Axes, BlockElem, Length, Point, Rel, Size};
use crate::visualize::{FillRule, Paint, Stroke};

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
#[elem(Show)]
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
    /// takes into account the adjacent control points. If you want to close
    /// with a straight line, simply add one last point that's the same as the
    /// start point.
    #[default(false)]
    pub closed: bool,

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
    pub vertices: Vec<PathVertex>,
}

impl Show for Packed<PathElem> {
    fn show(&self, engine: &mut Engine, _: StyleChain) -> SourceResult<Content> {
        Ok(BlockElem::single_layouter(self.clone(), engine.routines.layout_path)
            .pack()
            .spanned(self.span()))
    }
}

/// A component used for path creation.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum PathVertex {
    Vertex(Axes<Rel<Length>>),
    MirroredControlPoint(Axes<Rel<Length>>, Axes<Rel<Length>>),
    AllControlPoints(Axes<Rel<Length>>, Axes<Rel<Length>>, Axes<Rel<Length>>),
}

impl PathVertex {
    pub fn vertex(&self) -> Axes<Rel<Length>> {
        match self {
            Vertex(x) => *x,
            MirroredControlPoint(x, _) => *x,
            AllControlPoints(x, _, _) => *x,
        }
    }

    pub fn control_point_from(&self) -> Axes<Rel<Length>> {
        match self {
            Vertex(_) => Axes::new(Rel::zero(), Rel::zero()),
            MirroredControlPoint(_, a) => a.map(|x| -x),
            AllControlPoints(_, _, b) => *b,
        }
    }

    pub fn control_point_to(&self) -> Axes<Rel<Length>> {
        match self {
            Vertex(_) => Axes::new(Rel::zero(), Rel::zero()),
            MirroredControlPoint(_, a) => *a,
            AllControlPoints(_, a, _) => *a,
        }
    }
}

cast! {
    PathVertex,
    self => match self {
        Vertex(x) => x.into_value(),
        MirroredControlPoint(x, c) => array![x, c].into_value(),
        AllControlPoints(x, c1, c2) => array![x, c1, c2].into_value(),
    },
    array: Array => {
        let mut iter = array.into_iter();
        match (iter.next(), iter.next(), iter.next(), iter.next()) {
            (Some(a), None, None, None) => {
                Vertex(a.cast()?)
            },
            (Some(a), Some(b), None, None) => {
                if Axes::<Rel<Length>>::castable(&a) {
                    MirroredControlPoint(a.cast()?, b.cast()?)
                } else {
                    Vertex(Axes::new(a.cast()?, b.cast()?))
                }
            },
            (Some(a), Some(b), Some(c), None) => {
                AllControlPoints(a.cast()?, b.cast()?, c.cast()?)
            },
            _ => bail!("path vertex must have 1, 2, or 3 points"),
        }
    },
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
