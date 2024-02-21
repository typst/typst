//! Calculations and processing of numeric values.

use std::cmp;
use std::cmp::Ordering;
use std::ops::{Div, Rem};

use crate::diag::{bail, At, SourceResult, StrResult};
use crate::eval::ops;
use crate::foundations::{cast, func, IntoValue, Module, Scope, Value};
use crate::layout::{Angle, Fr, Length, Ratio};
use crate::syntax::{Span, Spanned};

/// A module with calculation definitions.
pub fn module() -> Module {
    let mut scope = Scope::new();
    scope.define_func::<abs>();
    scope.define_func::<pow>();
    scope.define_func::<exp>();
    scope.define_func::<sqrt>();
    scope.define_func::<root>();
    scope.define_func::<sin>();
    scope.define_func::<cos>();
    scope.define_func::<tan>();
    scope.define_func::<asin>();
    scope.define_func::<acos>();
    scope.define_func::<atan>();
    scope.define_func::<atan2>();
    scope.define_func::<sinh>();
    scope.define_func::<cosh>();
    scope.define_func::<tanh>();
    scope.define_func::<log>();
    scope.define_func::<ln>();
    scope.define_func::<fact>();
    scope.define_func::<perm>();
    scope.define_func::<binom>();
    scope.define_func::<gcd>();
    scope.define_func::<lcm>();
    scope.define_func::<floor>();
    scope.define_func::<ceil>();
    scope.define_func::<trunc>();
    scope.define_func::<fract>();
    scope.define_func::<round>();
    scope.define_func::<clamp>();
    scope.define_func::<min>();
    scope.define_func::<max>();
    scope.define_func::<even>();
    scope.define_func::<odd>();
    scope.define_func::<rem>();
    scope.define_func::<div_euclid>();
    scope.define_func::<rem_euclid>();
    scope.define_func::<quo>();
    scope.define("inf", f64::INFINITY);
    scope.define("nan", f64::NAN);
    scope.define("pi", std::f64::consts::PI);
    scope.define("tau", std::f64::consts::TAU);
    scope.define("e", std::f64::consts::E);
    Module::new("calc", scope)
}

/// Calculates the absolute value of a numeric value.
///
/// ```example
/// #calc.abs(-5) \
/// #calc.abs(5pt - 2cm) \
/// #calc.abs(2fr)
/// ```
#[func(title = "Absolute")]
pub fn abs(
    /// The value whose absolute value to calculate.
    value: ToAbs,
) -> Value {
    value.0
}

/// A value of which the absolute value can be taken.
pub struct ToAbs(Value);

cast! {
    ToAbs,
    v: i64 => Self(v.abs().into_value()),
    v: f64 => Self(v.abs().into_value()),
    v: Length => Self(Value::Length(v.try_abs()
        .ok_or("cannot take absolute value of this length")?)),
    v: Angle => Self(Value::Angle(v.abs())),
    v: Ratio => Self(Value::Ratio(v.abs())),
    v: Fr => Self(Value::Fraction(v.abs())),
}

/// Raises a value to some exponent.
///
/// ```example
/// #calc.pow(2, 3)
/// ```
#[func(title = "Power")]
pub fn pow(
    /// The callsite span.
    span: Span,
    /// The base of the power.
    base: Num,
    /// The exponent of the power.
    exponent: Spanned<Num>,
) -> SourceResult<Num> {
    match exponent.v {
        _ if exponent.v.float() == 0.0 && base.float() == 0.0 => {
            bail!(span, "zero to the power of zero is undefined")
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
        (Num::Int(a), Num::Int(b)) if b >= 0 => {
            a.checked_pow(b as u32).map(Num::Int).ok_or_else(too_large).at(span)?
        }
        (a, b) => Num::Float(if a.float() == std::f64::consts::E {
            b.float().exp()
        } else if a.float() == 2.0 {
            b.float().exp2()
        } else if let Num::Int(b) = b {
            a.float().powi(b as i32)
        } else {
            a.float().powf(b.float())
        }),
    };

    if result.float().is_nan() {
        bail!(span, "the result is not a real number")
    }

    Ok(result)
}

/// Raises a value to some exponent of e.
///
/// ```example
/// #calc.exp(1)
/// ```
#[func(title = "Exponential")]
pub fn exp(
    /// The callsite span.
    span: Span,
    /// The exponent of the power.
    exponent: Spanned<Num>,
) -> SourceResult<f64> {
    match exponent.v {
        Num::Int(i) if i32::try_from(i).is_err() => {
            bail!(exponent.span, "exponent is too large")
        }
        Num::Float(f) if !f.is_normal() && f != 0.0 => {
            bail!(exponent.span, "exponent may not be infinite, subnormal, or NaN")
        }
        _ => {}
    };

    let result = exponent.v.float().exp();
    if result.is_nan() {
        bail!(span, "the result is not a real number")
    }

    Ok(result)
}

/// Calculates the square root of a number.
///
/// ```example
/// #calc.sqrt(16) \
/// #calc.sqrt(2.5)
/// ```
#[func(title = "Square Root")]
pub fn sqrt(
    /// The number whose square root to calculate. Must be non-negative.
    value: Spanned<Num>,
) -> SourceResult<f64> {
    if value.v.float() < 0.0 {
        bail!(value.span, "cannot take square root of negative number");
    }
    Ok(value.v.float().sqrt())
}

/// Calculates the real nth root of a number.
///
/// If the number is negative, then n must be odd.
///
/// ```example
/// #calc.root(16.0, 4) \
/// #calc.root(27.0, 3)
/// ```
#[func]
pub fn root(
    /// The expression to take the root of
    radicand: f64,
    /// Which root of the radicand to take
    index: Spanned<i64>,
) -> SourceResult<f64> {
    if index.v == 0 {
        bail!(index.span, "cannot take the 0th root of a number");
    } else if radicand < 0.0 {
        if index.v % 2 == 0 {
            bail!(
                index.span,
                "negative numbers do not have a real nth root when n is even"
            );
        } else {
            Ok(-(-radicand).powf(1.0 / index.v as f64))
        }
    } else {
        Ok(radicand.powf(1.0 / index.v as f64))
    }
}

/// Calculates the sine of an angle.
///
/// When called with an integer or a float, they will be interpreted as
/// radians.
///
/// ```example
/// #assert(calc.sin(90deg) == calc.sin(-270deg))
/// #calc.sin(1.5) \
/// #calc.sin(90deg)
/// ```
#[func(title = "Sine")]
pub fn sin(
    /// The angle whose sine to calculate.
    angle: AngleLike,
) -> f64 {
    match angle {
        AngleLike::Angle(a) => a.sin(),
        AngleLike::Int(n) => (n as f64).sin(),
        AngleLike::Float(n) => n.sin(),
    }
}

/// Calculates the cosine of an angle.
///
/// When called with an integer or a float, they will be interpreted as
/// radians.
///
/// ```example
/// #calc.cos(90deg) \
/// #calc.cos(1.5) \
/// #calc.cos(90deg)
/// ```
#[func(title = "Cosine")]
pub fn cos(
    /// The angle whose cosine to calculate.
    angle: AngleLike,
) -> f64 {
    match angle {
        AngleLike::Angle(a) => a.cos(),
        AngleLike::Int(n) => (n as f64).cos(),
        AngleLike::Float(n) => n.cos(),
    }
}

/// Calculates the tangent of an angle.
///
/// When called with an integer or a float, they will be interpreted as
/// radians.
///
/// ```example
/// #calc.tan(1.5) \
/// #calc.tan(90deg)
/// ```
#[func(title = "Tangent")]
pub fn tan(
    /// The angle whose tangent to calculate.
    angle: AngleLike,
) -> f64 {
    match angle {
        AngleLike::Angle(a) => a.tan(),
        AngleLike::Int(n) => (n as f64).tan(),
        AngleLike::Float(n) => n.tan(),
    }
}

/// Calculates the arcsine of a number.
///
/// ```example
/// #calc.asin(0) \
/// #calc.asin(1)
/// ```
#[func(title = "Arcsine")]
pub fn asin(
    /// The number whose arcsine to calculate. Must be between -1 and 1.
    value: Spanned<Num>,
) -> SourceResult<Angle> {
    let val = value.v.float();
    if val < -1.0 || val > 1.0 {
        bail!(value.span, "value must be between -1 and 1");
    }
    Ok(Angle::rad(val.asin()))
}

/// Calculates the arccosine of a number.
///
/// ```example
/// #calc.acos(0) \
/// #calc.acos(1)
/// ```
#[func(title = "Arccosine")]
pub fn acos(
    /// The number whose arcsine to calculate. Must be between -1 and 1.
    value: Spanned<Num>,
) -> SourceResult<Angle> {
    let val = value.v.float();
    if val < -1.0 || val > 1.0 {
        bail!(value.span, "value must be between -1 and 1");
    }
    Ok(Angle::rad(val.acos()))
}

/// Calculates the arctangent of a number.
///
/// ```example
/// #calc.atan(0) \
/// #calc.atan(1)
/// ```
#[func(title = "Arctangent")]
pub fn atan(
    /// The number whose arctangent to calculate.
    value: Num,
) -> Angle {
    Angle::rad(value.float().atan())
}

/// Calculates the four-quadrant arctangent of a coordinate.
///
/// The arguments are `(x, y)`, not `(y, x)`.
///
/// ```example
/// #calc.atan2(1, 1) \
/// #calc.atan2(-2, -3)
/// ```
#[func(title = "Four-quadrant Arctangent")]
pub fn atan2(
    /// The X coordinate.
    x: Num,
    /// The Y coordinate.
    y: Num,
) -> Angle {
    Angle::rad(f64::atan2(y.float(), x.float()))
}

/// Calculates the hyperbolic sine of a hyperbolic angle.
///
/// ```example
/// #calc.sinh(0) \
/// #calc.sinh(1.5)
/// ```
#[func(title = "Hyperbolic Sine")]
pub fn sinh(
    /// The hyperbolic angle whose hyperbolic sine to calculate.
    value: f64,
) -> f64 {
    value.sinh()
}

/// Calculates the hyperbolic cosine of a hyperbolic angle.
///
/// ```example
/// #calc.cosh(0) \
/// #calc.cosh(1.5)
/// ```
#[func(title = "Hyperbolic Cosine")]
pub fn cosh(
    /// The hyperbolic angle whose hyperbolic cosine to calculate.
    value: f64,
) -> f64 {
    value.cosh()
}

/// Calculates the hyperbolic tangent of an hyperbolic angle.
///
/// ```example
/// #calc.tanh(0) \
/// #calc.tanh(1.5)
/// ```
#[func(title = "Hyperbolic Tangent")]
pub fn tanh(
    /// The hyperbolic angle whose hyperbolic tangent to calculate.
    value: f64,
) -> f64 {
    value.tanh()
}

/// Calculates the logarithm of a number.
///
/// If the base is not specified, the logarithm is calculated in base 10.
///
/// ```example
/// #calc.log(100)
/// ```
#[func(title = "Logarithm")]
pub fn log(
    /// The callsite span.
    span: Span,
    /// The number whose logarithm to calculate. Must be strictly positive.
    value: Spanned<Num>,
    /// The base of the logarithm. May not be zero.
    #[named]
    #[default(Spanned::new(10.0, Span::detached()))]
    base: Spanned<f64>,
) -> SourceResult<f64> {
    let number = value.v.float();
    if number <= 0.0 {
        bail!(value.span, "value must be strictly positive")
    }

    if !base.v.is_normal() {
        bail!(base.span, "base may not be zero, NaN, infinite, or subnormal")
    }

    let result = if base.v == std::f64::consts::E {
        number.ln()
    } else if base.v == 2.0 {
        number.log2()
    } else if base.v == 10.0 {
        number.log10()
    } else {
        number.log(base.v)
    };

    if result.is_infinite() || result.is_nan() {
        bail!(span, "the result is not a real number")
    }

    Ok(result)
}

/// Calculates the natural logarithm of a number.
///
/// ```example
/// #calc.ln(calc.e)
/// ```
#[func(title = "Natural Logarithm")]
pub fn ln(
    /// The callsite span.
    span: Span,
    /// The number whose logarithm to calculate. Must be strictly positive.
    value: Spanned<Num>,
) -> SourceResult<f64> {
    let number = value.v.float();
    if number <= 0.0 {
        bail!(value.span, "value must be strictly positive")
    }

    let result = number.ln();
    if result.is_infinite() {
        bail!(span, "result close to -inf")
    }

    Ok(result)
}

/// Calculates the factorial of a number.
///
/// ```example
/// #calc.fact(5)
/// ```
#[func(title = "Factorial")]
pub fn fact(
    /// The number whose factorial to calculate. Must be non-negative.
    number: u64,
) -> StrResult<i64> {
    Ok(fact_impl(1, number).ok_or_else(too_large)?)
}

/// Calculates a permutation.
///
/// Returns the `k`-permutation of `n`, or the number of ways to choose `k`
/// items from a set of `n` with regard to order.
///
/// ```example
/// $ "perm"(n, k) &= n!/((n - k)!) \
///   "perm"(5, 3) &= #calc.perm(5, 3) $
/// ```
#[func(title = "Permutation")]
pub fn perm(
    /// The base number. Must be non-negative.
    base: u64,
    /// The number of permutations. Must be non-negative.
    numbers: u64,
) -> StrResult<i64> {
    // By convention.
    if base < numbers {
        return Ok(0);
    }

    Ok(fact_impl(base - numbers + 1, base).ok_or_else(too_large)?)
}

/// Calculates the product of a range of numbers. Used to calculate
/// permutations. Returns None if the result is larger than `i64::MAX`
fn fact_impl(start: u64, end: u64) -> Option<i64> {
    // By convention
    if end + 1 < start {
        return Some(0);
    }

    let real_start: u64 = cmp::max(1, start);
    let mut count: u64 = 1;
    for i in real_start..=end {
        count = count.checked_mul(i)?;
    }

    count.try_into().ok()
}

/// Calculates a binomial coefficient.
///
/// Returns the `k`-combination of `n`, or the number of ways to choose `k`
/// items from a set of `n` without regard to order.
///
/// ```example
/// #calc.binom(10, 5)
/// ```
#[func(title = "Binomial")]
pub fn binom(
    /// The upper coefficient. Must be non-negative.
    n: u64,
    /// The lower coefficient. Must be non-negative.
    k: u64,
) -> StrResult<i64> {
    Ok(binom_impl(n, k).ok_or_else(too_large)?)
}

/// Calculates a binomial coefficient, with `n` the upper coefficient and `k`
/// the lower coefficient. Returns `None` if the result is larger than
/// `i64::MAX`
fn binom_impl(n: u64, k: u64) -> Option<i64> {
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

    result.try_into().ok()
}

/// Calculates the greatest common divisor of two integers.
///
/// ```example
/// #calc.gcd(7, 42)
/// ```
#[func(title = "Greatest Common Divisor")]
pub fn gcd(
    /// The first integer.
    a: i64,
    /// The second integer.
    b: i64,
) -> i64 {
    let (mut a, mut b) = (a, b);
    while b != 0 {
        let temp = b;
        b = a % b;
        a = temp;
    }

    a.abs()
}

/// Calculates the least common multiple of two integers.
///
/// ```example
/// #calc.lcm(96, 13)
/// ```
#[func(title = "Least Common Multiple")]
pub fn lcm(
    /// The first integer.
    a: i64,
    /// The second integer.
    b: i64,
) -> StrResult<i64> {
    if a == b {
        return Ok(a.abs());
    }

    Ok(a.checked_div(gcd(a, b))
        .and_then(|gcd| gcd.checked_mul(b))
        .map(|v| v.abs())
        .ok_or_else(too_large)?)
}

/// Rounds a number down to the nearest integer.
///
/// If the number is already an integer, it is returned unchanged.
///
/// ```example
/// #assert(calc.floor(3.14) == 3)
/// #assert(calc.floor(3) == 3)
/// #calc.floor(500.1)
/// ```
#[func]
pub fn floor(
    /// The number to round down.
    value: Num,
) -> i64 {
    match value {
        Num::Int(n) => n,
        Num::Float(n) => n.floor() as i64,
    }
}

/// Rounds a number up to the nearest integer.
///
/// If the number is already an integer, it is returned unchanged.
///
/// ```example
/// #assert(calc.ceil(3.14) == 4)
/// #assert(calc.ceil(3) == 3)
/// #calc.ceil(500.1)
/// ```
#[func]
pub fn ceil(
    /// The number to round up.
    value: Num,
) -> i64 {
    match value {
        Num::Int(n) => n,
        Num::Float(n) => n.ceil() as i64,
    }
}

/// Returns the integer part of a number.
///
/// If the number is already an integer, it is returned unchanged.
///
/// ```example
/// #assert(calc.trunc(3) == 3)
/// #assert(calc.trunc(-3.7) == -3)
/// #calc.trunc(15.9)
/// ```
#[func(title = "Truncate")]
pub fn trunc(
    /// The number to truncate.
    value: Num,
) -> i64 {
    match value {
        Num::Int(n) => n,
        Num::Float(n) => n.trunc() as i64,
    }
}

/// Returns the fractional part of a number.
///
/// If the number is an integer, returns `0`.
///
/// ```example
/// #assert(calc.fract(3) == 0)
/// #calc.fract(-3.1)
/// ```
#[func(title = "Fractional")]
pub fn fract(
    /// The number to truncate.
    value: Num,
) -> Num {
    match value {
        Num::Int(_) => Num::Int(0),
        Num::Float(n) => Num::Float(n.fract()),
    }
}

/// Rounds a number to the nearest integer.
///
/// Optionally, a number of decimal places can be specified.
///
/// ```example
/// #assert(calc.round(3.14) == 3)
/// #assert(calc.round(3.5) == 4)
/// #calc.round(3.1415, digits: 2)
/// ```
#[func]
pub fn round(
    /// The number to round.
    value: Num,
    /// The number of decimal places.
    #[named]
    #[default(0)]
    digits: i64,
) -> Num {
    match value {
        Num::Int(n) if digits == 0 => Num::Int(n),
        _ => {
            let n = value.float();
            let factor = 10.0_f64.powi(digits as i32);
            Num::Float((n * factor).round() / factor)
        }
    }
}

/// Clamps a number between a minimum and maximum value.
///
/// ```example
/// #assert(calc.clamp(5, 0, 10) == 5)
/// #assert(calc.clamp(5, 6, 10) == 6)
/// #calc.clamp(5, 0, 4)
/// ```
#[func]
pub fn clamp(
    /// The number to clamp.
    value: Num,
    /// The inclusive minimum value.
    min: Num,
    /// The inclusive maximum value.
    max: Spanned<Num>,
) -> SourceResult<Num> {
    if max.v.float() < min.float() {
        bail!(max.span, "max must be greater than or equal to min")
    }
    Ok(value.apply3(min, max.v, i64::clamp, f64::clamp))
}

/// Determines the minimum of a sequence of values.
///
/// ```example
/// #calc.min(1, -3, -5, 20, 3, 6) \
/// #calc.min("typst", "in", "beta")
/// ```
#[func(title = "Minimum")]
pub fn min(
    /// The callsite span.
    span: Span,
    /// The sequence of values from which to extract the minimum.
    /// Must not be empty.
    #[variadic]
    values: Vec<Spanned<Value>>,
) -> SourceResult<Value> {
    minmax(span, values, Ordering::Less)
}

/// Determines the maximum of a sequence of values.
///
/// ```example
/// #calc.max(1, -3, -5, 20, 3, 6) \
/// #calc.max("typst", "in", "beta")
/// ```
#[func(title = "Maximum")]
pub fn max(
    /// The callsite span.
    span: Span,
    /// The sequence of values from which to extract the maximum.
    /// Must not be empty.
    #[variadic]
    values: Vec<Spanned<Value>>,
) -> SourceResult<Value> {
    minmax(span, values, Ordering::Greater)
}

/// Find the minimum or maximum of a sequence of values.
fn minmax(
    span: Span,
    values: Vec<Spanned<Value>>,
    goal: Ordering,
) -> SourceResult<Value> {
    let mut iter = values.into_iter();
    let Some(Spanned { v: mut extremum, .. }) = iter.next() else {
        bail!(span, "expected at least one value");
    };

    for Spanned { v, span } in iter {
        let ordering = ops::compare(&v, &extremum).at(span)?;
        if ordering == goal {
            extremum = v;
        }
    }

    Ok(extremum)
}

/// Determines whether an integer is even.
///
/// ```example
/// #calc.even(4) \
/// #calc.even(5) \
/// #range(10).filter(calc.even)
/// ```
#[func]
pub fn even(
    /// The number to check for evenness.
    value: i64,
) -> bool {
    value % 2 == 0
}

/// Determines whether an integer is odd.
///
/// ```example
/// #calc.odd(4) \
/// #calc.odd(5) \
/// #range(10).filter(calc.odd)
/// ```
#[func]
pub fn odd(
    /// The number to check for oddness.
    value: i64,
) -> bool {
    value % 2 != 0
}

/// Calculates the remainder of two numbers.
///
/// The value `calc.rem(x, y)` always has the same sign as `x`, and is smaller
/// in magnitude than `y`.
///
/// ```example
/// #calc.rem(7, 3) \
/// #calc.rem(7, -3) \
/// #calc.rem(-7, 3) \
/// #calc.rem(-7, -3) \
/// #calc.rem(1.75, 0.5)
/// ```
#[func(title = "Remainder")]
pub fn rem(
    /// The dividend of the remainder.
    dividend: Num,
    /// The divisor of the remainder.
    divisor: Spanned<Num>,
) -> SourceResult<Num> {
    if divisor.v.float() == 0.0 {
        bail!(divisor.span, "divisor must not be zero");
    }
    Ok(dividend.apply2(divisor.v, Rem::rem, Rem::rem))
}

/// Performs euclidean division of two numbers.
///
/// The result of this computation is that of a division rounded to the integer
/// `{n}` such that the dividend is greater than or equal to `{n}` times the divisor.
///
/// ```example
/// #calc.div-euclid(7, 3) \
/// #calc.div-euclid(7, -3) \
/// #calc.div-euclid(-7, 3) \
/// #calc.div-euclid(-7, -3) \
/// #calc.div-euclid(1.75, 0.5)
/// ```
#[func(title = "Euclidean Division")]
pub fn div_euclid(
    /// The dividend of the division.
    dividend: Num,
    /// The divisor of the division.
    divisor: Spanned<Num>,
) -> SourceResult<Num> {
    if divisor.v.float() == 0.0 {
        bail!(divisor.span, "divisor must not be zero");
    }
    Ok(dividend.apply2(divisor.v, i64::div_euclid, f64::div_euclid))
}

/// This calculates the least nonnegative remainder of a division.
///
/// Warning: Due to a floating point round-off error, the remainder may equal the absolute
/// value of the divisor if the dividend is much smaller in magnitude than the divisor
/// and the dividend is negative. This only applies for floating point inputs.
///
/// ```example
/// #calc.rem-euclid(7, 3) \
/// #calc.rem-euclid(7, -3) \
/// #calc.rem-euclid(-7, 3) \
/// #calc.rem-euclid(-7, -3) \
/// #calc.rem(1.75, 0.5)
/// ```
#[func(title = "Euclidean Remainder")]
pub fn rem_euclid(
    /// The dividend of the remainder.
    dividend: Num,
    /// The divisor of the remainder.
    divisor: Spanned<Num>,
) -> SourceResult<Num> {
    if divisor.v.float() == 0.0 {
        bail!(divisor.span, "divisor must not be zero");
    }
    Ok(dividend.apply2(divisor.v, i64::rem_euclid, f64::rem_euclid))
}

/// Calculates the quotient (floored division) of two numbers.
///
/// ```example
/// $ "quo"(a, b) &= floor(a/b) \
///   "quo"(14, 5) &= #calc.quo(14, 5) \
///   "quo"(3.46, 0.5) &= #calc.quo(3.46, 0.5) $
/// ```
#[func(title = "Quotient")]
pub fn quo(
    /// The dividend of the quotient.
    dividend: Num,
    /// The divisor of the quotient.
    divisor: Spanned<Num>,
) -> SourceResult<i64> {
    if divisor.v.float() == 0.0 {
        bail!(divisor.span, "divisor must not be zero");
    }

    Ok(floor(dividend.apply2(divisor.v, Div::div, Div::div)))
}

/// A value which can be passed to functions that work with integers and floats.
#[derive(Debug, Copy, Clone)]
pub enum Num {
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
}

cast! {
    Num,
    self => match self {
        Self::Int(v) => v.into_value(),
        Self::Float(v) => v.into_value(),
    },
    v: i64 => Self::Int(v),
    v: f64 => Self::Float(v),
}

/// A value that can be passed to a trigonometric function.
pub enum AngleLike {
    Int(i64),
    Float(f64),
    Angle(Angle),
}

cast! {
    AngleLike,
    v: i64 => Self::Int(v),
    v: f64 => Self::Float(v),
    v: Angle => Self::Angle(v),
}

/// The error message when the result is too large to be represented.
#[cold]
fn too_large() -> &'static str {
    "the result is too large"
}
