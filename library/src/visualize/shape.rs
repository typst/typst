use std::f64::consts::SQRT_2;

use crate::prelude::*;

/// A rectangle with optional content.
///
/// ## Example
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
///
/// Display: Rectangle
/// Category: visualize
#[element(Layout)]
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
    /// - `{none}` to disable the stroke.
    /// - `{auto}` for a stroke of `{1pt}` black if and if only if no fill is
    ///   given.
    /// - A length specifying the stroke's thickness. The color is inherited,
    ///   defaulting to black.
    /// - A color to use for the stroke. The thickness is inherited, defaulting
    ///   to `{1pt}`.
    /// - A stroke combined from color and thickness using the `+` operator as
    ///   in `{2pt + red}`.
    /// - A stroke described by a dictionary with any of the following keys:
    ///     - `color`: the color to use for the stroke
    ///     - `thickness`: the stroke's thickness
    ///     - `cap`: one of `"butt"`, `"round"` or `"square"`, the line cap of the stroke
    ///     - `join`: one of `"miter"`, `"round"` or `"bevel"`, the line join of the stroke
    ///     - `miter-limit`: the miter limit to use if `join` is `"miter"`, defaults to 4.0
    ///     - `dash`: the dash pattern to use. Can be any of the following:
    ///         - One of the strings `"solid"`, `"dotted"`, `"densely-dotted"`, `"loosely-dotted"`,
    ///           `"dashed"`, `"densely-dashed"`, `"loosely-dashed"`, `"dashdotted"`,
    ///           `"densely-dashdotted"` or `"loosely-dashdotted"`
    ///         - An array with elements that specify the lengths of dashes and gaps, alternating.
    ///           Elements can also be the string `"dot"` for a length equal to the line thickness.
    ///         - A dict with the keys `array`, same as the array above, and `phase`, the offset to
    ///           the start of the first dash.
    /// - Another dictionary describing the stroke for each side inidvidually.
    ///   The dictionary can contain the following keys in order
    ///   of precedence:
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
    pub stroke: Smart<Sides<Option<Option<PartialStroke>>>>,

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
    ///
    /// The default value is `{5pt}`.
    ///
    /// _Note:_ When the rectangle contains text, its exact size depends on the
    /// current [text edges]($func/text.top-edge).
    ///
    /// ```example
    /// #rect(inset: 0pt)[Tight])
    /// ```
    #[resolve]
    #[fold]
    #[default(Sides::splat(Abs::pt(5.0).into()))]
    pub inset: Sides<Option<Rel<Length>>>,

    /// How much to expand the rectangle's size without affecting the layout.
    /// See the [box's documentation]($func/box.outset) for more details.
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

impl Layout for RectElem {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        layout(
            vt,
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
/// ## Example
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
///
/// Display: Square
/// Category: visualize
#[element(Layout)]
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

    /// How to fill the square. See the
    /// [rectangle's documentation]($func/rect.fill) for more details.
    pub fill: Option<Paint>,

    /// How to stroke the square. See the [rectangle's
    /// documentation]($func/rect.stroke) for more details.
    #[resolve]
    #[fold]
    pub stroke: Smart<Sides<Option<Option<PartialStroke>>>>,

    /// How much to round the square's corners. See the [rectangle's
    /// documentation]($func/rect.radius) for more details.
    #[resolve]
    #[fold]
    pub radius: Corners<Option<Rel<Length>>>,

    /// How much to pad the square's content. See the [rectangle's
    /// documentation]($func/rect.inset) for more details.
    ///
    /// The default value is `{5pt}`.
    #[resolve]
    #[fold]
    #[default(Sides::splat(Abs::pt(5.0).into()))]
    pub inset: Sides<Option<Rel<Length>>>,

    /// How much to expand the square's size without affecting the layout. See
    /// the [rectangle's documentation]($func/rect.outset) for more details.
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

impl Layout for SquareElem {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        layout(
            vt,
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
/// ## Example
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
///
/// Display: Ellipse
/// Category: visualize
#[element(Layout)]
pub struct EllipseElem {
    /// The ellipse's width, relative to its parent container.
    pub width: Smart<Rel<Length>>,

    /// The ellipse's height, relative to its parent container.
    pub height: Smart<Rel<Length>>,

    /// How to fill the ellipse. See the
    /// [rectangle's documentation]($func/rect.fill) for more details.
    pub fill: Option<Paint>,

    /// How to stroke the ellipse. See the [rectangle's
    /// documentation]($func/rect.stroke) for more details.
    #[resolve]
    #[fold]
    pub stroke: Smart<Option<PartialStroke>>,

    /// How much to pad the ellipse's content. See the [rectangle's
    /// documentation]($func/rect.inset) for more details.
    ///
    /// The default value is `{5pt}`.
    #[resolve]
    #[fold]
    #[default(Sides::splat(Abs::pt(5.0).into()))]
    pub inset: Sides<Option<Rel<Length>>>,

    /// How much to expand the ellipse's size without affecting the layout. See
    /// the [rectangle's documentation]($func/rect.outset) for more details.
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

impl Layout for EllipseElem {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        layout(
            vt,
            styles,
            regions,
            ShapeKind::Ellipse,
            &self.body(styles),
            Axes::new(self.width(styles), self.height(styles)),
            self.fill(styles),
            self.stroke(styles).map(Sides::splat),
            self.inset(styles),
            self.outset(styles),
            Corners::splat(Rel::zero()),
            self.span(),
        )
    }
}

/// A circle with optional content.
///
/// ## Example
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
///
/// Display: Circle
/// Category: visualize
#[element(Layout)]
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

    /// The circle's height.This is mutually exclusive with `radius` and
    /// `width`.
    ///
    /// In contrast to `radius`, this can be relative to the parent container's
    /// height.
    #[parse(match size {
        None => args.named("height")?,
        size => size,
    })]
    pub height: Smart<Rel<Length>>,

    /// How to fill the circle. See the
    /// [rectangle's documentation]($func/rect.fill) for more details.
    pub fill: Option<Paint>,

    /// How to stroke the circle. See the [rectangle's
    /// documentation]($func/rect.stroke) for more details.
    #[resolve]
    #[fold]
    #[default(Smart::Auto)]
    pub stroke: Smart<Option<PartialStroke>>,

    /// How much to pad the circle's content. See the [rectangle's
    /// documentation]($func/rect.inset) for more details.
    ///
    /// The default value is `{5pt}`.
    #[resolve]
    #[fold]
    #[default(Sides::splat(Abs::pt(5.0).into()))]
    pub inset: Sides<Option<Rel<Length>>>,

    /// How much to expand the circle's size without affecting the layout. See
    /// the [rectangle's documentation]($func/rect.outset) for more details.
    #[resolve]
    #[fold]
    pub outset: Sides<Option<Rel<Length>>>,

    /// The content to place into the circle. The circle expands to fit this
    /// content, keeping the 1-1 aspect ratio.
    #[positional]
    pub body: Option<Content>,
}

impl Layout for CircleElem {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        layout(
            vt,
            styles,
            regions,
            ShapeKind::Circle,
            &self.body(styles),
            Axes::new(self.width(styles), self.height(styles)),
            self.fill(styles),
            self.stroke(styles).map(Sides::splat),
            self.inset(styles),
            self.outset(styles),
            Corners::splat(Rel::zero()),
            self.span(),
        )
    }
}

/// Layout a shape.
fn layout(
    vt: &mut Vt,
    styles: StyleChain,
    regions: Regions,
    kind: ShapeKind,
    body: &Option<Content>,
    sizing: Axes<Smart<Rel<Length>>>,
    fill: Option<Paint>,
    stroke: Smart<Sides<Option<PartialStroke<Abs>>>>,
    mut inset: Sides<Rel<Abs>>,
    outset: Sides<Rel<Abs>>,
    radius: Corners<Rel<Abs>>,
    span: Span,
) -> SourceResult<Fragment> {
    let resolved = sizing
        .zip(regions.base())
        .map(|(s, r)| s.map(|v| v.resolve(styles).relative_to(r)));

    let mut frame;
    if let Some(child) = body {
        let region = resolved.unwrap_or(regions.base());
        if kind.is_round() {
            inset = inset.map(|side| side + Ratio::new(0.5 - SQRT_2 / 4.0));
        }

        // Pad the child.
        let child = child.clone().padded(inset.map(|side| side.map(Length::from)));
        let expand = sizing.as_ref().map(Smart::is_custom);
        let pod = Regions::one(region, expand);
        frame = child.layout(vt, styles, pod)?.into_frame();

        // Enforce correct size.
        *frame.size_mut() = expand.select(region, frame.size());

        // Relayout with full expansion into square region to make sure
        // the result is really a square or circle.
        if kind.is_quadratic() {
            frame.set_size(Size::splat(frame.size().max_by_side()));
            let length = frame.size().max_by_side().min(region.min_by_side());
            let pod = Regions::one(Size::splat(length), Axes::splat(true));
            frame = child.layout(vt, styles, pod)?.into_frame();
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
        frame = Frame::new(size);
    }

    // Prepare stroke.
    let stroke = match stroke {
        Smart::Auto if fill.is_none() => Sides::splat(Some(Stroke::default())),
        Smart::Auto => Sides::splat(None),
        Smart::Custom(strokes) => {
            strokes.map(|s| s.map(PartialStroke::unwrap_or_default))
        }
    };

    // Add fill and/or stroke.
    if fill.is_some() || stroke.iter().any(Option::is_some) {
        if kind.is_round() {
            let outset = outset.relative_to(frame.size());
            let size = frame.size() + outset.sum_by_axis();
            let pos = Point::new(-outset.left, -outset.top);
            let shape = ellipse(size, fill, stroke.left);
            frame.prepend(pos, FrameItem::Shape(shape, span));
        } else {
            frame.fill_and_stroke(fill, stroke, outset, radius, span);
        }
    }

    // Apply metadata.
    frame.meta(styles, false);

    Ok(Fragment::frame(frame))
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
