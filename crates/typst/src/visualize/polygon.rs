use std::f64::consts::PI;

use crate::diag::{bail, SourceResult};
use crate::engine::Engine;
use crate::foundations::{
    elem, func, scope, Content, NativeElement, Packed, Resolve, Show, Smart, StyleChain,
};
use crate::introspection::Locator;
use crate::layout::{Axes, BlockElem, Em, Frame, FrameItem, Length, Point, Region, Rel};
use crate::syntax::Span;
use crate::utils::Numeric;
use crate::visualize::{FillRule, FixedStroke, Geometry, Paint, Path, Shape, Stroke};

/// A closed polygon.
///
/// The polygon is defined by its corner points and is closed automatically.
///
/// # Example
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
#[elem(scope, Show)]
pub struct PolygonElem {
    /// How to fill the polygon.
    ///
    /// When setting a fill, the default stroke disappears. To create a
    /// rectangle with both fill and stroke, you have to configure both.
    pub fill: Option<Paint>,

    /// The drawing rule used to fill the polygon.
    ///
    /// See the [path documentation]($path.fill-rule) for an example.
    #[default]
    pub fill_rule: FillRule,

    /// How to [stroke] the polygon. This can be:
    ///
    /// Can be set to  `{none}` to disable the stroke or to `{auto}` for a
    /// stroke of `{1pt}` black if and if only if no fill is given.
    #[resolve]
    #[fold]
    pub stroke: Smart<Option<Stroke>>,

    /// The vertices of the polygon. Each point is specified as an array of two
    /// [relative lengths]($relative).
    #[variadic]
    pub vertices: Vec<Axes<Rel<Length>>>,
}

#[scope]
impl PolygonElem {
    /// A regular polygon, defined by its size and number of vertices.
    ///
    /// ```example
    /// #polygon.regular(
    ///   fill: blue.lighten(80%),
    ///   stroke: blue,
    ///   size: 30pt,
    ///   vertices: 3,
    /// )
    /// ```
    #[func(title = "Regular Polygon")]
    pub fn regular(
        /// The call span of this function.
        span: Span,
        /// How to fill the polygon. See the general
        /// [polygon's documentation]($polygon.fill) for more details.
        #[named]
        fill: Option<Option<Paint>>,

        /// How to stroke the polygon. See the general
        /// [polygon's documentation]($polygon.stroke) for more details.
        #[named]
        stroke: Option<Smart<Option<Stroke>>>,

        /// The diameter of the [circumcircle](https://en.wikipedia.org/wiki/Circumcircle)
        /// of the regular polygon.
        #[named]
        #[default(Em::one().into())]
        size: Length,

        /// The number of vertices in the polygon.
        #[named]
        #[default(3)]
        vertices: u64,
    ) -> Content {
        let radius = size / 2.0;
        let angle = |i: f64| {
            2.0 * PI * i / (vertices as f64) + PI * (1.0 / 2.0 - 1.0 / vertices as f64)
        };
        let (horizontal_offset, vertical_offset) = (0..=vertices)
            .map(|v| {
                (
                    (radius * angle(v as f64).cos()) + radius,
                    (radius * angle(v as f64).sin()) + radius,
                )
            })
            .fold((radius, radius), |(min_x, min_y), (v_x, v_y)| {
                (
                    if min_x < v_x { min_x } else { v_x },
                    if min_y < v_y { min_y } else { v_y },
                )
            });
        let vertices = (0..=vertices)
            .map(|v| {
                let x = (radius * angle(v as f64).cos()) + radius - horizontal_offset;
                let y = (radius * angle(v as f64).sin()) + radius - vertical_offset;
                Axes::new(x, y).map(Rel::from)
            })
            .collect();

        let mut elem = PolygonElem::new(vertices);
        if let Some(fill) = fill {
            elem.push_fill(fill);
        }
        if let Some(stroke) = stroke {
            elem.push_stroke(stroke);
        }
        elem.pack().spanned(span)
    }
}

impl Show for Packed<PolygonElem> {
    fn show(&self, _: &mut Engine, _: StyleChain) -> SourceResult<Content> {
        Ok(BlockElem::single_layouter(self.clone(), layout_polygon)
            .pack()
            .spanned(self.span()))
    }
}

/// Layout the polygon.
#[typst_macros::time(span = elem.span())]
fn layout_polygon(
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
