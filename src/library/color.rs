use crate::color::RgbaColor;
use super::*;

/// `rgb`: Create an RGB(A) color.
pub async fn rgb(span: Span, mut args: TableValue, _: LayoutContext<'_>) -> Pass<Value> {
    let mut f = Feedback::new();

    let color = RgbaColor::new(
        clamp(args.expect::<Spanned<f64>>("red value", span, &mut f), &mut f),
        clamp(args.expect::<Spanned<f64>>("green value", span, &mut f), &mut f),
        clamp(args.expect::<Spanned<f64>>("blue value", span, &mut f), &mut f),
        clamp(args.take::<Spanned<f64>>(), &mut f),
    );

    args.unexpected(&mut f);
    Pass::new(Value::Color(color), f)
}

fn clamp(component: Option<Spanned<f64>>, f: &mut Feedback) -> u8 {
    component.map(|c| {
        if c.v < 0.0 || c.v > 255.0 {
            error!(@f, c.span, "should be between 0 and 255")
        }
        c.v.min(255.0).max(0.0).round() as u8
    }).unwrap_or_default()
}
