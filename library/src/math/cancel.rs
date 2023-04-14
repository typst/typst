use super::*;

/// Displays a diagonal line over math content.
///
/// ## Example
///
/// ```example
/// Here, we can simplify: $ (a dot.c b dot.c cancel(x)) / cancel(x) $
///
/// We can also invert the line: $ (a cancel((b + c), invert: #true)) / cancel(b + c, invert: #true) $
///
/// Or draw two lines (a cross): $ x + cancel(a + b + c, cross: #true) $
///
/// Or set its length, relative to the diagonal: $cancel(x, length: #200%)$
///
/// Rotating and even making it red and thicker is possible: $ cancel(x, rotate: #30deg, stroke: #{red + 1.5pt}) $
/// ```
///
/// Display: Cancel
/// Category: math
#[element(LayoutMath)]
pub struct CancelElem {
    /// The content which the line should be placed.
    #[required]
    pub body: Content,

    /// The length of the line, relative to the length of the main diagonal spanning the whole
    /// element being "cancelled".
    ///
    /// Defaults to `{100% + 3pt}`.
    pub length: Smart<Rel<Length>>,

    /// If the cancel line should be inverted (heading northwest instead of northeast).
    ///
    /// Defaults to `{false}`.
    #[default(false)]
    pub invert: bool,

    /// If two opposing cancel lines should be drawn, forming a cross over the element.
    /// Overrides `invert`.
    ///
    /// Defaults to `{false}`.
    #[default(false)]
    pub cross: bool,

    /// Rotate the cancel line by a certain angle. See the
    /// [line's documentation]($func/line.angle) for more details.
    #[default(Angle::zero())]
    pub rotate: Angle,

    /// How to stroke the cancel line. See the
    /// [line's documentation]($func/line.stroke) for more details.
    #[resolve]
    #[fold]
    pub stroke: PartialStroke,
}

impl LayoutMath for CancelElem {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        let mut body = ctx.layout_frame(&self.body())?;

        let size = body.size();
        let (width, height) = (size.x, size.y);

        let length = self
            .length(ctx.styles()) // empirically pleasant default
            .unwrap_or(Rel::new(Ratio::one(), Abs::pt(3.0).into()))
            .resolve(ctx.styles());

        // default stroke has 0.5pt for better visuals
        let stroke = self.stroke(ctx.styles()).unwrap_or(Stroke {
            paint: TextElem::fill_in(ctx.styles()),
            thickness: Abs::pt(0.5),
            ..Default::default()
        });

        let invert = self.invert(ctx.styles());
        let cross = self.cross(ctx.styles());
        let angle = self.rotate(ctx.styles());

        let invert_first_line = !cross && invert;

        let first_line = draw_cancel_line(
            length,
            stroke.clone(),
            invert_first_line,
            angle,
            size,
            self.span(),
        );

        // the origin of our line is the very middle of the element
        body.push_frame(Point::new(width / 2.0, height / 2.0), first_line);

        if cross {
            // draw the second line
            let second_line = draw_cancel_line(
                length,
                stroke,
                /*invert:*/ true,
                angle,
                size,
                self.span(),
            );

            body.push_frame(Point::new(width / 2.0, height / 2.0), second_line);
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
    let (width, height) = (body_size.x, body_size.y);
    let (width_pt, height_pt) = (width.to_pt(), height.to_pt());

    //              /|
    // diagonal_pt / |  height_pt
    //            /  |
    //           /   |
    //           -----
    //          width_pt
    let diagonal_pt = width_pt.hypot(height_pt);
    let diagonal = Abs::pt(diagonal_pt);

    let length = length.relative_to(diagonal);

    let mid_x = width / 2.0;
    let mid_y = height / 2.0;

    // scale the amount needed such that the cancel line has the given 'length'
    // (reference length is the whole diagonal)
    // scales from the center.
    let scale = length.to_pt() / diagonal_pt;

    // invert horizontally if 'invert' was given
    let scale_x = scale * invert.then_some(-1.0).unwrap_or(1.0);
    let scale_y = scale;
    let scales = Axes::new(scale_x, scale_y);

    // draw a line from bottom left to top right of the given element,
    // where the origin represents the very middle of that element
    // that is, a line from (-width / 2, height / 2) with length components (width, -height)
    // (sign is inverted in the y-axis)
    // after applying the scale, the line will have the correct length and orientation
    // (inverted if needed)
    let start = Axes::new(-mid_x, mid_y).zip(scales).map(|(l, s)| l * s);

    let delta = Axes::new(width, -height).zip(scales).map(|(l, s)| l * s);

    let mut cancel_line_frame = Frame::new(body_size);
    cancel_line_frame.push(
        start.to_point(),
        FrameItem::Shape(Geometry::Line(delta.to_point()).stroked(stroke), span),
    );

    // having the middle of the line at the origin is convenient here
    cancel_line_frame.transform(Transform::rotate(angle));

    cancel_line_frame
}
