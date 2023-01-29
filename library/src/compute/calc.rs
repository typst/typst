use std::cmp::Ordering;

use crate::prelude::*;

/// # Absolute
/// The absolute value of a numeric value.
///
/// ## Example
/// ```
/// #abs(-5) \
/// #abs(5pt - 2cm) \
/// #abs(2fr)
/// ```
///
/// ## Parameters
/// - value: ToAbs (positional, required)
///   The value whose absolute value to calculate.
///
/// ## Category
/// calculate
#[func]
pub fn abs(args: &mut Args) -> SourceResult<Value> {
    Ok(args.expect::<ToAbs>("value")?.0)
}

/// A value of which the absolute value can be taken.
struct ToAbs(Value);

castable! {
    ToAbs,
    v: i64 => Self(Value::Int(v.abs())),
    v: f64 => Self(Value::Float(v.abs())),
    v: Length => Self(Value::Length(v.try_abs()
        .ok_or_else(|| "cannot take absolute value of this length")?)),
    v: Angle => Self(Value::Angle(v.abs())),
    v: Ratio => Self(Value::Ratio(v.abs())),
    v: Fr => Self(Value::Fraction(v.abs())),
}

/// # Minimum
/// The minimum of a sequence of values.
///
/// ## Example
/// ```
/// #min(1, -3, -5, 20, 3, 6) \
/// #min("typst", "in", "beta")
/// ```
///
/// ## Parameters
/// - values: Value (positional, variadic)
///   The sequence of values from which to extract the minimum.
///   Must not be empty.
///
/// - returns: any
///
/// ## Category
/// calculate
#[func]
pub fn min(args: &mut Args) -> SourceResult<Value> {
    minmax(args, Ordering::Less)
}

/// # Maximum
/// The maximum of a sequence of values.
///
/// ## Example
/// ```
/// #max(1, -3, -5, 20, 3, 6) \
/// #max("typst", "in", "beta")
/// ```
///
/// ## Parameters
/// - values: Value (positional, variadic)
///   The sequence of values from which to extract the maximum.
///   Must not be empty.
///
/// - returns: any
///
/// ## Category
/// calculate
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

/// # Even
/// Whether an integer is even.
///
/// ## Example
/// ```
/// #even(4) \
/// #even(5) \
/// #range(10).filter(even)
/// ```
///
/// ## Parameters
/// - value: i64 (positional, required)
///   The number to check for evenness.
///
/// - returns: boolean
///
/// ## Category
/// calculate
#[func]
pub fn even(args: &mut Args) -> SourceResult<Value> {
    Ok(Value::Bool(args.expect::<i64>("value")? % 2 == 0))
}

/// # Odd
/// Whether an integer is odd.
///
/// ## Example
/// ```
/// #odd(4) \
/// #odd(5) \
/// #range(10).filter(odd)
/// ```
///
///
/// ## Parameters
/// - value: i64 (positional, required)
///   The number to check for oddness.
///
/// - returns: boolean
///
/// ## Category
/// calculate
#[func]
pub fn odd(args: &mut Args) -> SourceResult<Value> {
    Ok(Value::Bool(args.expect::<i64>("value")? % 2 != 0))
}

/// # Modulus
/// The modulus of two numbers.
///
/// ## Example
/// ```
/// #mod(20, 6) \
/// #mod(1.75, 0.5)
/// ```
///
/// ## Parameters
/// - dividend: ToMod (positional, required)
///   The dividend of the modulus.
///
/// - divisor: ToMod (positional, required)
///   The divisor of the modulus.
///
/// - returns: integer or float
///
/// ## Category
/// calculate
#[func]
pub fn mod_(args: &mut Args) -> SourceResult<Value> {
    let Spanned { v: v1, span: span1 } = args.expect("dividend")?;
    let Spanned { v: v2, span: span2 } = args.expect("divisor")?;

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

/// A value which can be passed to the `mod` function.
struct ToMod;

castable! {
    ToMod,
    _: i64 => Self,
    _: f64 => Self,
}
