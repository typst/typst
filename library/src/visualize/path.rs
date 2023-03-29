use crate::prelude::*;
use crate::visualize::path::PathVertex::{AllControlPoints, MirroredControlPoint, Vertex};

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
            Vertex(x) => *x,
            MirroredControlPoint(_, a) => a.map(|x| -x),
            AllControlPoints(_, _, b) => *b,
        }
    }

    pub fn control_point_to(&self) -> Axes<Rel<Length>> {
        match self {
            Vertex(x) => *x,
            MirroredControlPoint(_, a) => *a,
            AllControlPoints(_, a, _) => *a,
        }
    }
}

cast_from_value! {
    PathVertex,
    array: Array => {
        let mut iter = array.into_iter();
        match (iter.next(), iter.next(), iter.next(), iter.next()) {
            (Some(a), None, None, None) => {
                Vertex(a.cast()?)
            },
            (Some(a), Some(b), None, None) => {
                if Axes::<Rel<Length>>::is(&a) {
                    MirroredControlPoint(a.cast()?, b.cast()?)
                } else {
                    Vertex(Axes::new(a.cast()?, b.cast()?))
                }
            },
            (Some(a), Some(b), Some(c), None) => {
                AllControlPoints(a.cast()?, b.cast()?, c.cast()?)
            },
            _ => Err("path vertex must be 1, 2, or 3 points")?,
        }
    },
}

cast_to_value! {
    v: PathVertex => {
        match v {
            PathVertex::Vertex(x) => {
                Value::from(x)
            },
            PathVertex::MirroredControlPoint(x, c) => {
                Value::Array(array![x, c])
            },
            PathVertex::AllControlPoints(x, c1, c2) => {
                Value::Array(array![x, c1, c2])
            },
        }
    }
}

/// A path going through a list of points, connected through Bezier curves.
///
/// ## Example
/// ```example
/// #set page(height: 100pt)
/// #path((10%, 10%), ((20%, 20%), (5%, 5%)))
/// #path((10%, 10%), (10%, 15%))
/// ```
///
/// Display: Path
/// Category: visualize
#[element(Layout)]
pub struct PathElem {
    #[default(false)]
    pub closed: bool,

    /// How to stroke the polygon. See the [lines's
    /// documentation]($func/line.stroke) for more details.
    #[resolve]
    #[fold]
    pub stroke: PartialStroke,

    /// The vertices of the path.
    ///
    /// Each vertex can be defined in 3 ways:
    ///
    /// - A regular point, like [line]($func/line)
    /// - An array of two points, the first being the vertex and the second being the control point.
    ///   The control point is expressed relative to the vertex and is mirrored to get the second control point.
    ///   The control point itself refers to the control point that affects the curve coming _into_ this vertex, including for the first point.
    /// - An array of three points, the first being the vertex and the next being the control points (control point for curves coming in and out respectively)
    #[variadic]
    pub vertices: Vec<PathVertex>,
}

impl Layout for PathElem {
    fn layout(&self, _: &mut Vt, styles: StyleChain, regions: Regions) -> SourceResult<Fragment> {
        let resolve = |axes: Axes<Rel<Length>>| {
            axes.resolve(styles).zip(regions.base()).map(|(l, b)| l.relative_to(b)).to_point()
        };

        let vertices: Vec<PathVertex> = self.vertices();
        let points: Vec<Point> = vertices
            .iter()
            .map(|c| resolve(c.vertex()))
            .collect();

        let size = points.iter().fold(Point::zero(), |max, c| c.max(max)).to_size(); // TODO: this is... uh, wrong.
        let target = regions.expand.select(regions.size, size);
        let mut frame = Frame::new(target);

        // Only create a path if there are more than zero points.
        if points.len() > 0 {
            let stroke = Some(self.stroke(styles).unwrap_or_default());

            // Construct a closed path given all points.
            let mut path = Path::new();
            path.move_to(points[0]);
            for (vertex_window, point_window) in vertices.windows(2).zip(points.windows(2)) {
                let from = vertex_window[0];
                let to = vertex_window[1];
                let from_point = point_window[0];
                let to_point = point_window[1];

                let from_control_point = resolve(from.control_point_from()) + from_point;
                let to_control_point = resolve(to.control_point_to()) + to_point;
                path.cubic_to(from_control_point, to_control_point, to_point);
            }

            if self.closed(styles) {
                let from = *vertices.last().unwrap();
                let to = vertices[0]; // We checked that we have at least one element.
                let from_point = *points.last().unwrap();
                let to_point = points[0];

                let from_control_point = resolve(from.control_point_from()) + from_point;
                let to_control_point = resolve(to.control_point_to()) + to_point;
                path.cubic_to(from_control_point, to_control_point, to_point);
            }

            let shape = Shape { geometry: Geometry::Path(path), stroke, fill: None };
            frame.push(Point::zero(), FrameItem::Shape(shape, self.span()));
        }

        Ok(Fragment::frame(frame))
    }
}