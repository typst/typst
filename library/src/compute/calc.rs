//! Calculations and processing of numeric values.

use std::cmp;
use std::cmp::Ordering;
use std::ops::{Div, Rem};

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
    scope.define("atan2", atan2);
    scope.define("sinh", sinh);
    scope.define("cosh", cosh);
    scope.define("tanh", tanh);
    scope.define("log", log);
    scope.define("fact", fact);
    scope.define("perm", perm);
    scope.define("binom", binom);
    scope.define("gcd", gcd);
    scope.define("lcm", lcm);
    scope.define("floor", floor);
    scope.define("ceil", ceil);
    scope.define("trunc", trunc);
    scope.define("fract", fract);
    scope.define("round", round);
    scope.define("clamp", clamp);
    scope.define("min", min);
    scope.define("max", max);
    scope.define("even", even);
    scope.define("odd", odd);
    scope.define("rem", rem);
    scope.define("quo", quo);
    scope.define("euclid_div", euclid_div);
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
        .ok_or("cannot take absolute value of this length")?)),
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
    match exponent.v {
        _ if exponent.v.float() == 0.0 && base.float() == 0.0 => {
            bail!(args.span, "zero to the power of zero is undefined")
        }
        Num::Int(i) if i32::try_from(i).is_err() => {
            bail!(exponent.span, "exponent is too large")
        }
        Num::Float(f) if !f.is_normal() && f != 0.0 => {
            bail!(exponent.span, "exponent may not be infinite, subnormal, or NaN")
        }
        _ => {}
    };

    let result = match (base, exponent.v) {
        (Num::Int(a), Num::Int(b)) if b >= 0 => a
            .checked_pow(b as u32)
            .map(Num::Int)
            .ok_or("the result is too large")
            .at(args.span)?,
        (a, Num::Int(b)) => Num::Float(a.float().powi(b as i32)),
        (a, b) => Num::Float(a.float().powf(b.float())),
    };

    if result.float().is_nan() {
        bail!(args.span, "the result is not a real number")
    }

    result.value()
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
        bail!(value.span, "value must be between -1 and 1");
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
        bail!(value.span, "value must be between -1 and 1");
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

/// Calculate the four-quadrant arctangent of a coordinate.
///
/// The arguments are `(x, y)`, not `(y, x)`.
///
/// ## Example
/// ```example
/// #calc.atan2(1, 1) \
/// #calc.atan2(-2, -3)
/// ```
///
/// Display: Four-quadrant Arctangent
/// Category: calculate
/// Returns: angle
#[func]
pub fn atan2(
    /// The X coordinate.
    x: Num,
    /// The Y coordinate.
    y: Num,
) -> Value {
    Value::Angle(Angle::rad(f64::atan2(y.float(), x.float())))
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
    /// The number whose logarithm to calculate. Must be strictly positive.
    value: Spanned<Num>,
    /// The base of the logarithm. Defaults to `{10}` and may not be zero.
    #[named]
    #[default(Spanned::new(10.0, Span::detached()))]
    base: Spanned<f64>,
) -> Value {
    let number = value.v.float();
    if number <= 0.0 {
        bail!(value.span, "value must be strictly positive")
    }

    if !base.v.is_normal() {
        bail!(base.span, "base may not be zero, NaN, infinite, or subnormal")
    }

    let result = if base.v == 2.0 {
        number.log2()
    } else if base.v == 10.0 {
        number.log10()
    } else {
        number.log(base.v)
    };

    if result.is_infinite() || result.is_nan() {
        bail!(args.span, "the result is not a real number")
    }

    Value::Float(result)
}

/// Calculate the factorial of a number.
///
/// ## Example
/// ```example
/// #calc.fact(5)
/// ```
///
/// Display: Factorial
/// Category: calculate
/// Returns: integer
#[func]
pub fn fact(
    /// The number whose factorial to calculate. Must be non-negative.
    number: u64,
) -> Value {
    factorial_range(1, number)
        .map(Value::Int)
        .ok_or("the result is too large")
        .at(args.span)?
}

/// Calculates the product of a range of numbers. Used to calculate
/// permutations. Returns None if the result is larger than `i64::MAX`
fn factorial_range(start: u64, end: u64) -> Option<i64> {
    // By convention
    if end + 1 < start {
        return Some(0);
    }

    let real_start: u64 = cmp::max(1, start);
    let mut count: u64 = 1;
    for i in real_start..=end {
        count = count.checked_mul(i)?;
    }

    i64::try_from(count).ok()
}

/// Calculate a permutation.
///
/// ## Example
/// ```example
/// #calc.perm(10, 5)
/// ```
///
/// Display: Permutation
/// Category: calculate
/// Returns: integer
#[func]
pub fn perm(
    /// The base number. Must be non-negative.
    base: u64,
    /// The number of permutations. Must be non-negative.
    numbers: u64,
) -> Value {
    // By convention.
    if base < numbers {
        return Ok(Value::Int(0));
    }

    factorial_range(base - numbers + 1, base)
        .map(Value::Int)
        .ok_or("the result is too large")
        .at(args.span)?
}

/// Calculate a binomial coefficient.
///
/// ## Example
/// ```example
/// #calc.binom(10, 5)
/// ```
///
/// Display: Binomial
/// Category: calculate
/// Returns: integer
#[func]
pub fn binom(
    /// The upper coefficient. Must be non-negative.
    n: u64,
    /// The lower coefficient. Must be non-negative.
    k: u64,
) -> Value {
    binomial(n, k)
        .map(Value::Int)
        .ok_or("the result is too large")
        .at(args.span)?
}

/// Calculates a binomial coefficient, with `n` the upper coefficient and `k`
/// the lower coefficient. Returns `None` if the result is larger than
/// `i64::MAX`
fn binomial(n: u64, k: u64) -> Option<i64> {
    if k > n {
        return Some(0);
    }

    // By symmetry
    let real_k = cmp::min(n - k, k);
    if real_k == 0 {
        return Some(1);
    }

    let mut result: u64 = 1;
    for i in 0..real_k {
        result = result.checked_mul(n - i)?.checked_div(i + 1)?;
    }

    i64::try_from(result).ok()
}

/// Calculate the greatest common divisor of two integers.
///
/// ## Example
/// ```example
/// #calc.gcd(7, 42)
/// ```
///
/// Display: Greatest Common Divisor
/// Category: calculate
/// Returns: integer
#[func]
pub fn gcd(
    /// The first integer.
    a: i64,
    /// The second integer.
    b: i64,
) -> Value {
    Value::Int(calculate_gcd(a, b))
}

/// Calculates the greatest common divisor of two integers
/// It is always non-negative.
fn calculate_gcd(mut a: i64, mut b: i64) -> i64 {
    while b != 0 {
        let temp = b;
        b = a % b;
        a = temp;
    }

    a.abs()
}

/// Calculate the least common multiple of two integers.
///
/// ## Example
/// ```example
/// #calc.lcm(96, 13)
/// ```
///
/// Display: Least Common Multiple
/// Category: calculate
/// Returns: integer
#[func]
pub fn lcm(
    /// The first integer.
    a: i64,
    /// The second integer.
    b: i64,
) -> Value {
    calculate_lcm(a, b)
        .map(Value::Int)
        .ok_or("the return value is too large")
        .at(args.span)?
}

/// Calculates the least common multiple between two non-zero integers
/// Returns None if the value cannot be computed.
/// It is always non-negative.
fn calculate_lcm(a: i64, b: i64) -> Option<i64> {
    if a == b {
        return Some(a.abs());
    }

    a.checked_div(calculate_gcd(a, b))?.checked_mul(b).map(|v| v.abs())
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

/// Returns the integer part of a number.
///
/// If the number is already an integer, it is returned unchanged.
///
/// ## Example
/// ```example
/// #assert(calc.trunc(3) == 3)
/// #assert(calc.trunc(-3.7) == -3)
/// #assert(calc.trunc(15.9) == 15)
/// ```
///
/// Display: Truncate
/// Category: calculate
/// Returns: integer
#[func]
pub fn trunc(
    /// The number to truncate.
    value: Num,
) -> Value {
    Value::Int(match value {
        Num::Int(n) => n,
        Num::Float(n) => n.trunc() as i64,
    })
}

/// Returns the fractional part of a number.
///
/// If the number is an integer, it returns `0`.
///
/// ## Example
/// ```example
/// #assert(calc.fract(3) == 0)
/// #calc.fract(-3.1)
/// ```
///
/// Display: Fractional
/// Category: calculate
/// Returns: integer or float
#[func]
pub fn fract(
    /// The number to truncate.
    value: Num,
) -> Value {
    match value {
        Num::Int(_) => Value::Int(0),
        Num::Float(n) => Value::Float(n.fract()),
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
    value.apply3(min, max.v, i64::clamp, f64::clamp).value()
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

/// Calculate the remainder of two numbers.
///
/// ## Example
/// ```example
/// #calc.rem(20, 6) \
/// #calc.rem(1.75, 0.5)
/// ```
///
/// Display: Remainder
/// Category: calculate
/// Returns: integer or float
#[func]
pub fn rem(
    /// The dividend of the remainder.
    dividend: Num,
    /// The divisor of the remainder.
    divisor: Spanned<Num>,
) -> Value {
    if divisor.v.float() == 0.0 {
        bail!(divisor.span, "divisor must not be zero");
    }
    dividend.apply2(divisor.v, Rem::rem, Rem::rem).value()
}

/// Calculate the quotient of two numbers.
///
/// ## Example
/// ```example
/// #calc.quo(14, 5) \
/// #calc.quo(3.46, 0.5)
/// ```
///
/// Display: Quotient
/// Category: calculate
/// Returns: integer or float
#[func]
pub fn quo(
    /// The dividend of the quotient.
    dividend: Num,
    /// The divisor of the quotient.
    divisor: Spanned<Num>,
) -> Value {
    if divisor.v.float() == 0.0 {
        bail!(divisor.span, "divisor must not be zero");
    }

    Value::Int(match dividend.apply2(divisor.v, Div::div, Div::div) {
        Num::Int(i) => i,
        Num::Float(f) => f.floor() as i64, // Note: the result should be an integer but floats doesn't have the same precision as i64.
    })
}

/// Calculate the euclidean division of two numbers.
///
/// ## Example
/// ```example
/// #assert(calc.euclid_div(14, 5) == ("rem": 4, "quo": 2)) \
/// #calc.euclid_div(3.46, 0.5)
/// ```
///
/// Display: Euclidian Divison
/// Category: calculate
/// Returns: Dictionnary with `rem` and `quo` keys. The values are integers or floats, depending on the input.
#[func]
pub fn euclid_div(
    /// The dividend.
    dividend: Num,
    /// The divisor.
    divisor: Spanned<Num>,
) -> Value {
    if divisor.v.float() == 0.0 {
        bail!(divisor.span, "divisor must not be zero");
    }

    let rem = dividend.apply2(divisor.v, Rem::rem, Rem::rem);
    let quo = Num::Int(match dividend.apply2(divisor.v, Div::div, Div::div) {
        Num::Int(i) => i,
        Num::Float(f) => f.floor() as i64, // Note: the result should be an integer but floats doesn't have the same precision as i64.
    });

    Value::Dict(dict!["rem" => rem.value(), "quo" => quo.value()])
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
    ) -> Num {
        match (self, other) {
            (Self::Int(a), Self::Int(b)) => Num::Int(int(a, b)),
            (a, b) => Num::Float(float(a.float(), b.float())),
        }
    }

    fn apply3(
        self,
        other: Self,
        third: Self,
        int: impl FnOnce(i64, i64, i64) -> i64,
        float: impl FnOnce(f64, f64, f64) -> f64,
    ) -> Num {
        match (self, other, third) {
            (Self::Int(a), Self::Int(b), Self::Int(c)) => Num::Int(int(a, b, c)),
            (a, b, c) => Num::Float(float(a.float(), b.float(), c.float())),
        }
    }

    fn float(self) -> f64 {
        match self {
            Self::Int(v) => v as f64,
            Self::Float(v) => v,
        }
    }

    fn value(self) -> Value {
        match self {
            Self::Int(v) => Value::Int(v),
            Self::Float(v) => Value::Float(v),
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
