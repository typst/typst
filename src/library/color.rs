use crate::color::RgbaColor;
use crate::prelude::*;

/// `rgb`: Create an RGB(A) color.
pub fn rgb(mut args: Args, ctx: &mut EvalContext) -> Value {
    let r = args.need::<_, Spanned<i64>>(ctx, 0, "red value");
    let g = args.need::<_, Spanned<i64>>(ctx, 1, "green value");
    let b = args.need::<_, Spanned<i64>>(ctx, 2, "blue value");
    let a = args.get::<_, Spanned<i64>>(ctx, 3);
    args.done(ctx);

    let mut clamp = |component: Option<Spanned<i64>>, default| {
        component.map_or(default, |c| {
            if c.v < 0 || c.v > 255 {
                ctx.diag(error!(c.span, "should be between 0 and 255"));
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
