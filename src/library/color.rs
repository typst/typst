use super::*;
use crate::color::RgbaColor;

/// `rgb`: Create an RGB(A) color.
pub async fn rgb(span: Span, mut args: DictValue, _: LayoutContext<'_>) -> Pass<Value> {
    let mut f = Feedback::new();

    let r = args.expect::<Spanned<f64>>("red value", span, &mut f);
    let g = args.expect::<Spanned<f64>>("green value", span, &mut f);
    let b = args.expect::<Spanned<f64>>("blue value", span, &mut f);
    let a = args.take::<Spanned<f64>>();

    let mut clamp = |component: Option<Spanned<f64>>, default| {
        component
            .map(|c| {
                if c.v < 0.0 || c.v > 255.0 {
                    error!(@f, c.span, "should be between 0 and 255")
                }
                c.v.min(255.0).max(0.0).round() as u8
            })
            .unwrap_or(default)
    };

    let color = RgbaColor::new(clamp(r, 0), clamp(g, 0), clamp(b, 0), clamp(a, 255));

    args.unexpected(&mut f);
    Pass::new(Value::Color(color), f)
}
