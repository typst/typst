use super::*;
use crate::color::RgbaColor;

/// `rgb`: Create an RGB(A) color.
pub async fn rgb(mut args: Args, ctx: &mut LayoutContext) -> Value {
    let r = args.get::<_, Spanned<i64>>(ctx, 0);
    let g = args.get::<_, Spanned<i64>>(ctx, 1);
    let b = args.get::<_, Spanned<i64>>(ctx, 2);
    let a = args.get::<_, Spanned<i64>>(ctx, 3);
    args.done(ctx);

    let mut clamp = |component: Option<Spanned<i64>>, default| {
        component.map_or(default, |c| {
            if c.v < 0 || c.v > 255 {
                error!(@ctx.f, c.span, "should be between 0 and 255")
            }
            c.v.max(0).min(255) as u8
        })
    };

    Value::Color(RgbaColor::new(
        clamp(r, 0),
        clamp(g, 0),
        clamp(b, 0),
        clamp(a, 255),
    ))
}
