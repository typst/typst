use super::*;
use crate::color::RgbaColor;

/// `rgb`: Create an RGB(A) color.
pub async fn rgb(mut args: DictValue, ctx: &mut LayoutContext) -> Value {
    let mut f = Feedback::new();

    let r = args.expect::<Spanned<i64>>("red value", Span::ZERO, &mut f);
    let g = args.expect::<Spanned<i64>>("green value", Span::ZERO, &mut f);
    let b = args.expect::<Spanned<i64>>("blue value", Span::ZERO, &mut f);
    let a = args.take::<Spanned<i64>>();

    let mut clamp = |component: Option<Spanned<i64>>, default| {
        component.map_or(default, |c| {
            if c.v < 0 || c.v > 255 {
                error!(@f, c.span, "should be between 0 and 255")
            }
            c.v.max(0).min(255) as u8
        })
    };

    args.unexpected(&mut ctx.f);
    Value::Color(RgbaColor::new(
        clamp(r, 0),
        clamp(g, 0),
        clamp(b, 0),
        clamp(a, 255),
    ))
}
