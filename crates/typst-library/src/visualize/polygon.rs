use std::f64::consts::PI;

use typst::eval::{CastInfo, Reflect};

use crate::prelude::*;

/// Display: Regular Polygon
/// Category: visualize
#[func]
pub fn regular_polygon(
    #[named] fill: Option<Paint>,
    #[default(Smart::Auto)]
    #[named]
    stroke: Smart<Option<PartialStroke>>,
    #[default(Smart::Auto)]
    #[named]
    size: Smart<Rel<Length>>,
    #[default(3)]
    #[named]
    vertices: u64,
) -> PolygonElem {
    let origin = size.unwrap_or_default() / 2.0;
    let angle = |i: f64| (2.0 * PI * i / (vertices as f64) + (1.5 * PI));

    let mut v = vec![];
    for i in 0..vertices + 1 {
        let x = origin * angle(i as f64).cos();
        let y = origin * angle(i as f64).sin();
        v.push(Axes::new(x, y));
    }

    PolygonElem::new(v).with_fill(fill).with_stroke(stroke)
}

/// A closed polygon.
///
/// The polygon is defined by its corner points and is closed automatically.
///
/// ## Example { #example }
/// ```example
/// #polygon(
///   fill: blue.lighten(80%),
///   stroke: blue,
///   (20%, 0pt),
///   (60%, 0pt),
///   (80%, 2cm),
///   (0%,  2cm),
/// )
/// ```
///
/// Display: Polygon
/// Category: visualize
#[element(Layout)]
#[scope(scope.define("regular", regular_polygon_func()); scope)]
pub struct PolygonElem {
    /// How to fill the polygon. See the
    /// [rectangle's documentation]($func/rect.fill) for more details.
    ///
    /// Currently all polygons are filled according to the
    /// [non-zero winding rule](https://en.wikipedia.org/wiki/Nonzero-rule).
    pub fill: Option<Paint>,
    pub test: Option<Paint>,

    /// How to stroke the polygon. This can be:
    ///
    /// See the [line's documentation]($func/line.stroke) for more details. Can
    /// be set to  `{none}` to disable the stroke or to `{auto}` for a stroke of
    /// `{1pt}` black if and if only if no fill is given.
    #[resolve]
    #[fold]
    pub stroke: Smart<Option<PartialStroke>>,

    /// The vertices of the polygon. Each point is specified as an array of two
    /// [relative lengths]($type/relative-length).
    #[variadic]
    pub vertices: Vec<Axes<Rel<Length>>>,
}

impl Layout for PolygonElem {
    #[tracing::instrument(name = "PolygonElem::layout", skip_all)]
    fn layout(
        &self,
        _: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let points: Vec<Point> = self
            .vertices()
            .iter()
            .map(|c| {
                c.resolve(styles)
                    .zip(regions.base())
                    .map(|(l, b)| l.relative_to(b))
                    .to_point()
            })
            .collect();

        let size = points.iter().fold(Point::zero(), |max, c| c.max(max)).to_size();
        let mut frame = Frame::new(size);

        // Only create a path if there are more than zero points.
        if points.is_empty() {
            return Ok(Fragment::frame(frame));
        }

        // Prepare fill and stroke.
        let fill = self.fill(styles);
        let stroke = match self.stroke(styles) {
            Smart::Auto if fill.is_none() => Some(Stroke::default()),
            Smart::Auto => None,
            Smart::Custom(stroke) => stroke.map(PartialStroke::unwrap_or_default),
        };

        // Construct a closed path given all points.
        let mut path = Path::new();
        path.move_to(points[0]);
        for &point in &points[1..] {
            path.line_to(point);
        }
        path.close_path();

        let shape = Shape { geometry: Geometry::Path(path), stroke, fill };
        frame.push(Point::zero(), FrameItem::Shape(shape, self.span()));

        Ok(Fragment::frame(frame))
    }
}

impl Reflect for PolygonElem {
    fn describe() -> CastInfo {
        CastInfo::Any
    }

    fn castable(_: &Value) -> bool {
        false
    }
}
