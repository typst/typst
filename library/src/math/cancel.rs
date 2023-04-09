use super::*;

/// # Cancel
/// Displays a diagonal line over math content.
///
/// ## Example
///
/// ```example
/// Here, we can simplify: $ (a dot.c b dot.c cancel(x)) / cancel(x) $
///
/// We can also invert the line: $ (a cancel((b + c), invert: #true)) / cancel(b + c, invert: #true) $
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
    /// Defaults to `{100% + 2pt}`.
    pub length: Smart<Rel<Length>>,

    /// If the cancel line should be inverted (heading northwest instead of northeast).
    ///
    /// Defaults to `{false}`.
    #[default(false)]
    pub invert: bool,

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
        let (width_pt, height_pt) = (width.to_pt(), height.to_pt());

        let diagonal_pt = width_pt.hypot(height_pt);
        let diagonal = Abs::pt(diagonal_pt);

        let length = self
            .length(ctx.styles())
            .unwrap_or(Rel::new(Ratio::one(), Abs::pt(2.0).into()))
            .resolve(ctx.styles())
            .relative_to(diagonal.into());

        // default stroke has 0.5pt
        let stroke = self.stroke(ctx.styles()).unwrap_or(Stroke {
            paint: TextElem::fill_in(ctx.styles()),
            thickness: Abs::pt(0.5),
        });

        let mid_x = width / 2.0;
        let mid_y = height / 2.0;

        let invert = self.invert(ctx.styles());

        let angle = self.rotate(ctx.styles());

        // scale the amount needed such that the cancel line has the given 'length'
        // (reference length is the whole diagonal)
        let scale = length.to_pt() / diagonal_pt;

        // invert horizontally if 'invert' was given
        let scale_x = scale * invert.then_some(-1.0).unwrap_or(1.0);
        let scale_y = scale;
        let scale_axes = Axes::new(scale_x, scale_y);

        // draw a line from bottom left to top right of the given element,
        // where the origin represents the very middle of that element
        // that is, a line from (-width / 2, height / 2) with length components (width, -height)
        // (sign is inverted in the y-axis)
        // after applying the scale, the line will have the correct length and orientation
        // (inverted if needed)
        let start = Axes::new(-mid_x, mid_y).zip(scale_axes).map(|(l, s)| l * s);

        let delta = Axes::new(width, -height).zip(scale_axes).map(|(l, s)| l * s);

        let mut cancel_line_frame = Frame::new(size);
        cancel_line_frame.push(
            start.to_point(),
            FrameItem::Shape(
                Geometry::Line(delta.to_point()).stroked(stroke),
                self.span(),
            ),
        );

        cancel_line_frame.transform(Transform::rotate(angle));

        // the origin of our line is the very middle of the element
        body.push_frame(Point::new(mid_x, mid_y), cancel_line_frame);

        ctx.push(FrameFragment::new(ctx, body));

        Ok(())
    }
}
