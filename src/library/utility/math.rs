use std::cmp::Ordering;

use crate::library::prelude::*;

/// The absolute value of a numeric value.
pub fn abs(_: &mut Context, args: &mut Args) -> TypResult<Value> {
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
pub fn min(_: &mut Context, args: &mut Args) -> TypResult<Value> {
    minmax(args, Ordering::Less)
}

/// The maximum of a sequence of values.
pub fn max(_: &mut Context, args: &mut Args) -> TypResult<Value> {
    minmax(args, Ordering::Greater)
}

/// Find the minimum or maximum of a sequence of values.
fn minmax(args: &mut Args, goal: Ordering) -> TypResult<Value> {
    let mut extremum = args.expect::<Value>("value")?;
    for Spanned { v, span } in args.all::<Spanned<Value>>()? {
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

/// Whether an integer is even.
pub fn even(_: &mut Context, args: &mut Args) -> TypResult<Value> {
    Ok(Value::Bool(args.expect::<i64>("integer")? % 2 == 0))
}

/// Whether an integer is odd.
pub fn odd(_: &mut Context, args: &mut Args) -> TypResult<Value> {
    Ok(Value::Bool(args.expect::<i64>("integer")? % 2 != 0))
}

/// The modulo of two numbers.
pub fn modulo(_: &mut Context, args: &mut Args) -> TypResult<Value> {
    let Spanned { v: v1, span: span1 } = args.expect("integer or float")?;
    let Spanned { v: v2, span: span2 } = args.expect("integer or float")?;

    let (a, b) = match (v1, v2) {
        (Value::Int(a), Value::Int(b)) => match a.checked_rem(b) {
            Some(res) => return Ok(Value::Int(res)),
            None => bail!(span2, "divisor must not be zero"),
        },
        (Value::Int(a), Value::Float(b)) => (a as f64, b),
        (Value::Float(a), Value::Int(b)) => (a, b as f64),
        (Value::Float(a), Value::Float(b)) => (a, b),
        (Value::Int(_), b) | (Value::Float(_), b) => bail!(
            span2,
            format!("expected integer or float, found {}", b.type_name())
        ),
        (a, _) => bail!(
            span1,
            format!("expected integer or float, found {}", a.type_name())
        ),
    };

    if b == 0.0 {
        bail!(span2, "divisor must not be zero");
    }

    Ok(Value::Float(a % b))
}

/// Create a sequence of numbers.
pub fn range(_: &mut Context, args: &mut Args) -> TypResult<Value> {
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
