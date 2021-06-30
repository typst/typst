use std::cmp::Ordering;
use std::str::FromStr;

use crate::color::{Color, RgbaColor};
use crate::pretty::pretty;

use super::*;

/// `type`: The name of a value's type.
pub fn type_(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    match args.expect::<Value>(ctx, "value") {
        Some(value) => value.type_name().into(),
        None => Value::Error,
    }
}

/// `repr`: The string representation of a value.
pub fn repr(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    match args.expect::<Value>(ctx, "value") {
        Some(value) => pretty(&value).into(),
        None => Value::Error,
    }
}

/// `len`: The length of a string, an array or a dictionary.
pub fn len(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    match args.expect::<Spanned<Value>>(ctx, "collection") {
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
pub fn rgb(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    Value::Color(Color::Rgba(
        if let Some(string) = args.eat::<Spanned<String>>(ctx) {
            match RgbaColor::from_str(&string.v) {
                Ok(color) => color,
                Err(_) => {
                    ctx.diag(error!(string.span, "invalid color"));
                    return Value::Error;
                }
            }
        } else {
            let r = args.expect(ctx, "red component");
            let g = args.expect(ctx, "green component");
            let b = args.expect(ctx, "blue component");
            let a = args.eat(ctx);
            let mut clamp = |component: Option<Spanned<f64>>, default| {
                component.map_or(default, |c| {
                    if c.v < 0.0 || c.v > 1.0 {
                        ctx.diag(warning!(c.span, "should be between 0.0 and 1.0"));
                    }
                    (c.v.max(0.0).min(1.0) * 255.0).round() as u8
                })
            };

            RgbaColor::new(clamp(r, 0), clamp(g, 0), clamp(b, 0), clamp(a, 255))
        },
    ))
}

/// `min`: The minimum of a sequence of values.
pub fn min(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    minmax(ctx, args, Ordering::Less)
}

/// `max`: The maximum of a sequence of values.
pub fn max(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    minmax(ctx, args, Ordering::Greater)
}

/// Find the minimum or maximum of a sequence of values.
fn minmax(ctx: &mut EvalContext, args: &mut FuncArgs, goal: Ordering) -> Value {
    let mut extremum = None;

    while let Some(value) = args.eat::<Value>(ctx) {
        if let Some(prev) = &extremum {
            match value.cmp(&prev) {
                Some(ordering) if ordering == goal => extremum = Some(value),
                Some(_) => {}
                None => {
                    ctx.diag(error!(
                        args.span,
                        "cannot compare {} with {}",
                        prev.type_name(),
                        value.type_name(),
                    ));
                    return Value::Error;
                }
            }
        } else {
            extremum = Some(value);
        }
    }

    extremum.unwrap_or_else(|| {
        args.expect::<Value>(ctx, "value");
        Value::Error
    })
}
