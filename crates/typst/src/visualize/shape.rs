use std::f64::consts::SQRT_2;

use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{elem, Content, Packed, Resolve, Smart, StyleChain};
use crate::layout::{
    Abs, Axes, Corner, Corners, Frame, FrameItem, LayoutMultiple, LayoutSingle, Length,
    Point, Ratio, Regions, Rel, Sides, Size,
};
use crate::syntax::Span;
use crate::util::Get;
use crate::visualize::{FixedStroke, Paint, Path, Stroke};

/// A rectangle with optional content.
///
/// # Example
/// ```example
/// // Without content.
/// #rect(width: 35%, height: 30pt)
///
/// // With content.
/// #rect[
///   Automatically sized \
///   to fit the content.
/// ]
/// ```
#[elem(title = "Rectangle", LayoutSingle)]
pub struct RectElem {
    /// The rectangle's width, relative to its parent container.
    pub width: Smart<Rel<Length>>,

    /// The rectangle's height, relative to its parent container.
    pub height: Smart<Rel<Length>>,

    /// How to fill the rectangle.
    ///
    /// When setting a fill, the default stroke disappears. To create a
    /// rectangle with both fill and stroke, you have to configure both.
    ///
    /// ```example
    /// #rect(fill: blue)
    /// ```
    pub fill: Option<Paint>,

    /// How to stroke the rectangle. This can be:
    ///
    /// - `{none}` to disable stroking
    /// - `{auto}` for a stroke of `{1pt + black}` if and if only if no fill is
    ///   given.
    /// - Any kind of [stroke]
    /// - A dictionary describing the stroke for each side inidvidually. The
    ///   dictionary can contain the following keys in order of precedence:
    ///   - `top`: The top stroke.
    ///   - `right`: The right stroke.
    ///   - `bottom`: The bottom stroke.
    ///   - `left`: The left stroke.
    ///   - `x`: The horizontal stroke.
    ///   - `y`: The vertical stroke.
    ///   - `rest`: The stroke on all sides except those for which the
    ///     dictionary explicitly sets a size.
    ///
    /// ```example
    /// #stack(
    ///   dir: ltr,
    ///   spacing: 1fr,
    ///   rect(stroke: red),
    ///   rect(stroke: 2pt),
    ///   rect(stroke: 2pt + red),
    /// )
    /// ```
    #[resolve]
    #[fold]
    pub stroke: Smart<Sides<Option<Option<Stroke>>>>,

    /// How much to round the rectangle's corners, relative to the minimum of
    /// the width and height divided by two. This can be:
    ///
    /// - A relative length for a uniform corner radius.
    /// - A dictionary: With a dictionary, the stroke for each side can be set
    ///   individually. The dictionary can contain the following keys in order
    ///   of precedence:
    ///   - `top-left`: The top-left corner radius.
    ///   - `top-right`: The top-right corner radius.
    ///   - `bottom-right`: The bottom-right corner radius.
    ///   - `bottom-left`: The bottom-left corner radius.
    ///   - `left`: The top-left and bottom-left corner radii.
    ///   - `top`: The top-left and top-right corner radii.
    ///   - `right`: The top-right and bottom-right corner radii.
    ///   - `bottom`: The bottom-left and bottom-right corner radii.
    ///   - `rest`: The radii for all corners except those for which the
    ///     dictionary explicitly sets a size.
    ///
    /// ```example
    /// #set rect(stroke: 4pt)
    /// #rect(
    ///   radius: (
    ///     left: 5pt,
    ///     top-right: 20pt,
    ///     bottom-right: 10pt,
    ///   ),
    ///   stroke: (
    ///     left: red,
    ///     top: yellow,
    ///     right: green,
    ///     bottom: blue,
    ///   ),
    /// )
    /// ```
    #[resolve]
    #[fold]
    pub radius: Corners<Option<Rel<Length>>>,

    /// How much to pad the rectangle's content.
    /// See the [box's documentation]($box.outset) for more details.
    #[resolve]
    #[fold]
    #[default(Sides::splat(Some(Abs::pt(5.0).into())))]
    pub inset: Sides<Option<Rel<Length>>>,

    /// How much to expand the rectangle's size without affecting the layout.
    /// See the [box's documentation]($box.outset) for more details.
    #[resolve]
    #[fold]
    pub outset: Sides<Option<Rel<Length>>>,

    /// The content to place into the rectangle.
    ///
    /// When this is omitted, the rectangle takes on a default size of at most
    /// `{45pt}` by `{30pt}`.
    #[positional]
    pub body: Option<Content>,
}

impl LayoutSingle for Packed<RectElem> {
    #[typst_macros::time(name = "rect", span = self.span())]
    fn layout(
        &self,
        engine: &mut Engine,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Frame> {
        layout(
            engine,
            styles,
            regions,
            ShapeKind::Rect,
            &self.body(styles),
            Axes::new(self.width(styles), self.height(styles)),
            self.fill(styles),
            self.stroke(styles),
            self.inset(styles),
            self.outset(styles),
            self.radius(styles),
            self.span(),
        )
    }
}

/// A square with optional content.
///
/// # Example
/// ```example
/// // Without content.
/// #square(size: 40pt)
///
/// // With content.
/// #square[
///   Automatically \
///   sized to fit.
/// ]
/// ```
#[elem(LayoutSingle)]
pub struct SquareElem {
    /// The square's side length. This is mutually exclusive with `width` and
    /// `height`.
    #[external]
    pub size: Smart<Length>,

    /// The square's width. This is mutually exclusive with `size` and `height`.
    ///
    /// In contrast to `size`, this can be relative to the parent container's
    /// width.
    #[parse(
        let size = args.named::<Smart<Length>>("size")?.map(|s| s.map(Rel::from));
        match size {
            None => args.named("width")?,
            size => size,
        }
    )]
    pub width: Smart<Rel<Length>>,

    /// The square's height. This is mutually exclusive with `size` and `width`.
    ///
    /// In contrast to `size`, this can be relative to the parent container's
    /// height.
    #[parse(match size {
        None => args.named("height")?,
        size => size,
    })]
    pub height: Smart<Rel<Length>>,

    /// How to fill the square. See the [rectangle's documentation]($rect.fill)
    /// for more details.
    pub fill: Option<Paint>,

    /// How to stroke the square. See the
    /// [rectangle's documentation]($rect.stroke) for more details.
    #[resolve]
    #[fold]
    pub stroke: Smart<Sides<Option<Option<Stroke>>>>,

    /// How much to round the square's corners. See the
    /// [rectangle's documentation]($rect.radius) for more details.
    #[resolve]
    #[fold]
    pub radius: Corners<Option<Rel<Length>>>,

    /// How much to pad the square's content. See the
    /// [box's documentation]($box.inset) for more details.
    #[resolve]
    #[fold]
    #[default(Sides::splat(Some(Abs::pt(5.0).into())))]
    pub inset: Sides<Option<Rel<Length>>>,

    /// How much to expand the square's size without affecting the layout. See
    /// the [box's documentation]($box.outset) for more details.
    #[resolve]
    #[fold]
    pub outset: Sides<Option<Rel<Length>>>,

    /// The content to place into the square. The square expands to fit this
    /// content, keeping the 1-1 aspect ratio.
    ///
    /// When this is omitted, the square takes on a default size of at most
    /// `{30pt}`.
    #[positional]
    pub body: Option<Content>,
}

impl LayoutSingle for Packed<SquareElem> {
    #[typst_macros::time(name = "square", span = self.span())]
    fn layout(
        &self,
        engine: &mut Engine,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Frame> {
        layout(
            engine,
            styles,
            regions,
            ShapeKind::Square,
            &self.body(styles),
            Axes::new(self.width(styles), self.height(styles)),
            self.fill(styles),
            self.stroke(styles),
            self.inset(styles),
            self.outset(styles),
            self.radius(styles),
            self.span(),
        )
    }
}

/// An ellipse with optional content.
///
/// # Example
/// ```example
/// // Without content.
/// #ellipse(width: 35%, height: 30pt)
///
/// // With content.
/// #ellipse[
///   #set align(center)
///   Automatically sized \
///   to fit the content.
/// ]
/// ```
#[elem(LayoutSingle)]
pub struct EllipseElem {
    /// The ellipse's width, relative to its parent container.
    pub width: Smart<Rel<Length>>,

    /// The ellipse's height, relative to its parent container.
    pub height: Smart<Rel<Length>>,

    /// How to fill the ellipse. See the [rectangle's documentation]($rect.fill)
    /// for more details.
    pub fill: Option<Paint>,

    /// How to stroke the ellipse. See the
    /// [rectangle's documentation]($rect.stroke) for more details.
    #[resolve]
    #[fold]
    pub stroke: Smart<Option<Stroke>>,

    /// How much to pad the ellipse's content. See the
    /// [box's documentation]($box.inset) for more details.
    #[resolve]
    #[fold]
    #[default(Sides::splat(Some(Abs::pt(5.0).into())))]
    pub inset: Sides<Option<Rel<Length>>>,

    /// How much to expand the ellipse's size without affecting the layout. See
    /// the [box's documentation]($box.outset) for more details.
    #[resolve]
    #[fold]
    pub outset: Sides<Option<Rel<Length>>>,

    /// The content to place into the ellipse.
    ///
    /// When this is omitted, the ellipse takes on a default size of at most
    /// `{45pt}` by `{30pt}`.
    #[positional]
    pub body: Option<Content>,
}

impl LayoutSingle for Packed<EllipseElem> {
    #[typst_macros::time(name = "ellipse", span = self.span())]
    fn layout(
        &self,
        engine: &mut Engine,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Frame> {
        layout(
            engine,
            styles,
            regions,
            ShapeKind::Ellipse,
            &self.body(styles),
            Axes::new(self.width(styles), self.height(styles)),
            self.fill(styles),
            self.stroke(styles).map(|s| Sides::splat(Some(s))),
            self.inset(styles),
            self.outset(styles),
            Corners::splat(None),
            self.span(),
        )
    }
}

/// A circle with optional content.
///
/// # Example
/// ```example
/// // Without content.
/// #circle(radius: 25pt)
///
/// // With content.
/// #circle[
///   #set align(center + horizon)
///   Automatically \
///   sized to fit.
/// ]
/// ```
#[elem(LayoutSingle)]
pub struct CircleElem {
    /// The circle's radius. This is mutually exclusive with `width` and
    /// `height`.
    #[external]
    pub radius: Length,

    /// The circle's width. This is mutually exclusive with `radius` and
    /// `height`.
    ///
    /// In contrast to `radius`, this can be relative to the parent container's
    /// width.
    #[parse(
        let size = args
            .named::<Smart<Length>>("radius")?
            .map(|s| s.map(|r| 2.0 * Rel::from(r)));
        match size {
            None => args.named("width")?,
            size => size,
        }
    )]
    pub width: Smart<Rel<Length>>,

    /// The circle's height. This is mutually exclusive with `radius` and
    /// `width`.
    ///
    /// In contrast to `radius`, this can be relative to the parent container's
    /// height.
    #[parse(match size {
        None => args.named("height")?,
        size => size,
    })]
    pub height: Smart<Rel<Length>>,

    /// How to fill the circle. See the [rectangle's documentation]($rect.fill)
    /// for more details.
    pub fill: Option<Paint>,

    /// How to stroke the circle. See the
    /// [rectangle's documentation]($rect.stroke) for more details.
    #[resolve]
    #[fold]
    #[default(Smart::Auto)]
    pub stroke: Smart<Option<Stroke>>,

    /// How much to pad the circle's content. See the
    /// [box's documentation]($box.inset) for more details.
    #[resolve]
    #[fold]
    #[default(Sides::splat(Some(Abs::pt(5.0).into())))]
    pub inset: Sides<Option<Rel<Length>>>,

    /// How much to expand the circle's size without affecting the layout. See
    /// the [box's documentation]($box.outset) for more details.
    #[resolve]
    #[fold]
    pub outset: Sides<Option<Rel<Length>>>,

    /// The content to place into the circle. The circle expands to fit this
    /// content, keeping the 1-1 aspect ratio.
    #[positional]
    pub body: Option<Content>,
}

impl LayoutSingle for Packed<CircleElem> {
    #[typst_macros::time(name = "circle", span = self.span())]
    fn layout(
        &self,
        engine: &mut Engine,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Frame> {
        layout(
            engine,
            styles,
            regions,
            ShapeKind::Circle,
            &self.body(styles),
            Axes::new(self.width(styles), self.height(styles)),
            self.fill(styles),
            self.stroke(styles).map(|s| Sides::splat(Some(s))),
            self.inset(styles),
            self.outset(styles),
            Corners::splat(None),
            self.span(),
        )
    }
}

/// Layout a shape.
#[allow(clippy::too_many_arguments)]
fn layout(
    engine: &mut Engine,
    styles: StyleChain,
    regions: Regions,
    kind: ShapeKind,
    body: &Option<Content>,
    sizing: Axes<Smart<Rel<Length>>>,
    fill: Option<Paint>,
    stroke: Smart<Sides<Option<Option<Stroke<Abs>>>>>,
    inset: Sides<Option<Rel<Abs>>>,
    outset: Sides<Option<Rel<Abs>>>,
    radius: Corners<Option<Rel<Abs>>>,
    span: Span,
) -> SourceResult<Frame> {
    let resolved = sizing
        .zip_map(regions.base(), |s, r| s.map(|v| v.resolve(styles).relative_to(r)));

    let mut frame;
    let mut inset = inset.unwrap_or_default();

    if let Some(child) = body {
        let region = resolved.unwrap_or(regions.base());

        if kind.is_round() {
            inset = inset.map(|side| side + Ratio::new(0.5 - SQRT_2 / 4.0));
        }

        // Pad the child.
        let child = child.clone().padded(inset.map(|side| side.map(Length::from)));
        let expand = sizing.as_ref().map(Smart::is_custom);
        let pod = Regions::one(region, expand);
        frame = child.layout(engine, styles, pod)?.into_frame();

        // Enforce correct size.
        *frame.size_mut() = expand.select(region, frame.size());

        // Relayout with full expansion into square region to make sure
        // the result is really a square or circle.
        if kind.is_quadratic() {
            frame.set_size(Size::splat(frame.size().max_by_side()));
            let length = frame.size().max_by_side().min(region.min_by_side());
            let pod = Regions::one(Size::splat(length), Axes::splat(true));
            frame = child.layout(engine, styles, pod)?.into_frame();
        }

        // Enforce correct size again.
        *frame.size_mut() = expand.select(region, frame.size());
        if kind.is_quadratic() {
            frame.set_size(Size::splat(frame.size().max_by_side()));
        }
    } else {
        // The default size that a shape takes on if it has no child and
        // enough space.
        let default = Size::new(Abs::pt(45.0), Abs::pt(30.0));
        let mut size = resolved.unwrap_or(default.min(regions.base()));
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
            let shape = ellipse(size, fill, stroke.left);
            frame.prepend(pos, FrameItem::Shape(shape, span));
        } else {
            frame.fill_and_stroke(
                fill,
                stroke,
                outset.unwrap_or_default(),
                radius.unwrap_or_default(),
                span,
            );
        }
    }

    Ok(frame)
}

/// A category of shape.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ShapeKind {
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

/// A geometric shape with optional fill and stroke.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Shape {
    /// The shape's geometry.
    pub geometry: Geometry,
    /// The shape's background fill.
    pub fill: Option<Paint>,
    /// The shape's border stroke.
    pub stroke: Option<FixedStroke>,
}

/// A shape's geometry.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Geometry {
    /// A line to a point (relative to its position).
    Line(Point),
    /// A rectangle with its origin in the topleft corner.
    Rect(Size),
    /// A bezier path.
    Path(Path),
}

impl Geometry {
    /// Fill the geometry without a stroke.
    pub fn filled(self, fill: Paint) -> Shape {
        Shape { geometry: self, fill: Some(fill), stroke: None }
    }

    /// Stroke the geometry without a fill.
    pub fn stroked(self, stroke: FixedStroke) -> Shape {
        Shape { geometry: self, fill: None, stroke: Some(stroke) }
    }

    /// The bounding box of the geometry.
    pub fn bbox_size(&self) -> Size {
        match self {
            Self::Line(line) => Size::new(line.x, line.y),
            Self::Rect(s) => *s,
            Self::Path(p) => p.bbox_size(),
        }
    }
}

/// Produce a shape that approximates an axis-aligned ellipse.
pub(crate) fn ellipse(
    size: Size,
    fill: Option<Paint>,
    stroke: Option<FixedStroke>,
) -> Shape {
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

    Shape { geometry: Geometry::Path(path), stroke, fill }
}

/// Creates a new rectangle as a path.
pub(crate) fn clip_rect(
    size: Size,
    radius: Corners<Rel<Abs>>,
    stroke: &Sides<Option<FixedStroke>>,
) -> Path {
    let stroke_widths = stroke
        .as_ref()
        .map(|s| s.as_ref().map_or(Abs::zero(), |s| s.thickness / 2.0));

    let max_radius = (size.x.min(size.y)) / 2.0
        + stroke_widths.iter().cloned().min().unwrap_or(Abs::zero());

    let radius = radius.map(|side| side.relative_to(max_radius * 2.0).min(max_radius));

    let corners = corners_control_points(size, radius, stroke, stroke_widths);

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
    path
}

/// Create a styled rectangle with shapes.
/// - use rect primitive for simple rectangles
/// - stroke sides if possible
/// - use fill for sides for best looks
pub(crate) fn styled_rect(
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
            (Some(a), Some(b)) => a.paint == b.paint && a.dash == b.dash,
            (None, None) => true,
            _ => false,
        },
    })
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
        .dash
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
