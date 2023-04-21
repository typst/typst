use super::*;

/// Displays a diagonal line over math content.
///
/// ## Example
///
/// ```example
/// Here, we can simplify:
/// $ (a dot.c b dot.c cancel(x)) / cancel(x) $
/// ```
///
/// Display: Cancel
/// Category: math
#[element(LayoutMath)]
pub struct CancelElem {
    /// The content over which the line should be placed.
    #[required]
    pub body: Content,

    /// The length of the line, relative to the length of the diagonal spanning the whole
    /// element being "cancelled". A value of `{100%}` would then have the line span precisely
    /// the element's diagonal.
    ///
    /// Defaults to `{100% + 3pt}`.
    ///
    /// ## Example
    ///
    /// ```example
    /// $ a + cancel(x, length: #200%) - b - $cancel(x, length: #200%) $
    /// ```
    #[default(Rel::new(Ratio::one(), Abs::pt(3.0).into()))]
    pub length: Rel<Length>,

    /// If the cancel line should be inverted (heading northwest instead of northeast).
    ///
    /// Defaults to `{false}`.
    ///
    /// ## Example
    ///
    /// ```example
    /// $ (a cancel((b + c), inverted: #true)) / cancel(b + c, inverted: #true) $
    /// ```
    #[default(false)]
    pub inverted: bool,

    /// If two opposing cancel lines should be drawn, forming a cross over the element.
    /// Overrides `inverted`.
    ///
    /// Defaults to `{false}`.
    ///
    /// ## Example
    ///
    /// ```example
    /// $ cancel(x, cross: #true) $
    /// ```
    #[default(false)]
    pub cross: bool,

    /// Rotate the cancel line by a certain angle. See the
    /// [line's documentation]($func/line.angle) for more details.
    ///
    /// ## Example
    ///
    /// ```example
    /// $ cancel(x, rotation: #30deg) $
    /// ```
    #[default(Angle::zero())]
    pub rotation: Angle,

    /// How to stroke the cancel line. See the
    /// [line's documentation]($func/line.stroke) for more details.
    ///
    /// ## Example
    ///
    /// ```example
    /// $ cancel(x, stroke: #{red + 1.5pt}) $
    /// ```
    #[resolve]
    #[fold]
    pub stroke: PartialStroke,
}

impl LayoutMath for CancelElem {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        let mut body = ctx.layout_frame(&self.body())?;

        let styles = ctx.styles();
        let body_size = body.size();
        let span = self.span();
        let length = self
            .length(styles)
            .resolve(styles);

        // Default stroke has 0.5pt for better visuals.
        let stroke = self.stroke(styles).unwrap_or(Stroke {
            paint: TextElem::fill_in(styles),
            thickness: Abs::pt(0.5),
            ..Default::default()
        });

        let invert = self.inverted(styles);
        let cross = self.cross(styles);
        let angle = self.rotation(styles);

        let invert_first_line = !cross && invert;
        let first_line = draw_cancel_line(
            length,
            stroke.clone(),
            invert_first_line,
            angle,
            body_size,
            span,
        );

        // The origin of our line is the very middle of the element.
        let center = body_size.to_point() / 2.0;
        body.push_frame(center, first_line);

        if cross {
            // Draw the second line.
            let second_line = draw_cancel_line(
                length,
                stroke,
                true,
                angle,
                body_size,
                span,
            );

            body.push_frame(center, second_line);
        }

        ctx.push(FrameFragment::new(ctx, body));

        Ok(())
    }
}

/// Draws a cancel line.
fn draw_cancel_line(
    length: Rel<Abs>,
    stroke: Stroke,
    invert: bool,
    angle: Angle,
    body_size: Size,
    span: Span,
) -> Frame {
    //            B
    //           /|
    // diagonal / | height
    //         /  |
    //        /   |
    //       O ----
    //         width
    let diagonal = body_size.to_point().hypot();
    let length = length.relative_to(diagonal);
    let (width, height) = (body_size.x, body_size.y);
    let mid = body_size / 2.0;

    // Scale the amount needed such that the cancel line has the given 'length'
    // (reference length, or 100%, is the whole diagonal).
    // Scales from the center.
    let scale = length.to_raw() / diagonal.to_raw();

    // invert horizontally if 'invert' was given
    let scale_x = scale * invert.then_some(-1.0).unwrap_or(1.0);
    let scale_y = scale;
    let scales = Axes::new(scale_x, scale_y);

    // Draw a line from bottom left to top right of the given element,
    // where the origin represents the very middle of that element
    // that is, a line from (-width / 2, height / 2) with length components (width, -height)
    // (sign is inverted in the y-axis).
    // After applying the scale, the line will have the correct length and orientation
    // (inverted if needed).
    let start = Axes::new(-mid.x, mid.y).zip(scales).map(|(l, s)| l * s);
    let delta = Axes::new(width, -height).zip(scales).map(|(l, s)| l * s);

    let mut frame = Frame::new(body_size);
    frame.push(
        start.to_point(),
        FrameItem::Shape(Geometry::Line(delta.to_point()).stroked(stroke), span),
    );

    // Having the middle of the line at the origin is convenient here.
    frame.transform(Transform::rotate(angle));
    frame
}
