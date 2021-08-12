use std::cmp::Ordering;
use std::str::FromStr;

use crate::color::{Color, RgbaColor};
use crate::pretty::pretty;

use super::*;

/// `type`: The name of a value's type.
pub fn type_(_: &mut EvalContext, args: &mut FuncArgs) -> TypResult<Value> {
    let value = args.expect::<Value>("value")?;
    Ok(value.type_name().into())
}

/// `repr`: The string representation of a value.
pub fn repr(_: &mut EvalContext, args: &mut FuncArgs) -> TypResult<Value> {
    let value = args.expect::<Value>("value")?;
    Ok(pretty(&value).into())
}

/// `len`: The length of a string, an array or a dictionary.
pub fn len(_: &mut EvalContext, args: &mut FuncArgs) -> TypResult<Value> {
    let Spanned { v, span } = args.expect("collection")?;
    Ok(match v {
        Value::Str(v) => Value::Int(v.len() as i64),
        Value::Array(v) => Value::Int(v.len()),
        Value::Dict(v) => Value::Int(v.len()),
        _ => bail!(args.source, span, "expected string, array or dictionary"),
    })
}

/// `rgb`: Create an RGB(A) color.
pub fn rgb(_: &mut EvalContext, args: &mut FuncArgs) -> TypResult<Value> {
    Ok(Value::Color(Color::Rgba(
        if let Some(string) = args.eat::<Spanned<EcoString>>() {
            match RgbaColor::from_str(&string.v) {
                Ok(color) => color,
                Err(_) => bail!(args.source, string.span, "invalid color"),
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

/// `min`: The minimum of a sequence of values.
pub fn min(_: &mut EvalContext, args: &mut FuncArgs) -> TypResult<Value> {
    minmax(args, Ordering::Less)
}

/// `max`: The maximum of a sequence of values.
pub fn max(_: &mut EvalContext, args: &mut FuncArgs) -> TypResult<Value> {
    minmax(args, Ordering::Greater)
}

/// Find the minimum or maximum of a sequence of values.
fn minmax(args: &mut FuncArgs, goal: Ordering) -> TypResult<Value> {
    let &mut FuncArgs { source, span, .. } = args;

    let mut extremum = args.expect::<Value>("value")?;
    for value in args.all::<Value>() {
        match value.partial_cmp(&extremum) {
            Some(ordering) => {
                if ordering == goal {
                    extremum = value;
                }
            }
            None => bail!(
                source,
                span,
                "cannot compare {} with {}",
                extremum.type_name(),
                value.type_name(),
            ),
        }
    }

    Ok(extremum)
}
