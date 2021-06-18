use crate::color::{Color, RgbaColor};
use crate::pretty::pretty;

use super::*;

/// `type`: The name of a value's type.
///
/// # Positional parameters
/// - Any value.
///
/// # Return value
/// The name of the value's type as a string.
pub fn type_(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    match args.eat_expect::<Value>(ctx, "value") {
        Some(value) => value.type_name().into(),
        None => Value::Error,
    }
}

/// `repr`: The string representation of a value.
///
/// # Positional parameters
/// - Any value.
///
/// # Return value
/// The string representation of the value.
pub fn repr(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    match args.eat_expect::<Value>(ctx, "value") {
        Some(value) => pretty(&value).into(),
        None => Value::Error,
    }
}

/// `len`: The length of a string, an array or a dictionary.
pub fn len(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    match args.eat_expect::<Spanned<Value>>(ctx, "collection") {
        Some(Spanned { v: Value::Str(v), .. }) => Value::Int(v.len() as i64),
        Some(Spanned { v: Value::Array(v), .. }) => Value::Int(v.len() as i64),
        Some(Spanned { v: Value::Dict(v), .. }) => Value::Int(v.len() as i64),
        Some(other) if other.v != Value::Error => {
            ctx.diag(error!(other.span, "expected string, array or dictionary"));
            Value::Error
        }
        _ => Value::Error,
    }
}

/// `rgb`: Create an RGB(A) color.
///
/// # Positional parameters
/// - Red component: of type `float`, between 0.0 and 1.0.
/// - Green component: of type `float`, between 0.0 and 1.0.
/// - Blue component: of type `float`, between 0.0 and 1.0.
/// - Alpha component: optional, of type `float`, between 0.0 and 1.0.
///
/// # Return value
/// The color with the given components.
pub fn rgb(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let r = args.eat_expect(ctx, "red component");
    let g = args.eat_expect(ctx, "green component");
    let b = args.eat_expect(ctx, "blue component");
    let a = args.eat(ctx);

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
