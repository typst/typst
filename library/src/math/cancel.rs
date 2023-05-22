use super::*;

pub enum Rotation {
    /// A string a match is replaced with.
    Angle(Angle),
    /// Function of type Dict -> Str (see `captures_to_dict` or `match_to_dict`)
    /// whose output is inserted for the match.
    Func(Func),
}

cast_from_value! {
    Rotation,
    v: Angle => Self::Angle(v),
    v: Func => Self::Func(v),
}

cast_to_value! {
    v: Rotation => match v {
        Rotation::Angle(v) => v.into(),
        Rotation::Func(v) => v.into(),
    }
}

/// Displays a diagonal line over a part of an equation.
///
/// This is commonly used to show the eliminiation of a term.
///
/// ## Example { #example }
/// ```example
/// >>> #set page(width: 140pt)
/// Here, we can simplify:
/// $ (a dot b dot cancel(x)) /
///     cancel(x) $
/// ```
///
/// Display: Cancel
/// Category: math
#[element(LayoutMath)]
pub struct CancelElem {
    /// The content over which the line should be placed.
    #[required]
    pub body: Content,

    /// The length of the line, relative to the length of the diagonal spanning
    /// the whole element being "cancelled". A value of `{100%}` would then have
    /// the line span precisely the element's diagonal.
    ///
    /// ```example
    /// >>> #set page(width: 140pt)
    /// $ a + cancel(x, length: #200%)
    ///     - cancel(x, length: #200%) $
    /// ```
    #[default(Rel::new(Ratio::one(), Abs::pt(3.0).into()))]
    pub length: Rel<Length>,

    /// If the cancel line should be inverted (pointing to the top left instead
    /// of top right).
    ///
    /// ```example
    /// >>> #set page(width: 140pt)
    /// $ (a cancel((b + c), inverted: #true)) /
    ///     cancel(b + c, inverted: #true) $
    /// ```
    #[default(false)]
    pub inverted: bool,

    /// If two opposing cancel lines should be drawn, forming a cross over the
    /// element. Overrides `inverted`.
    ///
    /// ```example
    /// >>> #set page(width: 140pt)
    /// $ cancel(Pi, cross: #true) $
    /// ```
    #[default(false)]
    pub cross: bool,

    /// Rotate the cancel line counterclockwise relative to the horizontal axis.
    /// - If given an angle, the line is rotated by that angle.
    /// - It given a function `#(angle) => angle`, the line is rotated by the
    /// angle returned by that function.
    /// - If absent, the line assumes the default angle; that is, along the diagonal
    /// line of the content box.
    ///
    /// ```example
    /// >>> #set page(width: 140pt)
    /// $
    /// cancel(Pi, rotation: #30deg)
    /// cancel(a, rotation: #0deg)
    /// cancel(1/(1+x), rotation: #(angle) => { angle + 30deg })
    /// $
    /// ```
    pub rotation: Option<Rotation>,

    /// How to stroke the cancel line. See the
    /// [line's documentation]($func/line.stroke) for more details.
    ///
    /// ```example
    /// >>> #set page(width: 140pt)
    /// $ cancel(
    ///   sum x,
    ///   stroke: #(
    ///     paint: red,
    ///     thickness: 1.5pt,
    ///     dash: "dashed",
    ///   ),
    /// ) $
    /// ```
    #[resolve]
    #[fold]
    #[default(PartialStroke {
        // Default stroke has 0.5pt for better visuals.
        thickness: Smart::Custom(Abs::pt(0.5)),
        ..Default::default()
    })]
    pub stroke: PartialStroke,
}

impl LayoutMath for CancelElem {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        let body = ctx.layout_fragment(&self.body())?;
        // Use the same math class as the body, in order to preserve automatic spacing around it.
        let body_class = body.class().unwrap_or(MathClass::Special);
        let mut body = body.into_frame();

        let styles = ctx.styles();
        let body_size = body.size();
        let span = self.span();
        let length = self.length(styles).resolve(styles);

        let stroke = self.stroke(styles).unwrap_or(Stroke {
            paint: TextElem::fill_in(styles),
            ..Default::default()
        });

        let invert = self.inverted(styles);
        let cross = self.cross(styles);
        let rotation = self.rotation(styles);

        let invert_first_line = !cross && invert;
        let first_line = draw_cancel_line(
            length,
            stroke.clone(),
            invert_first_line,
            &rotation,
            body_size,
            span,
        );

        // The origin of our line is the very middle of the element.
        let center = body_size.to_point() / 2.0;
        body.push_frame(center, first_line);

        if cross {
            // Draw the second line.
            let second_line =
                draw_cancel_line(length, stroke, true, &rotation, body_size, span);

            body.push_frame(center, second_line);
        }

        ctx.push(FrameFragment::new(ctx, body).with_class(body_class));

        Ok(())
    }
}

/// Draws a cancel line.
fn draw_cancel_line(
    length: Rel<Abs>,
    stroke: Stroke,
    invert: bool,
    rotation: &Option<Rotation>,
    body_size: Size,
    span: Span,
) -> Frame {
    let (width, height) = (body_size.x, body_size.y);
    let mid = body_size / 2.0;

    // The default angle is the diagonal's angle.
    let mut angle = Angle::rad(f64::atan(height.to_raw() / width.to_raw()));
    if let Some(rotation) = rotation {
        angle = match rotation {
            Rotation::Angle(v) => *v,
            Rotation::Func(_) => {
                Angle::deg(0.0)
                // let args =
                //     Args::new(func.span(), [Value::Angle(angle)]);
                // func.call_vm(vm, args)?  There's no VM passed in
            }
        }
    }

    // Unless intentionally scaled up or down, the line shall end on the content box's
    // boundary. Specifically, if the angle is the diagonal's angle, the line shall end
    // at the content box's opposite corners. We calculate the line's initial length
    // based on this principle.
    let mut half_len = mid.x.safe_div(angle.cos()).abs();
    if half_len > mid.to_point().hypot() || half_len == Abs::zero() {
        half_len = mid.y.safe_div(angle.sin()).abs();
    }

    let mut frame = Frame::new(body_size);
    // Draw a horizontal line, then rotate the line.
    let scale_factor =
        length.relative_to(half_len * 2.0).to_raw() / (half_len.to_raw() * 2.0);
    let start = Axes::with_x(-half_len * scale_factor);
    let delta = Axes::with_x(half_len * 2.0 * scale_factor);
    frame.push(
        start.to_point(),
        FrameItem::Shape(Geometry::Line(delta.to_point()).stroked(stroke), span),
    );
    // Having the middle of the line at the origin is convenient here. Note
    // Transform::rotate() rotates clockwise.
    frame.transform(Transform::rotate(if invert { angle } else { -angle }));

    frame
}
