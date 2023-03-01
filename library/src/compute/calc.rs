use std::cmp::Ordering;
use std::ops::Rem;

use typst::eval::{Module, Scope};

use crate::prelude::*;

/// A module with computational functions.
pub fn calc() -> Module {
    let mut scope = Scope::new();
    scope.def_func::<AbsFunc>("abs");
    scope.def_func::<PowFunc>("pow");
    scope.def_func::<SqrtFunc>("sqrt");
    scope.def_func::<SinFunc>("sin");
    scope.def_func::<CosFunc>("cos");
    scope.def_func::<TanFunc>("tan");
    scope.def_func::<AsinFunc>("asin");
    scope.def_func::<AcosFunc>("acos");
    scope.def_func::<AtanFunc>("atan");
    scope.def_func::<SinhFunc>("sinh");
    scope.def_func::<CoshFunc>("cosh");
    scope.def_func::<TanhFunc>("tanh");
    scope.def_func::<LogFunc>("log");
    scope.def_func::<FloorFunc>("floor");
    scope.def_func::<CeilFunc>("ceil");
    scope.def_func::<RoundFunc>("round");
    scope.def_func::<ClampFunc>("clamp");
    scope.def_func::<MinFunc>("min");
    scope.def_func::<MaxFunc>("max");
    scope.def_func::<EvenFunc>("even");
    scope.def_func::<OddFunc>("odd");
    scope.def_func::<ModFunc>("mod");
    scope.define("inf", Value::Float(f64::INFINITY));
    scope.define("nan", Value::Float(f64::NAN));
    scope.define("pi", Value::Float(std::f64::consts::PI));
    scope.define("e", Value::Float(std::f64::consts::E));
    Module::new("calc").with_scope(scope)
}

/// # Absolute
/// Calculate the absolute value of a numeric value.
///
/// ## Example
/// ```example
/// #calc.abs(-5) \
/// #calc.abs(5pt - 2cm) \
/// #calc.abs(2fr)
/// ```
///
/// ## Parameters
/// - value: `ToAbs` (positional, required)
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

/// # Power
/// Raise a value to some exponent.
///
/// ## Example
/// ```example
/// #calc.pow(2, 3)
/// ```
///
/// ## Parameters
/// - base: `Num` (positional, required)
///   The base of the power.
/// - exponent: `Num` (positional, required)
///   The exponent of the power. Must be non-negative.
///
/// ## Category
/// calculate
#[func]
pub fn pow(args: &mut Args) -> SourceResult<Value> {
    let base = args.expect::<Num>("base")?;
    let exponent = args
        .expect::<Spanned<Num>>("exponent")
        .and_then(|n| match n.v {
            Num::Int(i) if i > u32::MAX as i64 => bail!(n.span, "exponent too large"),
            Num::Int(i) if i >= 0 => Ok(n),
            Num::Float(f) if f >= 0.0 => Ok(n),
            _ => bail!(n.span, "exponent must be non-negative"),
        })?
        .v;
    Ok(base.apply2(exponent, |a, b| a.pow(b as u32), f64::powf))
}

/// # Square Root
/// Calculate the square root of a number.
///
/// ## Example
/// ```example
/// #calc.sqrt(16) \
/// #calc.sqrt(2.5)
/// ```
///
/// ## Parameters
/// - value: `Num` (positional, required)
///   The number whose square root to calculate. Must be non-negative.
///
/// ## Category
/// calculate
#[func]
pub fn sqrt(args: &mut Args) -> SourceResult<Value> {
    let value = args.expect::<Spanned<Num>>("value")?;
    if value.v.float() < 0.0 {
        bail!(value.span, "cannot take square root of negative number");
    }
    Ok(Value::Float(value.v.float().sqrt()))
}

/// # Sine
/// Calculate the sine of an angle.
///
/// When called with an integer or a float, they will be interpreted as
/// radians.
///
/// ## Example
/// ```example
/// #assert(calc.sin(90deg) == calc.sin(-270deg))
/// #calc.sin(1.5) \
/// #calc.sin(90deg)
/// ```
///
/// ## Parameters
/// - angle: `AngleLike` (positional, required)
///   The angle whose sine to calculate.
///
/// ## Category
/// calculate
#[func]
pub fn sin(args: &mut Args) -> SourceResult<Value> {
    let arg = args.expect::<AngleLike>("angle")?;
    Ok(Value::Float(match arg {
        AngleLike::Angle(a) => a.sin(),
        AngleLike::Int(n) => (n as f64).sin(),
        AngleLike::Float(n) => n.sin(),
    }))
}

/// # Cosine
/// Calculate the cosine of an angle.
///
/// When called with an integer or a float, they will be interpreted as
/// radians.
///
/// ## Example
/// ```example
/// #calc.cos(90deg)
/// #calc.cos(1.5) \
/// #calc.cos(90deg)
/// ```
///
/// ## Parameters
/// - angle: `AngleLike` (positional, required)
///   The angle whose cosine to calculate.
///
/// ## Category
/// calculate
#[func]
pub fn cos(args: &mut Args) -> SourceResult<Value> {
    let arg = args.expect::<AngleLike>("angle")?;
    Ok(Value::Float(match arg {
        AngleLike::Angle(a) => a.cos(),
        AngleLike::Int(n) => (n as f64).cos(),
        AngleLike::Float(n) => n.cos(),
    }))
}

/// # Tangent
/// Calculate the tangent of an angle.
///
/// When called with an integer or a float, they will be interpreted as
/// radians.
///
/// ## Example
/// ```example
/// #calc.tan(1.5) \
/// #calc.tan(90deg)
/// ```
///
/// ## Parameters
/// - angle: `AngleLike` (positional, required)
///   The angle whose tangent to calculate.
///
/// ## Category
/// calculate
#[func]
pub fn tan(args: &mut Args) -> SourceResult<Value> {
    let arg = args.expect::<AngleLike>("angle")?;
    Ok(Value::Float(match arg {
        AngleLike::Angle(a) => a.tan(),
        AngleLike::Int(n) => (n as f64).tan(),
        AngleLike::Float(n) => n.tan(),
    }))
}

/// # Arcsine
/// Calculate the arcsine of a number.
///
/// ## Example
/// ```example
/// #calc.asin(0) \
/// #calc.asin(1)
/// ```
///
/// ## Parameters
/// - value: `Num` (positional, required)
///   The number whose arcsine to calculate. Must be between -1 and 1.
///
/// ## Category
/// calculate
#[func]
pub fn asin(args: &mut Args) -> SourceResult<Value> {
    let Spanned { v, span } = args.expect::<Spanned<Num>>("value")?;
    let val = v.float();
    if val < -1.0 || val > 1.0 {
        bail!(span, "arcsin must be between -1 and 1");
    }
    Ok(Value::Angle(Angle::rad(val.asin())))
}

/// # Arccosine
/// Calculate the arccosine of a number.
///
/// ## Example
/// ```example
/// #calc.acos(0) \
/// #calc.acos(1)
/// ```
///
/// ## Parameters
/// - value: `Num` (positional, required)
///   The number whose arccosine to calculate. Must be between -1 and 1.
///
/// ## Category
/// calculate
#[func]
pub fn acos(args: &mut Args) -> SourceResult<Value> {
    let Spanned { v, span } = args.expect::<Spanned<Num>>("value")?;
    let val = v.float();
    if val < -1.0 || val > 1.0 {
        bail!(span, "arccos must be between -1 and 1");
    }
    Ok(Value::Angle(Angle::rad(val.acos())))
}

/// # Arctangent
/// Calculate the arctangent of a number.
///
/// ## Example
/// ```example
/// #calc.atan(0) \
/// #calc.atan(1)
/// ```
///
/// ## Parameters
/// - value: `Num` (positional, required)
///   The number whose arctangent to calculate.
///
/// ## Category
/// calculate
#[func]
pub fn atan(args: &mut Args) -> SourceResult<Value> {
    let value = args.expect::<Num>("value")?;
    Ok(Value::Angle(Angle::rad(value.float().atan())))
}

/// # Hyperbolic sine
/// Calculate the hyperbolic sine of an angle.
///
/// When called with an integer or a float, they will be interpreted as radians.
///
/// ## Example
/// ```example
/// #calc.sinh(0) \
/// #calc.sinh(45deg)
/// ```
///
/// ## Parameters
/// - angle: `AngleLike` (positional, required)
///   The angle whose hyperbolic sine to calculate.
///
/// ## Category
/// calculate
#[func]
pub fn sinh(args: &mut Args) -> SourceResult<Value> {
    let arg = args.expect::<AngleLike>("angle")?;
    Ok(Value::Float(match arg {
        AngleLike::Angle(a) => a.to_rad().sinh(),
        AngleLike::Int(n) => (n as f64).sinh(),
        AngleLike::Float(n) => n.sinh(),
    }))
}

/// # Hyperbolic cosine
/// Calculate the hyperbolic cosine of an angle.
///
/// When called with an integer or a float, they will be interpreted as radians.
///
/// ## Example
/// ```example
/// #calc.cosh(0) \
/// #calc.cosh(45deg)
/// ```
///
/// ## Parameters
/// - angle: `AngleLike` (positional, required)
///   The angle whose hyperbolic cosine to calculate.
///
/// ## Category
/// calculate
#[func]
pub fn cosh(args: &mut Args) -> SourceResult<Value> {
    let arg = args.expect::<AngleLike>("angle")?;
    Ok(Value::Float(match arg {
        AngleLike::Angle(a) => a.to_rad().cosh(),
        AngleLike::Int(n) => (n as f64).cosh(),
        AngleLike::Float(n) => n.cosh(),
    }))
}

/// # Hyperbolic tangent
/// Calculate the hyperbolic tangent of an angle.
///
/// When called with an integer or a float, they will be interpreted as radians.
///
/// ## Example
/// ```example
/// #calc.tanh(0) \
/// #calc.tanh(45deg)
/// ```
///
/// ## Parameters
/// - angle: `AngleLike` (positional, required)
///   The angle whose hyperbolic tangent to calculate.
///
/// ## Category
/// calculate
#[func]
pub fn tanh(args: &mut Args) -> SourceResult<Value> {
    let arg = args.expect::<AngleLike>("angle")?;
    Ok(Value::Float(match arg {
        AngleLike::Angle(a) => a.to_rad().tanh(),
        AngleLike::Int(n) => (n as f64).tanh(),
        AngleLike::Float(n) => n.tanh(),
    }))
}

/// # Logarithm
/// Calculate the logarithm of a number.
///
/// If the base is not specified, the logarithm is calculated in base 10.
///
/// ## Example
/// ```example
/// #calc.log(100) \
/// ```
///
/// ## Parameters
/// - value: `Num` (positional, required)
///   The number whose logarithm to calculate.
/// - base: `Num` (named)
///   The base of the logarithm.
///
/// ## Category
/// calculate
#[func]
pub fn log(args: &mut Args) -> SourceResult<Value> {
    let value = args.expect::<f64>("value")?;
    let base = args.named::<f64>("base")?.unwrap_or_else(|| 10.0);
    Ok(Value::Float(value.log(base)))
}

/// # Round down
/// Round a number down to the nearest integer.
///
/// If the number is already an integer, it is returned unchanged.
///
/// ## Example
/// ```example
/// #assert(calc.floor(3.14) == 3)
/// #assert(calc.floor(3) == 3)
/// #calc.floor(500.1)
/// ```
///
/// ## Parameters
/// - value: `Num` (positional, required)
///   The number to round down.
///
/// ## Category
/// calculate
#[func]
pub fn floor(args: &mut Args) -> SourceResult<Value> {
    let value = args.expect::<Num>("value")?;
    Ok(match value {
        Num::Int(n) => Value::Int(n),
        Num::Float(n) => Value::Int(n.floor() as i64),
    })
}

/// # Round up
/// Round a number up to the nearest integer.
///
/// If the number is already an integer, it is returned unchanged.
///
/// ## Example
/// ```example
/// #assert(calc.ceil(3.14) == 4)
/// #assert(calc.ceil(3) == 3)
/// #calc.ceil(500.1)
/// ```
///
/// ## Parameters
/// - value: `Num` (positional, required)
///   The number to round up.
///
/// ## Category
/// calculate
#[func]
pub fn ceil(args: &mut Args) -> SourceResult<Value> {
    let value = args.expect::<Num>("value")?;
    Ok(match value {
        Num::Int(n) => Value::Int(n),
        Num::Float(n) => Value::Int(n.ceil() as i64),
    })
}

/// # Round
/// Round a number to the nearest integer.
///
/// Optionally, a number of decimal places can be specified.
///
/// ## Example
/// ```example
/// #assert(calc.round(3.14) == 3)
/// #assert(calc.round(3.5) == 4)
/// #calc.round(3.1415, digits: 2)
/// ```
///
/// ## Parameters
/// - value: `Num` (positional, required)
///   The number to round.
/// - digits: `i64` (named)
///
/// ## Category
/// calculate
#[func]
pub fn round(args: &mut Args) -> SourceResult<Value> {
    let value = args.expect::<Num>("value")?;
    let digits = args.named::<u32>("digits")?.unwrap_or(0);
    Ok(match value {
        Num::Int(n) if digits == 0 => Value::Int(n),
        _ => {
            let n = value.float();
            let factor = 10.0_f64.powi(digits as i32) as f64;
            Value::Float((n * factor).round() / factor)
        }
    })
}

/// # Clamp
/// Clamp a number between a minimum and maximum value.
///
/// ## Example
/// ```example
/// #assert(calc.clamp(5, 0, 10) == 5)
/// #assert(calc.clamp(5, 6, 10) == 6)
/// #calc.clamp(5, 0, 4)
/// ```
///
/// ## Parameters
/// - value: `Num` (positional, required)
///   The number to clamp.
/// - min: `Num` (positional, required)
///   The inclusive minimum value.
/// - max: `Num` (positional, required)
///   The inclusive maximum value.
///
/// ## Category
/// calculate
#[func]
pub fn clamp(args: &mut Args) -> SourceResult<Value> {
    let value = args.expect::<Num>("value")?;
    let min = args.expect::<Num>("min")?;
    let max = args.expect::<Spanned<Num>>("max")?;
    if max.v.float() < min.float() {
        bail!(max.span, "max must be greater than or equal to min")
    }
    Ok(value.apply3(min, max.v, i64::clamp, f64::clamp))
}

/// # Minimum
/// Determine the minimum of a sequence of values.
///
/// ## Example
/// ```example
/// #calc.min(1, -3, -5, 20, 3, 6) \
/// #calc.min("typst", "in", "beta")
/// ```
///
/// ## Parameters
/// - values: `Value` (positional, variadic)
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
/// Determine the maximum of a sequence of values.
///
/// ## Example
/// ```example
/// #calc.max(1, -3, -5, 20, 3, 6) \
/// #calc.max("typst", "in", "beta")
/// ```
///
/// ## Parameters
/// - values: `Value` (positional, variadic)
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
/// Determine whether an integer is even.
///
/// ## Example
/// ```example
/// #calc.even(4) \
/// #calc.even(5) \
/// #range(10).filter(calc.even)
/// ```
///
/// ## Parameters
/// - value: `i64` (positional, required)
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
/// Determine whether an integer is odd.
///
/// ## Example
/// ```example
/// #calc.odd(4) \
/// #calc.odd(5) \
/// #range(10).filter(calc.odd)
/// ```
///
///
/// ## Parameters
/// - value: `i64` (positional, required)
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
/// Calculate the modulus of two numbers.
///
/// ## Example
/// ```example
/// #calc.mod(20, 6) \
/// #calc.mod(1.75, 0.5)
/// ```
///
/// ## Parameters
/// - dividend: `Num` (positional, required)
///   The dividend of the modulus.
///
/// - divisor: `Num` (positional, required)
///   The divisor of the modulus.
///
/// - returns: integer or float
///
/// ## Category
/// calculate
#[func]
pub fn mod_(args: &mut Args) -> SourceResult<Value> {
    let dividend = args.expect::<Num>("dividend")?;
    let Spanned { v: divisor, span } = args.expect::<Spanned<Num>>("divisor")?;
    if divisor.float() == 0.0 {
        bail!(span, "divisor must not be zero");
    }
    Ok(dividend.apply2(divisor, Rem::rem, Rem::rem))
}

/// A value which can be passed to functions that work with integers and floats.
#[derive(Debug, Copy, Clone)]
enum Num {
    Int(i64),
    Float(f64),
}

impl Num {
    fn apply2(
        self,
        other: Self,
        int: impl FnOnce(i64, i64) -> i64,
        float: impl FnOnce(f64, f64) -> f64,
    ) -> Value {
        match (self, other) {
            (Self::Int(a), Self::Int(b)) => Value::Int(int(a, b)),
            (a, b) => Value::Float(float(a.float(), b.float())),
        }
    }

    fn apply3(
        self,
        other: Self,
        third: Self,
        int: impl FnOnce(i64, i64, i64) -> i64,
        float: impl FnOnce(f64, f64, f64) -> f64,
    ) -> Value {
        match (self, other, third) {
            (Self::Int(a), Self::Int(b), Self::Int(c)) => Value::Int(int(a, b, c)),
            (a, b, c) => Value::Float(float(a.float(), b.float(), c.float())),
        }
    }

    fn float(self) -> f64 {
        match self {
            Self::Int(v) => v as f64,
            Self::Float(v) => v,
        }
    }
}

castable! {
    Num,
    v: i64 => Self::Int(v),
    v: f64 => Self::Float(v),
}

/// A value that can be passed to a trigonometric function.
enum AngleLike {
    Int(i64),
    Float(f64),
    Angle(Angle),
}

castable! {
    AngleLike,
    v: i64 => Self::Int(v),
    v: f64 => Self::Float(v),
    v: Angle => Self::Angle(v),
}
