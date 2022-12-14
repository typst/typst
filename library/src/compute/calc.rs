use std::cmp::Ordering;

use crate::prelude::*;

/// The absolute value of a numeric value.
#[func]
pub fn abs(args: &mut Args) -> SourceResult<Value> {
    let Spanned { v, span } = args.expect("numeric value")?;
    Ok(match v {
        Value::Int(v) => Value::Int(v.abs()),
        Value::Float(v) => Value::Float(v.abs()),
        Value::Angle(v) => Value::Angle(v.abs()),
        Value::Ratio(v) => Value::Ratio(v.abs()),
        Value::Fraction(v) => Value::Fraction(v.abs()),
        Value::Length(_) | Value::Relative(_) => {
            bail!(span, "cannot take absolute value of a length")
        }
        v => bail!(span, "expected numeric value, found {}", v.type_name()),
    })
}

/// The minimum of a sequence of values.
#[func]
pub fn min(args: &mut Args) -> SourceResult<Value> {
    minmax(args, Ordering::Less)
}

/// The maximum of a sequence of values.
#[func]
pub fn max(args: &mut Args) -> SourceResult<Value> {
    minmax(args, Ordering::Greater)
}

/// Find the minimum or maximum of a sequence of values.
fn minmax(args: &mut Args, goal: Ordering) -> SourceResult<Value> {
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
                "cannot compare {} and {}",
                extremum.type_name(),
                v.type_name(),
            ),
        }
    }
    Ok(extremum)
}

/// Whether an integer is even.
#[func]
pub fn even(args: &mut Args) -> SourceResult<Value> {
    Ok(Value::Bool(args.expect::<i64>("integer")? % 2 == 0))
}

/// Whether an integer is odd.
#[func]
pub fn odd(args: &mut Args) -> SourceResult<Value> {
    Ok(Value::Bool(args.expect::<i64>("integer")? % 2 != 0))
}

/// The modulo of two numbers.
#[func]
pub fn mod_(args: &mut Args) -> SourceResult<Value> {
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
        (Value::Int(_), b) | (Value::Float(_), b) => {
            bail!(span2, format!("expected integer or float, found {}", b.type_name()))
        }
        (a, _) => {
            bail!(span1, format!("expected integer or float, found {}", a.type_name()))
        }
    };

    if b == 0.0 {
        bail!(span2, "divisor must not be zero");
    }

    Ok(Value::Float(a % b))
}
