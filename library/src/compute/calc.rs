//! Calculations and processing of numeric values.

use std::cmp::Ordering;
use std::ops::Rem;

use typst::eval::{Module, Scope};

use crate::prelude::*;

/// A module with computational functions.
pub fn module() -> Module {
    let mut scope = Scope::new();
    scope.define("abs", abs);
    scope.define("pow", pow);
    scope.define("sqrt", sqrt);
    scope.define("sin", sin);
    scope.define("cos", cos);
    scope.define("tan", tan);
    scope.define("asin", asin);
    scope.define("acos", acos);
    scope.define("atan", atan);
    scope.define("sinh", sinh);
    scope.define("cosh", cosh);
    scope.define("tanh", tanh);
    scope.define("log", log);
    scope.define("floor", floor);
    scope.define("ceil", ceil);
    scope.define("round", round);
    scope.define("clamp", clamp);
    scope.define("min", min);
    scope.define("max", max);
    scope.define("even", even);
    scope.define("odd", odd);
    scope.define("mod", mod_);
    scope.define("inf", Value::Float(f64::INFINITY));
    scope.define("nan", Value::Float(f64::NAN));
    scope.define("pi", Value::Float(std::f64::consts::PI));
    scope.define("e", Value::Float(std::f64::consts::E));
    Module::new("calc").with_scope(scope)
}

/// Calculate the absolute value of a numeric value.
///
/// ## Example
/// ```example
/// #calc.abs(-5) \
/// #calc.abs(5pt - 2cm) \
/// #calc.abs(2fr)
/// ```
///
/// Display: Absolute
/// Category: calculate
/// Returns: any
#[func]
pub fn abs(
    /// The value whose absolute value to calculate.
    value: ToAbs,
) -> Value {
    value.0
}

/// A value of which the absolute value can be taken.
struct ToAbs(Value);

cast_from_value! {
    ToAbs,
    v: i64 => Self(Value::Int(v.abs())),
    v: f64 => Self(Value::Float(v.abs())),
    v: Length => Self(Value::Length(v.try_abs()
        .ok_or_else(|| "cannot take absolute value of this length")?)),
    v: Angle => Self(Value::Angle(v.abs())),
    v: Ratio => Self(Value::Ratio(v.abs())),
    v: Fr => Self(Value::Fraction(v.abs())),
}

/// Raise a value to some exponent.
///
/// ## Example
/// ```example
/// #calc.pow(2, 3)
/// ```
///
/// Display: Power
/// Category: calculate
/// Returns: integer or float
#[func]
pub fn pow(
    /// The base of the power.
    base: Num,
    /// The exponent of the power. Must be non-negative.
    exponent: Spanned<Num>,
) -> Value {
    let Spanned { v: exp, span } = exponent;
    match exp {
        _ if exp.float() == 0.0 && base.float() == 0.0 => {
            bail!(args.span, "zero to the power of zero is undefined")
        }
        Num::Int(i) if i32::try_from(i).is_err() => {
            bail!(span, "exponent is too large")
        }
        Num::Float(f) if !f.is_normal() && f != 0.0 => {
            bail!(span, "exponent may not be NaN, infinite, or subnormal")
        }
        _ => {}
    };

    let return_value = match (base, exp) {
        (Num::Int(a), Num::Int(b)) if b >= 0 => Value::Int(a.pow(b as u32)),
        (a, Num::Int(b)) => Value::Float(a.float().powi(b as i32)),
        (a, b) => Value::Float(a.float().powf(b.float())),
    };

    let is_nan = match return_value {
        Value::Float(f) => f.is_nan(),
        Value::Int(i) => (i as f64).is_nan(),
        _ => false
    };

    if is_nan {
        bail!(span, "the return value is not a real number")
    }

    return_value
}

/// Calculate the square root of a number.
///
/// ## Example
/// ```example
/// #calc.sqrt(16) \
/// #calc.sqrt(2.5)
/// ```
///
/// Display: Square Root
/// Category: calculate
/// Returns: float
#[func]
pub fn sqrt(
    /// The number whose square root to calculate. Must be non-negative.
    value: Spanned<Num>,
) -> Value {
    if value.v.float() < 0.0 {
        bail!(value.span, "cannot take square root of negative number");
    }
    Value::Float(value.v.float().sqrt())
}

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
/// Display: Sine
/// Category: calculate
/// Returns: float
#[func]
pub fn sin(
    /// The angle whose sine to calculate.
    angle: AngleLike,
) -> Value {
    Value::Float(match angle {
        AngleLike::Angle(a) => a.sin(),
        AngleLike::Int(n) => (n as f64).sin(),
        AngleLike::Float(n) => n.sin(),
    })
}

/// Calculate the cosine of an angle.
///
/// When called with an integer or a float, they will be interpreted as
/// radians.
///
/// ## Example
/// ```example
/// #calc.cos(90deg) \
/// #calc.cos(1.5) \
/// #calc.cos(90deg)
/// ```
///
/// Display: Cosine
/// Category: calculate
/// Returns: float
#[func]
pub fn cos(
    /// The angle whose cosine to calculate.
    angle: AngleLike,
) -> Value {
    Value::Float(match angle {
        AngleLike::Angle(a) => a.cos(),
        AngleLike::Int(n) => (n as f64).cos(),
        AngleLike::Float(n) => n.cos(),
    })
}

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
/// Display: Tangent
/// Category: calculate
/// Returns: float
#[func]
pub fn tan(
    /// The angle whose tangent to calculate.
    angle: AngleLike,
) -> Value {
    Value::Float(match angle {
        AngleLike::Angle(a) => a.tan(),
        AngleLike::Int(n) => (n as f64).tan(),
        AngleLike::Float(n) => n.tan(),
    })
}

/// Calculate the arcsine of a number.
///
/// ## Example
/// ```example
/// #calc.asin(0) \
/// #calc.asin(1)
/// ```
///
/// Display: Arcsine
/// Category: calculate
/// Returns: angle
#[func]
pub fn asin(
    /// The number whose arcsine to calculate. Must be between -1 and 1.
    value: Spanned<Num>,
) -> Value {
    let val = value.v.float();
    if val < -1.0 || val > 1.0 {
        bail!(value.span, "arcsin must be between -1 and 1");
    }
    Value::Angle(Angle::rad(val.asin()))
}

/// Calculate the arccosine of a number.
///
/// ## Example
/// ```example
/// #calc.acos(0) \
/// #calc.acos(1)
/// ```
///
/// Display: Arccosine
/// Category: calculate
/// Returns: angle
#[func]
pub fn acos(
    /// The number whose arcsine to calculate. Must be between -1 and 1.
    value: Spanned<Num>,
) -> Value {
    let val = value.v.float();
    if val < -1.0 || val > 1.0 {
        bail!(value.span, "arccos must be between -1 and 1");
    }
    Value::Angle(Angle::rad(val.acos()))
}

/// Calculate the arctangent of a number.
///
/// ## Example
/// ```example
/// #calc.atan(0) \
/// #calc.atan(1)
/// ```
///
/// Display: Arctangent
/// Category: calculate
/// Returns: angle
#[func]
pub fn atan(
    /// The number whose arctangent to calculate.
    value: Num,
) -> Value {
    Value::Angle(Angle::rad(value.float().atan()))
}

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
/// Display: Hyperbolic sine
/// Category: calculate
/// Returns: float
#[func]
pub fn sinh(
    /// The angle whose hyperbolic sine to calculate.
    angle: AngleLike,
) -> Value {
    Value::Float(match angle {
        AngleLike::Angle(a) => a.to_rad().sinh(),
        AngleLike::Int(n) => (n as f64).sinh(),
        AngleLike::Float(n) => n.sinh(),
    })
}

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
/// Display: Hyperbolic cosine
/// Category: calculate
/// Returns: float
#[func]
pub fn cosh(
    /// The angle whose hyperbolic cosine to calculate.
    angle: AngleLike,
) -> Value {
    Value::Float(match angle {
        AngleLike::Angle(a) => a.to_rad().cosh(),
        AngleLike::Int(n) => (n as f64).cosh(),
        AngleLike::Float(n) => n.cosh(),
    })
}

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
/// Display: Hyperbolic tangent
/// Category: calculate
/// Returns: float
#[func]
pub fn tanh(
    /// The angle whose hyperbolic tangent to calculate.
    angle: AngleLike,
) -> Value {
    Value::Float(match angle {
        AngleLike::Angle(a) => a.to_rad().tanh(),
        AngleLike::Int(n) => (n as f64).tanh(),
        AngleLike::Float(n) => n.tanh(),
    })
}

/// Calculate the logarithm of a number.
///
/// If the base is not specified, the logarithm is calculated in base 10.
///
/// ## Example
/// ```example
/// #calc.log(100)
/// ```
///
/// Display: Logarithm
/// Category: calculate
/// Returns: float
#[func]
pub fn log(
    /// The number whose logarithm to calculate.
    value: Spanned<Num>,
    /// The base of the logarithm.
    #[named]
    #[default(10.0)]
    base: f64,
) -> Value {
    let number = value.v.float();

    if number <= 0 as f64 {
        bail!(value.span, "a logarithm parameter must be strictly positive")
    }
    if base == 0 as f64 {
        bail!(value.span, "a logarithm base cannot be null")
    }

    let return_value = if base == 2.0 {
        number.log2()
    } else if base == 10.0 {
        number.log10()
    } else {
        number.log(base)
    };
    
    if return_value.is_subnormal() || return_value.is_nan() {
        bail!(value.span, "this logarithm doesn't return a real value")
    }

    Value::Float(return_value)
}

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
/// Display: Round down
/// Category: calculate
/// Returns: integer
#[func]
pub fn floor(
    /// The number to round down.
    value: Num,
) -> Value {
    match value {
        Num::Int(n) => Value::Int(n),
        Num::Float(n) => Value::Int(n.floor() as i64),
    }
}

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
/// Display: Round up
/// Category: calculate
/// Returns: integer
#[func]
pub fn ceil(
    /// The number to round up.
    value: Num,
) -> Value {
    match value {
        Num::Int(n) => Value::Int(n),
        Num::Float(n) => Value::Int(n.ceil() as i64),
    }
}

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
/// Display: Round
/// Category: calculate
/// Returns: integer or float
#[func]
pub fn round(
    /// The number to round.
    value: Num,
    /// The number of decimal places.
    #[named]
    #[default(0)]
    digits: i64,
) -> Value {
    match value {
        Num::Int(n) if digits == 0 => Value::Int(n),
        _ => {
            let n = value.float();
            let factor = 10.0_f64.powi(digits as i32);
            Value::Float((n * factor).round() / factor)
        }
    }
}

/// Clamp a number between a minimum and maximum value.
///
/// ## Example
/// ```example
/// #assert(calc.clamp(5, 0, 10) == 5)
/// #assert(calc.clamp(5, 6, 10) == 6)
/// #calc.clamp(5, 0, 4)
/// ```
///
/// Display: Clamp
/// Category: calculate
/// Returns: integer or float
#[func]
pub fn clamp(
    /// The number to clamp.
    value: Num,
    /// The inclusive minimum value.
    min: Num,
    /// The inclusive maximum value.
    max: Spanned<Num>,
) -> Value {
    if max.v.float() < min.float() {
        bail!(max.span, "max must be greater than or equal to min")
    }
    value.apply3(min, max.v, i64::clamp, f64::clamp)
}

/// Determine the minimum of a sequence of values.
///
/// ## Example
/// ```example
/// #calc.min(1, -3, -5, 20, 3, 6) \
/// #calc.min("typst", "in", "beta")
/// ```
///
/// Display: Minimum
/// Category: calculate
/// Returns: any
#[func]
pub fn min(
    /// The sequence of values from which to extract the minimum.
    /// Must not be empty.
    #[variadic]
    values: Vec<Spanned<Value>>,
) -> Value {
    minmax(args.span, values, Ordering::Less)?
}

/// Determine the maximum of a sequence of values.
///
/// ## Example
/// ```example
/// #calc.max(1, -3, -5, 20, 3, 6) \
/// #calc.max("typst", "in", "beta")
/// ```
///
/// Display: Maximum
/// Category: calculate
/// Returns: any
#[func]
pub fn max(
    /// The sequence of values from which to extract the maximum.
    /// Must not be empty.
    #[variadic]
    values: Vec<Spanned<Value>>,
) -> Value {
    minmax(args.span, values, Ordering::Greater)?
}

/// Find the minimum or maximum of a sequence of values.
fn minmax(
    span: Span,
    values: Vec<Spanned<Value>>,
    goal: Ordering,
) -> SourceResult<Value> {
    let mut iter = values.into_iter();
    let Some(Spanned { v: mut extremum, ..}) = iter.next() else {
        bail!(span, "expected at least one value");
    };

    for Spanned { v, span } in iter {
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

/// Determine whether an integer is even.
///
/// ## Example
/// ```example
/// #calc.even(4) \
/// #calc.even(5) \
/// #range(10).filter(calc.even)
/// ```
///
/// Display: Even
/// Category: calculate
/// Returns: boolean
#[func]
pub fn even(
    /// The number to check for evenness.
    value: i64,
) -> Value {
    Value::Bool(value % 2 == 0)
}

/// Determine whether an integer is odd.
///
/// ## Example
/// ```example
/// #calc.odd(4) \
/// #calc.odd(5) \
/// #range(10).filter(calc.odd)
/// ```
///
/// Display: Odd
/// Category: calculate
/// Returns: boolean
#[func]
pub fn odd(
    /// The number to check for oddness.
    value: i64,
) -> Value {
    Value::Bool(value % 2 != 0)
}

/// Calculate the modulus of two numbers.
///
/// ## Example
/// ```example
/// #calc.mod(20, 6) \
/// #calc.mod(1.75, 0.5)
/// ```
///
/// Display: Modulus
/// Category: calculate
/// Returns: integer or float
#[func]
pub fn mod_(
    /// The dividend of the modulus.
    dividend: Num,
    /// The divisor of the modulus.
    divisor: Spanned<Num>,
) -> Value {
    if divisor.v.float() == 0.0 {
        bail!(divisor.span, "divisor must not be zero");
    }
    dividend.apply2(divisor.v, Rem::rem, Rem::rem)
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

cast_from_value! {
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

cast_from_value! {
    AngleLike,
    v: i64 => Self::Int(v),
    v: f64 => Self::Float(v),
    v: Angle => Self::Angle(v),
}
