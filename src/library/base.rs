use crate::color::{Color, RgbaColor};
use crate::pretty::pretty;

use super::*;

/// `repr`: Get the string representation of a value.
///
/// # Positional arguments
/// - Any value.
///
/// # Return value
/// The string representation of the value.
pub fn repr(ctx: &mut EvalContext, args: &mut ValueArgs) -> Value {
    match args.require::<Value>(ctx, "value") {
        Some(value) => pretty(&value).into(),
        None => Value::Error,
    }
}

/// `rgb`: Create an RGB(A) color.
///
/// # Positional arguments
/// - Red component:   of type `float`, between 0.0 and 1.0.
/// - Green component: of type `float`, between 0.0 and 1.0.
/// - Blue component:  of type `float`, between 0.0 and 1.0.
/// - Alpha component: optional, of type `float`, between 0.0 and 1.0.
pub fn rgb(ctx: &mut EvalContext, args: &mut ValueArgs) -> Value {
    let r = args.require(ctx, "red component");
    let g = args.require(ctx, "green component");
    let b = args.require(ctx, "blue component");
    let a = args.find(ctx);

    let mut clamp = |component: Option<Spanned<f64>>, default| {
        component.map_or(default, |c| {
            if c.v < 0.0 || c.v > 1.0 {
                ctx.diag(warning!(c.span, "should be between 0.0 and 1.0"));
            }
            (c.v.max(0.0).min(1.0) * 255.0).round() as u8
        })
    };

    Value::Color(Color::Rgba(RgbaColor::new(
        clamp(r, 0),
        clamp(g, 0),
        clamp(b, 0),
        clamp(a, 255),
    )))
}

/// `type`: Find out the name of a value's type.
///
/// # Positional arguments
/// - Any value.
///
/// # Return value
/// The name of the value's type as a string.
pub fn type_(ctx: &mut EvalContext, args: &mut ValueArgs) -> Value {
    match args.require::<Value>(ctx, "value") {
        Some(value) => value.type_name().into(),
        None => Value::Error,
    }
}
