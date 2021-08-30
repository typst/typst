use std::cmp::Ordering;
use std::str::FromStr;

use crate::color::{Color, RgbaColor};

use super::*;

/// `type`: The name of a value's type.
pub fn type_(_: &mut EvalContext, args: &mut Arguments) -> TypResult<Value> {
    Ok(args.expect::<Value>("value")?.type_name().into())
}

/// `repr`: The string representation of a value.
pub fn repr(_: &mut EvalContext, args: &mut Arguments) -> TypResult<Value> {
    Ok(args.expect::<Value>("value")?.to_string().into())
}

/// `len`: The length of a string, an array or a dictionary.
pub fn len(_: &mut EvalContext, args: &mut Arguments) -> TypResult<Value> {
    let Spanned { v, span } = args.expect("collection")?;
    Ok(Value::Int(match v {
        Value::Str(v) => v.len(),
        Value::Array(v) => v.len(),
        Value::Dict(v) => v.len(),
        _ => bail!(span, "expected string, array or dictionary"),
    }))
}

/// `rgb`: Create an RGB(A) color.
pub fn rgb(_: &mut EvalContext, args: &mut Arguments) -> TypResult<Value> {
    Ok(Value::Color(Color::Rgba(
        if let Some(string) = args.eat::<Spanned<Str>>() {
            match RgbaColor::from_str(&string.v) {
                Ok(color) => color,
                Err(_) => bail!(string.span, "invalid color"),
            }
        } else {
            let r = args.expect("red component")?;
            let g = args.expect("green component")?;
            let b = args.expect("blue component")?;
            let a = args.eat().unwrap_or(1.0);
            let f = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
            RgbaColor::new(f(r), f(g), f(b), f(a))
        },
    )))
}

/// `abs`: The absolute value of a numeric value.
pub fn abs(_: &mut EvalContext, args: &mut Arguments) -> TypResult<Value> {
    let Spanned { v, span } = args.expect("numeric value")?;
    Ok(match v {
        Value::Int(v) => Value::Int(v.abs()),
        Value::Float(v) => Value::Float(v.abs()),
        Value::Length(v) => Value::Length(v.abs()),
        Value::Angle(v) => Value::Angle(v.abs()),
        Value::Relative(v) => Value::Relative(v.abs()),
        Value::Fractional(v) => Value::Fractional(v.abs()),
        Value::Linear(_) => bail!(span, "cannot take absolute value of a linear"),
        _ => bail!(span, "expected numeric value"),
    })
}

/// `min`: The minimum of a sequence of values.
pub fn min(_: &mut EvalContext, args: &mut Arguments) -> TypResult<Value> {
    minmax(args, Ordering::Less)
}

/// `max`: The maximum of a sequence of values.
pub fn max(_: &mut EvalContext, args: &mut Arguments) -> TypResult<Value> {
    minmax(args, Ordering::Greater)
}

/// Find the minimum or maximum of a sequence of values.
fn minmax(args: &mut Arguments, goal: Ordering) -> TypResult<Value> {
    let mut extremum = args.expect::<Value>("value")?;
    for Spanned { v, span } in args.all::<Spanned<Value>>() {
        match v.partial_cmp(&extremum) {
            Some(ordering) => {
                if ordering == goal {
                    extremum = v;
                }
            }
            None => bail!(
                span,
                "cannot compare {} with {}",
                extremum.type_name(),
                v.type_name(),
            ),
        }
    }
    Ok(extremum)
}

/// `lower`: Convert a string to lowercase.
pub fn lower(_: &mut EvalContext, args: &mut Arguments) -> TypResult<Value> {
    Ok(args.expect::<Str>("string")?.to_lowercase().into())
}

/// `upper`: Convert a string to uppercase.
pub fn upper(_: &mut EvalContext, args: &mut Arguments) -> TypResult<Value> {
    Ok(args.expect::<Str>("string")?.to_uppercase().into())
}
