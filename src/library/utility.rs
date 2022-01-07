//! Computational utility functions.

use std::cmp::Ordering;
use std::str::FromStr;

use super::prelude::*;
use crate::eval::Array;

/// Ensure that a condition is fulfilled.
pub fn assert(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let Spanned { v, span } = args.expect::<Spanned<bool>>("condition")?;
    if !v {
        bail!(span, "assertion failed");
    }
    Ok(Value::None)
}

/// The name of a value's type.
pub fn type_(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    Ok(args.expect::<Value>("value")?.type_name().into())
}

/// The string representation of a value.
pub fn repr(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    Ok(args.expect::<Value>("value")?.repr().into())
}

/// Join a sequence of values, optionally interspersing it with another value.
pub fn join(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let span = args.span;
    let sep = args.named::<Value>("sep")?.unwrap_or(Value::None);

    let mut result = Value::None;
    let mut iter = args.all::<Value>();

    if let Some(first) = iter.next() {
        result = first;
    }

    for value in iter {
        result = result.join(sep.clone()).at(span)?;
        result = result.join(value).at(span)?;
    }

    Ok(result)
}

/// Convert a value to a integer.
pub fn int(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let Spanned { v, span } = args.expect("value")?;
    Ok(Value::Int(match v {
        Value::Bool(v) => v as i64,
        Value::Int(v) => v,
        Value::Float(v) => v as i64,
        Value::Str(v) => match v.parse() {
            Ok(v) => v,
            Err(_) => bail!(span, "invalid integer"),
        },
        v => bail!(span, "cannot convert {} to integer", v.type_name()),
    }))
}

/// Convert a value to a float.
pub fn float(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let Spanned { v, span } = args.expect("value")?;
    Ok(Value::Float(match v {
        Value::Int(v) => v as f64,
        Value::Float(v) => v,
        Value::Str(v) => match v.parse() {
            Ok(v) => v,
            Err(_) => bail!(span, "invalid float"),
        },
        v => bail!(span, "cannot convert {} to float", v.type_name()),
    }))
}

/// Try to convert a value to a string.
pub fn str(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let Spanned { v, span } = args.expect("value")?;
    Ok(Value::Str(match v {
        Value::Int(v) => format_eco!("{}", v),
        Value::Float(v) => format_eco!("{}", v),
        Value::Str(v) => v,
        v => bail!(span, "cannot convert {} to string", v.type_name()),
    }))
}

/// Create an RGB(A) color.
pub fn rgb(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    Ok(Value::from(
        if let Some(string) = args.find::<Spanned<EcoString>>() {
            match RgbaColor::from_str(&string.v) {
                Ok(color) => color,
                Err(_) => bail!(string.span, "invalid hex string"),
            }
        } else {
            let r = args.expect("red component")?;
            let g = args.expect("green component")?;
            let b = args.expect("blue component")?;
            let a = args.eat()?.unwrap_or(Spanned::new(1.0, Span::detached()));
            let f = |Spanned { v, span }: Spanned<f64>| {
                if (0.0 ..= 1.0).contains(&v) {
                    Ok((v * 255.0).round() as u8)
                } else {
                    bail!(span, "value must be between 0.0 and 1.0");
                }
            };
            RgbaColor::new(f(r)?, f(g)?, f(b)?, f(a)?)
        },
    ))
}

/// The absolute value of a numeric value.
pub fn abs(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let Spanned { v, span } = args.expect("numeric value")?;
    Ok(match v {
        Value::Int(v) => Value::Int(v.abs()),
        Value::Float(v) => Value::Float(v.abs()),
        Value::Length(v) => Value::Length(v.abs()),
        Value::Angle(v) => Value::Angle(v.abs()),
        Value::Relative(v) => Value::Relative(v.abs()),
        Value::Fractional(v) => Value::Fractional(v.abs()),
        Value::Linear(_) => bail!(span, "cannot take absolute value of a linear"),
        v => bail!(span, "expected numeric value, found {}", v.type_name()),
    })
}

/// The minimum of a sequence of values.
pub fn min(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    minmax(args, Ordering::Less)
}

/// The maximum of a sequence of values.
pub fn max(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    minmax(args, Ordering::Greater)
}

/// Find the minimum or maximum of a sequence of values.
fn minmax(args: &mut Args, goal: Ordering) -> TypResult<Value> {
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

/// Create a sequence of numbers.
pub fn range(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let first = args.expect::<i64>("end")?;
    let (start, end) = match args.eat::<i64>()? {
        Some(second) => (first, second),
        None => (0, first),
    };

    let step: i64 = match args.named("step")? {
        Some(Spanned { v: 0, span }) => bail!(span, "step must not be zero"),
        Some(Spanned { v, .. }) => v,
        None => 1,
    };

    let mut x = start;
    let mut seq = vec![];

    while x.cmp(&end) == 0.cmp(&step) {
        seq.push(Value::Int(x));
        x += step;
    }

    Ok(Value::Array(Array::from_vec(seq)))
}

/// Convert a string to lowercase.
pub fn lower(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    Ok(args.expect::<EcoString>("string")?.to_lowercase().into())
}

/// Convert a string to uppercase.
pub fn upper(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    Ok(args.expect::<EcoString>("string")?.to_uppercase().into())
}

/// The length of a string, an array or a dictionary.
pub fn len(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let Spanned { v, span } = args.expect("collection")?;
    Ok(Value::Int(match v {
        Value::Str(v) => v.len() as i64,
        Value::Array(v) => v.len(),
        Value::Dict(v) => v.len(),
        v => bail!(
            span,
            "expected string, array or dictionary, found {}",
            v.type_name(),
        ),
    }))
}

/// The sorted version of an array.
pub fn sorted(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let Spanned { v, span } = args.expect::<Spanned<Array>>("array")?;
    Ok(Value::Array(v.sorted().at(span)?))
}
