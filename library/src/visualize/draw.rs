use crate::prelude::*;

/// A module with all the drawing operations.
pub fn draw() -> crate::Module {
    let mut draw = crate::Scope::deduplicating();
    draw.define("close", close);
    draw.define("moveto", moveto);
    draw.define("lineto", lineto);
    draw.define("cubicto", cubicto);
    draw.define("arcto", arcto);
    draw.define("circle", circle);
    draw.define("path", DrawPathElem::func());

    crate::Module::new("draw").with_scope(draw)
}

/// Draws a path with specified fill and stroke.
///
/// The following path operations can be used to draw the path, note that they are all in
/// the `draw` module:
/// - [`close`]($func/draw.close)
/// - [`moveto`]($func/draw.moveto)
/// - [`lineto`]($func/draw.lineto)
/// - [`cubicto`]($func/draw.cubicto)
/// - [`arcto`]($func/draw.arcto)
/// - [`circle`]($func/draw.circle)
///
/// ## Example
/// ```example
/// #draw.path(stroke: blue, fill: blue.lighten(80%), {
///   import draw: *
///   moveto(y: 10pt)
///   for ms in (1, -1) {
///     for sxy in ((1, -1), (-1, 1)) {
///       let sx = sxy.at(0)
///       let sy = sxy.at(1)
///       let i = 0
///       while i < 4 {
///         lineto(dx: ms*sx*10pt, dy: ms*sy*10pt)
///         lineto(dx: ms*10pt, dy: ms*10pt)
///         i += 1
///       }
///     }
///   }
///   close()
///   circle(x: 40pt, y: 50pt, r: 15pt, ccw: false)
/// })
/// ```
///
/// Display: Path
/// Category: visualize
#[element(Layout)]
pub struct DrawPathElem {
    /// How to fill the path. See the
    /// [rectangle's documentation]($func/rect.fill) for more details.
    ///
    /// Currently all polygons are filled according to the
    /// [non-zero winding rule](https://en.wikipedia.org/wiki/Nonzero-rule).
    pub fill: Option<Paint>,
    /// How to stroke the path. See the [lines's
    /// documentation]($func/line.stroke) for more details.
    #[resolve]
    #[fold]
    #[default(Some(PartialStroke::default()))]
    pub stroke: Option<PartialStroke>,
    /// Whether to close the path.
    #[default]
    pub closed: bool,
    #[required]
    pub path: PathBuilder,
}

impl Layout for DrawPathElem {
    fn layout(
        &self,
        _: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let mut path = self.path().resolve(styles).to_path(regions.base());
        if self.closed(styles) {
            path.close_path();
        }
        let fill = self.fill(styles);
        let stroke = self.stroke(styles).map(PartialStroke::unwrap_or_default);

        let target = regions.expand.select(regions.size, path.size());

        let mut frame = Frame::new(target);
        let shape = Shape { geometry: Geometry::Path(path), stroke, fill };
        frame.push(Point::zero(), FrameItem::Shape(shape, self.span()));
        Ok(Fragment::frame(frame))
    }
}

#[func]
/// A path segment that closes the current path.
///
/// Display: Close
/// Category: visualize
/// Returns: path
pub fn close() -> Value {
    let mut path: PathBuilder = PathBuilder::new();
    path.close_path();
    path.into()
}

/// A path segment with a single move to operation.
///
/// Display: MoveTo
/// Category: visualize
/// Returns: path
#[func]
pub fn moveto(
    /// The x coordinate to move to.
    #[named]
    x: Option<Rel<Length>>,
    /// The y coordinate to move to.
    #[named]
    y: Option<Rel<Length>>,
    /// The delta distance to move in the x coordinate. Cannot be used together with `x`.
    #[named]
    #[default(Spanned::new(None, Span::detached()))]
    dx: Spanned<Option<Rel<Length>>>,
    /// The delta distance to move in the y coordinate. Cannot be used together with `y`.
    #[named]
    #[default(Spanned::new(None, Span::detached()))]
    dy: Spanned<Option<Rel<Length>>>,
    /// The length to move in the direction of `angle`. Cannot be used with `x`, `dx`, `y` or `dy`.
    #[named]
    #[default(Spanned::new(None, Span::detached()))]
    length: Spanned<Option<Rel<Length>>>,
    /// The angle to move in. Is only considered if `length` is set.
    #[named]
    #[default]
    angle: Angle,
) -> Value {
    if x.is_some() && dx.v.is_some() {
        bail!(dx.span, "cannot have both a `x` and `dx` argument")
    }
    if y.is_some() && dy.v.is_some() {
        bail!(dy.span, "cannot have both a `y` and `dy` argument")
    }

    let x_val = x.map(DeltaAbs::Abs).or(dx.v.map(DeltaAbs::Delta));
    let y_val = y.map(DeltaAbs::Abs).or(dy.v.map(DeltaAbs::Delta));

    let (x, y) = if let Some(len) = length.v {
        if x_val.is_some() || y_val.is_some() {
            bail!(length.span, "cannot have both a `x`, `dx`, `y` or `dy` at the same time as a `length` argument")
        }

        let x = angle.cos() * len;
        let y = angle.sin() * len;

        (DeltaAbs::Delta(x), DeltaAbs::Delta(y))
    } else {
        (
            x_val.unwrap_or(DeltaAbs::Delta(Rel::zero())),
            y_val.unwrap_or(DeltaAbs::Delta(Rel::zero())),
        )
    };

    let mut path = PathBuilder::new();
    path.move_to(Axes::new(x, y));
    path.into()
}

/// A path with a line segment.
///
/// ## Example
/// ```example
/// #draw.path(stroke: blue, #draw.line(length: 100%))
/// #draw.path(stroke: blue, #draw.line(x: 50%, y: 50%))
/// ```
///
/// Display: LineTo
/// Category: visualize
/// Returns: path
#[func]
pub fn lineto(
    /// The x coordinate for the line endpoint.
    #[named]
    x: Option<Rel<Length>>,
    /// The y coordinate for the line endpoint.
    #[named]
    y: Option<Rel<Length>>,
    /// The delta distance of the line in the x coordinate. Cannot be used together with `x`.
    #[named]
    #[default(Spanned::new(None, Span::detached()))]
    dx: Spanned<Option<Rel<Length>>>,
    /// The delta distance of the line in the y coordinate. Cannot be used together with `y`.
    #[named]
    #[default(Spanned::new(None, Span::detached()))]
    dy: Spanned<Option<Rel<Length>>>,
    /// The length og the line in the direction of `angle`. Cannot be used with `x`, `dx`, `y` or `dy`.
    #[named]
    #[default(Spanned::new(None, Span::detached()))]
    length: Spanned<Option<Rel<Length>>>,
    /// The angle og the line. Is only considered if `length` is set.
    #[named]
    #[default]
    angle: Angle,
) -> Value {
    if x.is_some() && dx.v.is_some() {
        bail!(dx.span, "cannot have both a `x` and `dx` argument")
    }
    if y.is_some() && dy.v.is_some() {
        bail!(dy.span, "cannot have both a `y` and `dy` argument")
    }

    let x_val = x.map(DeltaAbs::Abs).or(dx.v.map(DeltaAbs::Delta));
    let y_val = y.map(DeltaAbs::Abs).or(dy.v.map(DeltaAbs::Delta));

    let (x, y) = if let Some(len) = length.v {
        if x_val.is_some() || y_val.is_some() {
            bail!(length.span, "cannot have both a `x`, `dx`, `y` or `dy` at the same time as a `length` argument")
        }

        let x = angle.cos() * len;
        let y = angle.sin() * len;

        (DeltaAbs::Delta(x), DeltaAbs::Delta(y))
    } else {
        (
            x_val.unwrap_or(DeltaAbs::Delta(Rel::zero())),
            y_val.unwrap_or(DeltaAbs::Delta(Rel::zero())),
        )
    };

    let mut path = PathBuilder::new();
    path.line_to(Axes::new(x, y));
    path.into()
}

/// A path with a bezier segment.
///
/// ## Example
/// ```example
/// #draw.path(stroke: blue, fill: blue.lighten(80%) closed: true,
///   draw.cubicto(x1: 50pt, y1: 0pt, x2: 0pt, y2: 50pt, x: 0pt, y: 0pt))
/// ```
///
/// Display: CubicTo
/// Category: visualize
/// Returns: path
#[func]
pub fn cubicto(
    /// The x coordinate for the curve endpoint.
    #[named]
    x: Option<Rel<Length>>,
    /// The y coordinate for the curve endpoint.
    #[named]
    y: Option<Rel<Length>>,
    /// The delta distance of the curve endpoint in the x coordinate. Cannot be used together with `x`.
    #[named]
    #[default(Spanned::new(None, Span::detached()))]
    dx: Spanned<Option<Rel<Length>>>,
    /// The delta distance of the curve endpoint in the y coordinate. Cannot be used together with `y`.
    #[named]
    #[default(Spanned::new(None, Span::detached()))]
    dy: Spanned<Option<Rel<Length>>>,

    /// The x coordinate for the first controlpoint.
    #[named]
    x1: Option<Rel<Length>>,
    /// The y coordinate for the first controlpoint.
    #[named]
    y1: Option<Rel<Length>>,
    /// The x delta distance of the first controlpoint. Cannot be used together with `x1`.
    #[named]
    #[default(Spanned::new(None, Span::detached()))]
    dx1: Spanned<Option<Rel<Length>>>,
    /// The y delta distance of the first controlpoint. Cannot be used together with `y1`.
    #[named]
    #[default(Spanned::new(None, Span::detached()))]
    dy1: Spanned<Option<Rel<Length>>>,

    /// The x coordinate for the second controlpoint.
    #[named]
    x2: Option<Rel<Length>>,
    /// The y coordinate for the second controlpoint.
    #[named]
    y2: Option<Rel<Length>>,
    /// The x delta distance of the second controlpoint. Cannot be used together with `x2`.
    #[named]
    #[default(Spanned::new(None, Span::detached()))]
    dx2: Spanned<Option<Rel<Length>>>,
    /// The y delta distance of the second controlpoint. Cannot be used together with `y2`.
    #[named]
    #[default(Spanned::new(None, Span::detached()))]
    dy2: Spanned<Option<Rel<Length>>>,
) -> Value {
    if x.is_some() && dx.v.is_some() {
        bail!(dx.span, "cannot have both a `x` and `dx` argument")
    }
    if y.is_some() && dy.v.is_some() {
        bail!(dy.span, "cannot have both a `y` and `dy` argument")
    }

    if x1.is_some() && dx1.v.is_some() {
        bail!(dx1.span, "cannot have both a `x1` and `dx1` argument")
    }
    if y1.is_some() && dy1.v.is_some() {
        bail!(dy1.span, "cannot have both a `y1` and `dy1` argument")
    }

    if x2.is_some() && dx2.v.is_some() {
        bail!(dx2.span, "cannot have both a `x2` and `dx2` argument")
    }
    if y2.is_some() && dy2.v.is_some() {
        bail!(dy2.span, "cannot have both a `y2` and `dy2` argument")
    }

    let x = x
        .map(DeltaAbs::Abs)
        .or(dx.v.map(DeltaAbs::Delta))
        .unwrap_or(DeltaAbs::Delta(Rel::zero()));
    let y = y
        .map(DeltaAbs::Abs)
        .or(dy.v.map(DeltaAbs::Delta))
        .unwrap_or(DeltaAbs::Delta(Rel::zero()));

    let x1 = x1
        .map(DeltaAbs::Abs)
        .or(dx1.v.map(DeltaAbs::Delta))
        .unwrap_or(DeltaAbs::Delta(Rel::zero()));
    let y1 = y1
        .map(DeltaAbs::Abs)
        .or(dy1.v.map(DeltaAbs::Delta))
        .unwrap_or(DeltaAbs::Delta(Rel::zero()));

    let x2 = x2
        .map(DeltaAbs::Abs)
        .or(dx2.v.map(DeltaAbs::Delta))
        .unwrap_or(DeltaAbs::Delta(Rel::zero()));
    let y2 = y2
        .map(DeltaAbs::Abs)
        .or(dy2.v.map(DeltaAbs::Delta))
        .unwrap_or(DeltaAbs::Delta(Rel::zero()));

    let mut path = PathBuilder::new();
    path.cubic_to(Axes::new(x1, y1), Axes::new(x2, y2), Axes::new(x, y));
    path.into()
}

/// A path with a arc segment.
///
/// ## Example
/// ```example
/// #draw.path(stroke: blue, fill: blue.lighten(80%) closed: true,
///   draw.arcto(y: 30pt, r: 15pt))
/// ```
///
/// This function takes the same parameters as an svg arc, see
/// https://developer.mozilla.org/en-US/docs/Web/SVG/Tutorial/Paths#arcs
/// for a detailed description of the varius parameters.
///
/// Display: ArcTo
/// Category: visualize
/// Returns: path
#[func]
pub fn arcto(
    /// The x endpoint of the arc.
    #[named]
    x: Option<Rel<Length>>,
    /// The y endpoint of the arc.
    #[named]
    y: Option<Rel<Length>>,
    /// The delta x endpoint of the arc. Cannot be used together with `x`.
    #[named]
    #[default(Spanned::new(None, Span::detached()))]
    dx: Spanned<Option<Rel<Length>>>,
    /// The delta y endpoint of the arc. Cannot be used together with `y`.
    #[named]
    #[default(Spanned::new(None, Span::detached()))]
    dy: Spanned<Option<Rel<Length>>>,
    #[named]
    /// The radius of the arc. Can be either a single value or a two-element array with
    /// different values.
    #[default(RadiusValue::Single(Abs::cm(1.0).into()))]
    r: RadiusValue,
    /// The angle to rotate the ellipse. Is only relevant if `r` is a two-element array with
    /// different values.
    #[named]
    #[default]
    x_rotation: Angle,
    /// Whether the arc should follow the path that is greater than 180 degrees/
    #[named]
    #[default]
    large: bool,
    /// Whether the arc should start in a positive or negative angle.
    #[named]
    #[default]
    sweep: bool,
) -> Value {
    if x.is_some() && dx.v.is_some() {
        bail!(dx.span, "cannot have both a `x` and `dx` argument")
    }
    if y.is_some() && dy.v.is_some() {
        bail!(dy.span, "cannot have both a `y` and `dy` argument")
    }

    let x_val = x
        .map(DeltaAbs::Abs)
        .or(dx.v.map(DeltaAbs::Delta))
        .unwrap_or(DeltaAbs::Delta(Rel::zero()));
    let y_val = y
        .map(DeltaAbs::Abs)
        .or(dy.v.map(DeltaAbs::Delta))
        .unwrap_or(DeltaAbs::Delta(Rel::zero()));

    let mut path = PathBuilder::new();
    path.arc_to(Axes::new(x_val, y_val), r.to_axes(), x_rotation, large, sweep);
    path.into()
}

#[derive(Debug, Clone, Hash)]
pub enum RadiusValue {
    Single(Length),
    Axes(Axes<Length>),
}

impl RadiusValue {
    fn to_axes(self) -> Axes<Length> {
        match self {
            Self::Single(v) => Axes::splat(v),
            Self::Axes(a) => a,
        }
    }
}

cast_from_value! {
    RadiusValue,
    v: Length => Self::Single(v),
    a: Axes<Length> => Self::Axes(a),
}

/// Creates a path segment containing a single closed circle.
///
/// ## Example
/// ```example
/// #draw.path(stroke: blue, fill: blue.lighten(80%) closed: true,
///   draw.circle(x: 30pt, y: 30pt, r: 15pt))
/// ```
///
/// Display: Circle
/// Category: visualize
/// Returns: path
#[func]
pub fn circle(
    /// The x coordinate of the circle center.
    #[named]
    x: Option<Rel<Length>>,
    /// The y coordinate of the circle center.
    #[named]
    y: Option<Rel<Length>>,
    /// The delta x coordinate of the circle center. Cannot be used together with `x`.
    #[named]
    #[default(Spanned::new(None, Span::detached()))]
    dx: Spanned<Option<Rel<Length>>>,
    /// The delta y coordinate of the circle center. Cannot be used together with `y`.
    #[named]
    #[default(Spanned::new(None, Span::detached()))]
    dy: Spanned<Option<Rel<Length>>>,
    /// The radius of the circle
    #[named]
    #[default(Abs::cm(1.0).into())]
    r: Length,
    /// Whether to draw the circle path in ccw direction, this can be used to create holes in
    /// a shape
    /// ```example
    /// #draw.path(stroke: blue, fill: blue.lighten(80%), {
    ///   draw.moveto(x: 20pt, y: 20pt)
    ///   draw.circle(r: 20pt)
    ///   draw.circle(r: 10pt, ccw: true)
    /// })
    /// ```
    #[named]
    #[default]
    ccw: bool,
) -> Value {
    if x.is_some() && dx.v.is_some() {
        bail!(dx.span, "cannot have both a `x` and `dx` argument")
    }
    if y.is_some() && dy.v.is_some() {
        bail!(dy.span, "cannot have both a `y` and `dy` argument")
    }

    let x_val = x
        .map(DeltaAbs::Abs)
        .or(dx.v.map(DeltaAbs::Delta))
        .unwrap_or(DeltaAbs::Delta(Rel::zero()));
    let y_val = y
        .map(DeltaAbs::Abs)
        .or(dy.v.map(DeltaAbs::Delta))
        .unwrap_or(DeltaAbs::Delta(Rel::zero()));

    let radius = Axes::new(r, r);

    let mut path = PathBuilder::new();
    path.move_to(Axes::new(x_val, y_val));
    path.move_to(Axes::new(DeltaAbs::Delta(r.into()), DeltaAbs::Delta(Rel::zero())));
    path.arc_to(
        Axes::new(DeltaAbs::Delta((-r * 2.0).into()), DeltaAbs::Delta(Rel::zero())),
        radius.clone(),
        Angle::zero(),
        false,
        ccw,
    );
    path.arc_to(
        Axes::new(DeltaAbs::Delta((r * 2.0).into()), DeltaAbs::Delta(Rel::zero())),
        radius,
        Angle::zero(),
        false,
        ccw,
    );
    path.close_path();
    path.move_to(Axes::new(DeltaAbs::Delta((-r).into()), DeltaAbs::Delta(Rel::zero())));
    path.into()
}
