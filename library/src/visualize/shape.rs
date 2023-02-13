use std::f64::consts::SQRT_2;

use crate::prelude::*;

/// # Rectangle
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
/// ## Parameters
/// - body: `Content` (positional)
///   The content to place into the rectangle.
///
///   When this is omitted, the rectangle takes on a default size of at most
///   `{45pt}` by `{30pt}`.
///
/// - width: `Rel<Length>` (named)
///   The rectangle's width, relative to its parent container.
///
/// - height: `Rel<Length>` (named)
///   The rectangle's height, relative to its parent container.
///
/// ## Category
/// visualize
#[func]
#[capable(Layout)]
#[derive(Debug, Hash)]
pub struct RectNode {
    pub body: Option<Content>,
    pub width: Smart<Rel<Length>>,
    pub height: Smart<Rel<Length>>,
}

#[node]
impl RectNode {
    /// How to fill the rectangle.
    ///
    /// When setting a fill, the default stroke disappears. To create a
    /// rectangle with both fill and stroke, you have to configure both.
    ///
    /// ```example
    /// #rect(fill: blue)
    /// ```
    pub const FILL: Option<Paint> = None;

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
    /// - A dictionary: With a dictionary, the stroke for each side can be set
    ///   individually. The dictionary can contain the following keys in order
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
    #[property(resolve, fold)]
    pub const STROKE: Smart<Sides<Option<Option<PartialStroke>>>> = Smart::Auto;

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
    #[property(resolve, fold)]
    pub const RADIUS: Corners<Option<Rel<Length>>> = Corners::splat(Rel::zero());

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
    #[property(resolve, fold)]
    pub const INSET: Sides<Option<Rel<Length>>> = Sides::splat(Abs::pt(5.0).into());

    /// How much to expand the rectangle's size without affecting the layout.
    /// See the [box's documentation]($func/box.outset) for more details.
    #[property(resolve, fold)]
    pub const OUTSET: Sides<Option<Rel<Length>>> = Sides::splat(Rel::zero());

    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let width = args.named("width")?.unwrap_or_default();
        let height = args.named("height")?.unwrap_or_default();
        let body = args.eat()?;
        Ok(Self { body, width, height }.pack())
    }
}

impl Layout for RectNode {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        layout(
            vt,
            ShapeKind::Rect,
            &self.body,
            Axes::new(self.width, self.height),
            styles.get(Self::FILL),
            styles.get(Self::STROKE),
            styles.get(Self::INSET),
            styles.get(Self::OUTSET),
            styles.get(Self::RADIUS),
            styles,
            regions,
        )
    }
}

/// # Square
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
/// ## Parameters
/// - body: `Content` (positional)
///   The content to place into the square. The square expands to fit this
///   content, keeping the 1-1 aspect ratio.
///
///   When this is omitted, the square takes on a default size of at most
///   `{30pt}`.
///
/// - size: `Length` (named)
///   The square's side length. This is mutually exclusive with `width` and
///   `height`.
///
/// - width: `Rel<Length>` (named)
///   The square's width. This is mutually exclusive with `size` and `height`.
///
///   In contrast to `size`, this can be relative to the parent container's
///   width.
///
/// - height: `Rel<Length>` (named)
///   The square's height. This is mutually exclusive with `size` and `width`.
///
///   In contrast to `size`, this can be relative to the parent container's
///   height.
///
/// ## Category
/// visualize
#[func]
#[capable(Layout)]
#[derive(Debug, Hash)]
pub struct SquareNode {
    pub body: Option<Content>,
    pub width: Smart<Rel<Length>>,
    pub height: Smart<Rel<Length>>,
}

#[node]
impl SquareNode {
    /// How to fill the square. See the
    /// [rectangle's documentation]($func/rect.fill) for more details.
    pub const FILL: Option<Paint> = None;

    /// How to stroke the square. See the [rectangle's
    /// documentation]($func/rect.stroke) for more details.
    #[property(resolve, fold)]
    pub const STROKE: Smart<Sides<Option<Option<PartialStroke>>>> = Smart::Auto;

    /// How much to round the square's corners. See the [rectangle's
    /// documentation]($func/rect.radius) for more details.
    #[property(resolve, fold)]
    pub const RADIUS: Corners<Option<Rel<Length>>> = Corners::splat(Rel::zero());

    /// How much to pad the square's content. See the [rectangle's
    /// documentation]($func/rect.inset) for more details.
    ///
    /// The default value is `{5pt}`.
    #[property(resolve, fold)]
    pub const INSET: Sides<Option<Rel<Length>>> = Sides::splat(Abs::pt(5.0).into());

    /// How much to expand the square's size without affecting the layout. See
    /// the [rectangle's documentation]($func/rect.outset) for more details.
    #[property(resolve, fold)]
    pub const OUTSET: Sides<Option<Rel<Length>>> = Sides::splat(Rel::zero());

    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let size = args.named::<Smart<Length>>("size")?.map(|s| s.map(Rel::from));
        let width = match size {
            None => args.named("width")?,
            size => size,
        }
        .unwrap_or_default();
        let height = match size {
            None => args.named("height")?,
            size => size,
        }
        .unwrap_or_default();
        let body = args.eat()?;
        Ok(Self { body, width, height }.pack())
    }
}

impl Layout for SquareNode {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        layout(
            vt,
            ShapeKind::Square,
            &self.body,
            Axes::new(self.width, self.height),
            styles.get(Self::FILL),
            styles.get(Self::STROKE),
            styles.get(Self::INSET),
            styles.get(Self::OUTSET),
            styles.get(Self::RADIUS),
            styles,
            regions,
        )
    }
}

/// # Ellipse
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
/// ## Parameters
/// - body: `Content` (positional)
///   The content to place into the ellipse.
///
///   When this is omitted, the ellipse takes on a default size of at most
///   `{45pt}` by `{30pt}`.
///
/// - width: `Rel<Length>` (named)
///   The ellipse's width, relative to its parent container.
///
/// - height: `Rel<Length>` (named)
///   The ellipse's height, relative to its parent container.
///
/// ## Category
/// visualize
#[func]
#[capable(Layout)]
#[derive(Debug, Hash)]
pub struct EllipseNode {
    pub body: Option<Content>,
    pub width: Smart<Rel<Length>>,
    pub height: Smart<Rel<Length>>,
}

#[node]
impl EllipseNode {
    /// How to fill the ellipse. See the
    /// [rectangle's documentation]($func/rect.fill) for more details.
    pub const FILL: Option<Paint> = None;

    /// How to stroke the ellipse. See the [rectangle's
    /// documentation]($func/rect.stroke) for more details.
    #[property(resolve, fold)]
    pub const STROKE: Smart<Option<PartialStroke>> = Smart::Auto;

    /// How much to pad the ellipse's content. See the [rectangle's
    /// documentation]($func/rect.inset) for more details.
    ///
    /// The default value is `{5pt}`.
    #[property(resolve, fold)]
    pub const INSET: Sides<Option<Rel<Length>>> = Sides::splat(Abs::pt(5.0).into());

    /// How much to expand the ellipse's size without affecting the layout. See
    /// the [rectangle's documentation]($func/rect.outset) for more details.
    #[property(resolve, fold)]
    pub const OUTSET: Sides<Option<Rel<Length>>> = Sides::splat(Rel::zero());

    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let width = args.named("width")?.unwrap_or_default();
        let height = args.named("height")?.unwrap_or_default();
        let body = args.eat()?;
        Ok(Self { body, width, height }.pack())
    }
}

impl Layout for EllipseNode {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        layout(
            vt,
            ShapeKind::Ellipse,
            &self.body,
            Axes::new(self.width, self.height),
            styles.get(Self::FILL),
            styles.get(Self::STROKE).map(Sides::splat),
            styles.get(Self::INSET),
            styles.get(Self::OUTSET),
            Corners::splat(Rel::zero()),
            styles,
            regions,
        )
    }
}

/// # Circle
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
/// ## Parameters
/// - body: `Content` (positional)
///   The content to place into the circle. The circle expands to fit this
///   content, keeping the 1-1 aspect ratio.
///
/// - radius: `Length` (named)
///   The circle's radius. This is mutually exclusive with `width` and
///   `height`.
///
/// - width: `Rel<Length>` (named)
///   The circle's width. This is mutually exclusive with `radius` and `height`.
///
///   In contrast to `size`, this can be relative to the parent container's
///   width.
///
/// - height: `Rel<Length>` (named)
///   The circle's height.This is mutually exclusive with `radius` and `width`.
///
///   In contrast to `size`, this can be relative to the parent container's
///   height.
///
/// ## Category
/// visualize
#[func]
#[capable(Layout)]
#[derive(Debug, Hash)]
pub struct CircleNode {
    pub body: Option<Content>,
    pub width: Smart<Rel<Length>>,
    pub height: Smart<Rel<Length>>,
}

#[node]
impl CircleNode {
    /// How to fill the circle. See the
    /// [rectangle's documentation]($func/rect.fill) for more details.
    pub const FILL: Option<Paint> = None;

    /// How to stroke the circle. See the [rectangle's
    /// documentation]($func/rect.stroke) for more details.
    #[property(resolve, fold)]
    pub const STROKE: Smart<Option<PartialStroke>> = Smart::Auto;

    /// How much to pad the circle's content. See the [rectangle's
    /// documentation]($func/rect.inset) for more details.
    ///
    /// The default value is `{5pt}`.
    #[property(resolve, fold)]
    pub const INSET: Sides<Option<Rel<Length>>> = Sides::splat(Abs::pt(5.0).into());

    /// How much to expand the circle's size without affecting the layout. See
    /// the [rectangle's documentation]($func/rect.outset) for more details.
    #[property(resolve, fold)]
    pub const OUTSET: Sides<Option<Rel<Length>>> = Sides::splat(Rel::zero());

    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let size = args
            .named::<Smart<Length>>("radius")?
            .map(|s| s.map(|r| 2.0 * Rel::from(r)));
        let width = match size {
            None => args.named("width")?,
            size => size,
        }
        .unwrap_or_default();
        let height = match size {
            None => args.named("height")?,
            size => size,
        }
        .unwrap_or_default();
        let body = args.eat()?;
        Ok(Self { body, width, height }.pack())
    }
}

impl Layout for CircleNode {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        layout(
            vt,
            ShapeKind::Circle,
            &self.body,
            Axes::new(self.width, self.height),
            styles.get(Self::FILL),
            styles.get(Self::STROKE).map(Sides::splat),
            styles.get(Self::INSET),
            styles.get(Self::OUTSET),
            Corners::splat(Rel::zero()),
            styles,
            regions,
        )
    }
}

/// Layout a shape.
fn layout(
    vt: &mut Vt,
    kind: ShapeKind,
    body: &Option<Content>,
    sizing: Axes<Smart<Rel<Length>>>,
    fill: Option<Paint>,
    stroke: Smart<Sides<Option<PartialStroke<Abs>>>>,
    mut inset: Sides<Rel<Abs>>,
    outset: Sides<Rel<Abs>>,
    radius: Corners<Rel<Abs>>,
    styles: StyleChain,
    regions: Regions,
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

        // Relayout with full expansion into square region to make sure
        // the result is really a square or circle.
        if kind.is_quadratic() {
            let length = frame.size().max_by_side().min(region.min_by_side());
            let size = Size::splat(length);
            let pod = Regions::one(size, Axes::splat(true));
            frame = child.layout(vt, styles, pod)?.into_frame();
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
            frame.prepend(pos, Element::Shape(shape));
        } else {
            frame.fill_and_stroke(fill, stroke, outset, radius);
        }
    }

    // Apply metadata.
    frame.meta(styles);

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
