use std::cmp::{self, Ordering};
use std::num::{NonZeroI64, NonZeroIsize, NonZeroU64, NonZeroUsize, ParseIntError};
use std::ops::{Div, Rem};

use ecow::{eco_format, EcoString};

use crate::diag::{bail, At, SourceResult, StrResult};
use crate::foundations::{cast, func, repr, scope, ty, Repr, Str, Value};
use crate::layout::Angle;
use crate::syntax::{Span, Spanned};

use super::calc::{minmax, Num};
use super::float::f64Ext;

/// A whole number.
///
/// The number can be negative, zero, or positive. As Typst uses 64 bits to
/// store integers, integers cannot be smaller than `{-9223372036854775808}` or
/// larger than `{9223372036854775807}`.
///
/// The number can also be specified as hexadecimal, octal, or binary by
/// starting it with a zero followed by either `x`, `o`, or `b`.
///
/// You can convert a value to an integer with this type's constructor.
///
/// # Example
/// ```example
/// #(1 + 2) \
/// #(2 - 5) \
/// #(3 + 4 < 8)
///
/// #0xff \
/// #0o10 \
/// #0b1001
/// ```
#[ty(scope, cast, name = "int", title = "Integer")]
type i64;

#[scope]
impl i64 {
    /// Converts a value to an integer.
    ///
    /// - Booleans are converted to `0` or `1`.
    /// - Floats are floored to the next 64-bit integer.
    /// - Strings are parsed in base 10.
    ///
    /// ```example
    /// #int(false) \
    /// #int(true) \
    /// #int(2.7) \
    /// #(int("27") + int("4"))
    /// ```
    #[func(constructor)]
    pub fn construct(
        /// The value that should be converted to an integer.
        value: ToInt,
    ) -> i64 {
        value.0
    }

    /// Calculates the sign of an integer.
    ///
    /// - If the number is positive, returns `{1}`.
    /// - If the number is negative, returns `{-1}`.
    /// - If the number is zero, returns `{0}`.
    ///
    /// ```example
    /// #(5).signum() \
    /// #(-5).signum() \
    /// #(0).signum() \
    /// ```
    #[func]
    pub fn signum(self) -> i64 {
        i64::signum(self)
    }

    /// Calculates the absolute value of an integer.
    ///
    /// ```example
    /// #(-5).abs() \
    /// #10.abs()
    /// ```
    #[func(title = "Absolute")]
    pub fn abs(self) -> i64 {
        i64::abs(self)
    }

    /// Raises an integer to some exponent.
    ///
    /// ```example
    /// #2.pow(3)
    /// #(-5).pow(3)
    /// ```
    #[func(title = "Power")]
    pub fn pow_(
        self,
        /// The callsite span.
        span: Span,
        /// The exponent of the power.
        exponent: Spanned<Num>,
    ) -> SourceResult<Num> {
        let exponent = match exponent.v {
            Num::Int(i) if i >= 0 => {
                let Ok(i) = i32::try_from(i) else {
                    bail!(exponent.span, "exponent is too large")
                };
                i
            }
            _ => {
                return f64Ext::pow(self as f64, span, exponent).map(Num::Float);
            }
        };

        self.checked_pow(exponent as u32)
            .map(Num::Int)
            .ok_or_else(too_large)
            .at(span)
    }

    /// Raises an integer to some exponent of e.
    ///
    /// ```example
    /// #1.exp()
    /// ```
    #[func(title = "Exponential")]
    pub fn exp(
        self,
        /// The callsite span.
        span: Span,
    ) -> SourceResult<f64> {
        if i32::try_from(self).is_err() {
            bail!(span, "exponent is too large")
        }
        f64Ext::exp_(self as f64, span)
    }

    /// Calculates the square root of an integer. Must not be negative.
    ///
    /// ```example
    /// #25.sqrt()
    /// ```
    #[func(title = "Square Root")]
    pub fn sqrt(self) -> StrResult<f64> {
        f64Ext::sqrt_(self as f64)
    }

    /// Calculates the real nth root of an integer.
    ///
    /// If the number is negative, then n must be odd.
    ///
    /// ```example
    /// #16.root(4) \
    /// #27.root(3)
    /// ```
    #[func]
    pub fn root(
        self,
        /// Which root of the radicand to take
        index: Spanned<i64>,
    ) -> SourceResult<f64> {
        f64Ext::root(self as f64, index)
    }

    /// Calculates the sine of an angle, interpreted as radians.
    ///
    /// ```example
    /// #1.sin()
    /// ```
    #[func(title = "Sine")]
    pub fn sin(self) -> f64 {
        (self as f64).sin()
    }

    /// Calculates the cosine of an angle, interpreted as radians.
    ///
    /// ```example
    /// #1.cos()
    /// ```
    #[func(title = "Cosine")]
    pub fn cos(self) -> f64 {
        (self as f64).cos()
    }

    /// Calculates the tangent of an angle, interpreted as radians.
    ///
    /// ```example
    /// #1.tan()
    /// ```
    #[func(title = "Tangent")]
    pub fn tan(self) -> f64 {
        (self as f64).tan()
    }

    /// Calculates the arcsine of an integer, which must be between -1 and 1.
    ///
    /// ```example
    /// #0.asin() \
    /// #1.asin()
    /// ```
    #[func(title = "Arcsine")]
    pub fn asin(self) -> StrResult<Angle> {
        f64Ext::asin(self as f64)
    }

    /// Calculates the arccosine of an integer, which must be between -1 and 1.
    ///
    /// ```example
    /// #0.acos() \
    /// #1.acos()
    /// ```
    #[func(title = "Arccosine")]
    pub fn acos(self) -> StrResult<Angle> {
        f64Ext::acos(self as f64)
    }

    /// Calculates the arctangent of an integer.
    ///
    /// ```example
    /// #0.atan() \
    /// #1.atan()
    /// ```
    #[func(title = "Arctangent")]
    pub fn atan(self) -> Angle {
        f64Ext::atan(self as f64)
    }

    /// Calculates the four-quadrant arctangent of a coordinate.
    ///
    /// The arguments are `(x, y)`, not `(y, x)`.
    ///
    /// ```example
    /// #int.atan2(1, 1) \
    /// #int.atan2(-2, -3)
    /// ```
    #[func(title = "Four-quadrant Arctangent")]
    pub fn atan2(
        self,
        /// The Y coordinate.
        y: Num,
    ) -> Angle {
        f64Ext::atan2_(self as f64, y)
    }

    /// Calculates the hyperbolic sine of a hyperbolic angle.
    ///
    /// ```example
    /// #1.sinh()
    /// ```
    #[func(title = "Hyperbolic Sine")]
    pub fn sinh(self) -> f64 {
        f64Ext::sinh(self as f64)
    }

    /// Calculates the hyperbolic cosine of a hyperbolic angle.
    ///
    /// ```example
    /// #1.cosh()
    /// ```
    #[func(title = "Hyperbolic Cosine")]
    pub fn cosh(self) -> f64 {
        f64Ext::cosh(self as f64)
    }

    /// Calculates the hyperbolic tangent of a hyperbolic angle.
    ///
    /// ```example
    /// #1.tanh()
    /// ```
    #[func(title = "Hyperbolic Tangent")]
    pub fn tanh(self) -> f64 {
        f64Ext::tanh(self as f64)
    }

    /// Calculates the logarithm of an integer, which must be strictly
    /// positive.
    ///
    /// If the base is not specified, the logarithm is calculated in base 10.
    ///
    /// ```example
    /// #100.log()
    /// ```
    #[func(title = "Logarithm")]
    pub fn log(
        self,
        /// The callsite span.
        span: Span,
        /// The base of the logarithm. May not be zero.
        #[named]
        #[default(Spanned::new(10.0, Span::detached()))]
        base: Spanned<f64>,
    ) -> SourceResult<f64> {
        f64Ext::log_(self as f64, span, base)
    }

    /// Calculates the natural logarithm of a number, which must be strictly
    /// positive.
    ///
    /// ```example
    /// #45.ln()
    /// ```
    #[func(title = "Natural Logarithm")]
    pub fn ln_(
        self,
        /// The callsite span.
        span: Span,
    ) -> SourceResult<f64> {
        f64Ext::ln_(self as f64, span)
    }

    /// Calculates the factorial of a number, which must be non-negative.
    ///
    /// ```example
    /// #5.fact()
    /// ```
    #[func(title = "Factorial")]
    pub fn fact(self) -> StrResult<i64> {
        Ok(fact_impl(1, as_u64(self)?).ok_or_else(too_large)?)
    }

    /// Calculates a permutation. Both operands must be non-negative.
    ///
    /// Returns the `k`-permutation of `n`, or the number of ways to choose `k`
    /// items from a set of `n` with regard to order.
    ///
    /// ```example
    /// $ "perm"(n, k) &= n!/((n - k)!) \
    ///   "perm"(5, 3) &= #5.perm(3) $
    /// ```
    #[func(title = "Permutation")]
    pub fn perm(
        self,
        /// The number of permutations. Must be non-negative.
        numbers: u64,
    ) -> StrResult<i64> {
        let base = as_u64(self)?;
        // By convention.
        if base < numbers {
            return Ok(0);
        }

        Ok(fact_impl(base - numbers + 1, base).ok_or_else(too_large)?)
    }

    /// Calculates a binomial coefficient. Both operands must be non-negative.
    ///
    /// Returns the `k`-combination of `n`, or the number of ways to choose `k`
    /// items from a set of `n` without regard to order.
    ///
    /// ```example
    /// #10.binom(5)
    /// #int.binom(10, 5)
    /// ```
    #[func(title = "Binomial")]
    pub fn binom(
        self,
        /// The lower coefficient. Must be non-negative.
        k: u64,
    ) -> StrResult<i64> {
        Ok(binom_impl(as_u64(self)?, k).ok_or_else(too_large)?)
    }

    /// Calculates the greatest common divisor of two integers.
    ///
    /// ```example
    /// #7.gcd(42)
    /// #int.gcd(7, 42)
    /// ```
    #[func(title = "Greatest Common Divisor")]
    pub fn gcd(
        self,
        /// The second integer.
        b: i64,
    ) -> i64 {
        let (mut a, mut b) = (self, b);
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
    /// #96.lcm(13)
    /// ```
    #[func(title = "Least Common Multiple")]
    pub fn lcm(
        self,
        /// The second integer.
        b: i64,
    ) -> StrResult<i64> {
        if self == b {
            return Ok(self.abs());
        }

        Ok(self
            .checked_div(i64Ext::gcd(self, b))
            .and_then(|gcd| gcd.checked_mul(b))
            .map(|v| v.abs())
            .ok_or_else(too_large)?)
    }

    /// Clamps an integer between a minimum and maximum value.
    ///
    /// ```example
    /// #assert.eq(5.clamp(0, 10), 5)
    /// #assert.eq(5.clamp(6, 10), 6)
    /// #5.clamp(0, 4)
    /// ```
    #[func]
    pub fn clamp_(
        self,
        /// The inclusive minimum value.
        min: Num,
        /// The inclusive maximum value.
        max: Spanned<Num>,
    ) -> SourceResult<Num> {
        if max.v.float() < min.float() {
            bail!(max.span, "max must be greater than or equal to min")
        }
        Ok(Num::Int(self).apply3(min, max.v, Ord::clamp, f64::clamp))
    }

    /// Determines the minimum of a sequence of numbers.
    ///
    /// ```example
    /// #int.min(1, -3, -5, 20, 3, 6)
    /// ```
    #[func(title = "Minimum")]
    pub fn min_(
        self,
        /// The callsite span.
        span: Span,
        /// The sequence of numbers from which to extract the minimum.
        #[variadic]
        values: Vec<Spanned<Value>>,
    ) -> SourceResult<Value> {
        let mut values = values;
        values.insert(0, Spanned::new(Value::Int(self), span));
        minmax(span, values, Ordering::Less)
    }

    /// Determines the maximum of a sequence of numbers.
    ///
    /// ```example
    /// #int.max(1, -3, -5, 20, 3, 6)
    /// ```
    #[func(title = "Maximum")]
    pub fn max_(
        self,
        /// The callsite span.
        span: Span,
        /// The sequence of numbers from which to extract the maximum.
        #[variadic]
        values: Vec<Spanned<Value>>,
    ) -> SourceResult<Value> {
        let mut values = values;
        values.insert(0, Spanned::new(Value::Int(self), span));
        minmax(span, values, Ordering::Greater)
    }

    /// Determines whether an integer is even.
    ///
    /// ```example
    /// #4.even() \
    /// #5.even() \
    /// #range(10).filter(int.even)
    /// ```
    #[func]
    pub fn even(self) -> bool {
        self % 2 == 0
    }

    /// Determines whether an integer is odd.
    ///
    /// ```example
    /// #4.odd() \
    /// #5.odd() \
    /// #range(10).filter(int.odd)
    /// ```
    #[func]
    pub fn odd(self) -> bool {
        self % 2 != 0
    }

    /// Calculates the remainder of two numbers.
    ///
    /// The value `x.rem(y)` always has the same sign as `x`, and is smaller
    /// in magnitude than `y`.
    ///
    /// ```example
    /// #7.rem(3) \
    /// #7.rem(-3) \
    /// #(-7).rem(3) \
    /// #(-7).rem(-3) \
    /// #3.rem(0.4)
    /// ```
    #[func(title = "Remainder")]
    pub fn rem_(
        self,
        /// The divisor of the remainder.
        divisor: Spanned<Num>,
    ) -> SourceResult<Num> {
        if divisor.v.float() == 0.0 {
            bail!(divisor.span, "divisor must not be zero");
        }
        Ok(Num::Int(self).apply2(divisor.v, Rem::rem, Rem::rem))
    }

    /// Performs euclidean division of two numbers.
    ///
    /// The result of this computation is that of a division rounded to the integer
    /// `{n}` such that the dividend is greater than or equal to `{n}` times the divisor.
    ///
    /// ```example
    /// #7.div-euclid(3) \
    /// #7.div-euclid(-3) \
    /// #(-7).div-euclid(3) \
    /// #(-7).div-euclid(-3) \
    /// #3.div-euclid(0.4)
    /// ```
    #[func(title = "Euclidean Division")]
    pub fn div_euclid_(
        self,
        /// The divisor of the division.
        divisor: Spanned<Num>,
    ) -> SourceResult<Num> {
        if divisor.v.float() == 0.0 {
            bail!(divisor.span, "divisor must not be zero");
        }
        Ok(Num::Int(self).apply2(divisor.v, i64::div_euclid, f64::div_euclid))
    }

    /// This calculates the least nonnegative remainder of a division.
    ///
    /// Warning: Due to a floating point round-off error, the remainder may equal the absolute
    /// value of the divisor if the dividend is much smaller in magnitude than the divisor
    /// and the dividend is negative. This only applies for floating point inputs.
    ///
    /// ```example
    /// #7.rem-euclid(3) \
    /// #7.rem-euclid(-3) \
    /// #7.rem-euclid(3) \
    /// #7.rem-euclid(-3) \
    /// #3.rem-euclid(0.4)
    /// ```
    #[func(title = "Euclidean Remainder")]
    pub fn rem_euclid_(
        self,
        /// The divisor of the remainder.
        divisor: Spanned<Num>,
    ) -> SourceResult<Num> {
        if divisor.v.float() == 0.0 {
            bail!(divisor.span, "divisor must not be zero");
        }
        Ok(Num::Int(self).apply2(divisor.v, i64::rem_euclid, f64::rem_euclid))
    }

    /// Calculates the quotient (floored division) of two numbers.
    ///
    /// ```example
    /// $ "quo"(a, b) &= floor(a/b) \
    ///   "quo"(14, 5) &= #14.quo(5) \
    ///   "quo"(4, 0.4) &= #4.quo(0.4) $
    /// ```
    #[func(title = "Quotient")]
    pub fn quo(
        self,
        /// The divisor of the quotient.
        divisor: Spanned<Num>,
    ) -> SourceResult<i64> {
        if divisor.v.float() == 0.0 {
            bail!(divisor.span, "divisor must not be zero");
        }

        Ok(Num::Int(self).apply2(divisor.v, Div::div, Div::div).floor())
    }

    /// Calculates the bitwise NOT of an integer.
    ///
    /// For the purposes of this function, the operand is treated as a signed
    /// integer of 64 bits.
    ///
    /// ```example
    /// #4.bit-not()
    /// #(-1).bit-not()
    /// ```
    #[func(title = "Bitwise NOT")]
    pub fn bit_not(self) -> i64 {
        !self
    }

    /// Calculates the bitwise AND between two integers.
    ///
    /// For the purposes of this function, the operands are treated as signed
    /// integers of 64 bits.
    ///
    /// ```example
    /// #128.bit-and(192)
    /// ```
    #[func(title = "Bitwise AND")]
    pub fn bit_and(
        self,
        /// The right-hand operand of the bitwise AND.
        rhs: i64,
    ) -> i64 {
        self & rhs
    }

    /// Calculates the bitwise OR between two integers.
    ///
    /// For the purposes of this function, the operands are treated as signed
    /// integers of 64 bits.
    ///
    /// ```example
    /// #64.bit-or(32)
    /// ```
    #[func(title = "Bitwise OR")]
    pub fn bit_or(
        self,
        /// The right-hand operand of the bitwise OR.
        rhs: i64,
    ) -> i64 {
        self | rhs
    }

    /// Calculates the bitwise XOR between two integers.
    ///
    /// For the purposes of this function, the operands are treated as signed
    /// integers of 64 bits.
    ///
    /// ```example
    /// #64.bit-xor(96)
    /// ```
    #[func(title = "Bitwise XOR")]
    pub fn bit_xor(
        self,
        /// The right-hand operand of the bitwise XOR.
        rhs: i64,
    ) -> i64 {
        self ^ rhs
    }

    /// Shifts the operand's bits to the left by the specified amount.
    ///
    /// For the purposes of this function, the operand is treated as a signed
    /// integer of 64 bits. An error will occur if the result is too large to
    /// fit in a 64-bit integer.
    ///
    /// ```example
    /// #33.bit-lshift(2)
    /// #(-1).bit-lshift(3)
    /// ```
    #[func(title = "Bitwise Left Shift")]
    pub fn bit_lshift(
        self,

        /// The amount of bits to shift. Must not be negative.
        shift: u32,
    ) -> StrResult<i64> {
        Ok(self.checked_shl(shift).ok_or("the result is too large")?)
    }

    /// Shifts the operand's bits to the right by the specified amount.
    /// Performs an arithmetic shift by default (extends the sign bit to the left,
    /// such that negative numbers stay negative), but that can be changed by the
    /// `logical` parameter.
    ///
    /// For the purposes of this function, the operand is treated as a signed
    /// integer of 64 bits.
    ///
    /// ```example
    /// #64.bit-rshift(2)
    /// #(-8).bit-rshift(2)
    /// #(-8).bit-rshift(2, logical: true)
    /// ```
    #[func(title = "Bitwise Right Shift")]
    pub fn bit_rshift(
        self,

        /// The amount of bits to shift. Must not be negative.
        ///
        /// Shifts larger than 63 are allowed and will cause the return value to
        /// saturate. For non-negative numbers, the return value saturates at `0`,
        /// while, for negative numbers, it saturates at `-1` if `logical` is set
        /// to `false`, or `0` if it is `true`. This behavior is consistent with
        /// just applying this operation multiple times. Therefore, the shift will
        /// always succeed.
        shift: u32,

        /// Toggles whether a logical (unsigned) right shift should be performed
        /// instead of arithmetic right shift.
        /// If this is `true`, negative operands will not preserve their sign bit,
        /// and bits which appear to the left after the shift will be `0`.
        /// This parameter has no effect on non-negative operands.
        #[named]
        #[default(false)]
        logical: bool,
    ) -> i64 {
        if logical {
            if shift >= u64::BITS {
                // Excessive logical right shift would be equivalent to setting
                // all bits to zero. Using `.min(63)` is not enough for logical
                // right shift, since `-1 >> 63` returns 1, whereas
                // `(-1).bit-rshift(64)` should return the same as
                // `(-1 >> 63) >> 1`, which is zero.
                0
            } else {
                // Here we reinterpret the signed integer's bits as unsigned to
                // perform logical right shift, and then reinterpret back as signed.
                // This is valid as, according to the Rust reference, casting between
                // two integers of same size (i64 <-> u64) is a no-op (two's complement
                // is used).
                // Reference:
                // https://doc.rust-lang.org/stable/reference/expressions/operator-expr.html#numeric-cast
                ((self as u64) >> shift) as i64
            }
        } else {
            // Saturate at -1 (negative) or 0 (otherwise) on excessive arithmetic
            // right shift. Shifting those numbers any further does not change
            // them, so it is consistent.
            let shift = shift.min(i64::BITS - 1);
            self >> shift
        }
    }
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

impl Repr for i64 {
    fn repr(&self) -> EcoString {
        eco_format!("{:?}", self)
    }
}

/// A value that can be cast to an integer.
pub struct ToInt(i64);

cast! {
    ToInt,
    v: bool => Self(v as i64),
    v: f64 => Self(v as i64),
    v: Str => Self(parse_int(&v).map_err(|_| eco_format!("invalid integer: {}", v))?),
    v: i64 => Self(v),
}

fn parse_int(mut s: &str) -> Result<i64, ParseIntError> {
    let mut sign = 1;
    if let Some(rest) = s.strip_prefix('-').or_else(|| s.strip_prefix(repr::MINUS_SIGN)) {
        sign = -1;
        s = rest;
    }
    if sign == -1 && s == "9223372036854775808" {
        return Ok(i64::MIN);
    }
    Ok(sign * s.parse::<i64>()?)
}

macro_rules! signed_int {
    ($($ty:ty)*) => {
        $(cast! {
            $ty,
            self => Value::Int(self as _),
            v: i64 => v.try_into().map_err(|_| "number too large")?,
        })*
    }
}

macro_rules! unsigned_int {
    ($($ty:ty)*) => {
        $(cast! {
            $ty,
            self => Value::Int(self as _),
            v: i64 => v.try_into().map_err(|_| {
                if v < 0 {
                    "number must be at least zero"
                } else {
                    "number too large"
                }
            })?,
        })*
    }
}

signed_int! { i8 i16 i32 isize }
unsigned_int! { u8 u16 u32 u64 usize }

cast! {
    NonZeroI64,
    self => Value::Int(self.get() as _),
    v: i64 => v.try_into()
        .map_err(|_| if v == 0 {
            "number must not be zero"
        } else {
            "number too large"
        })?,
}

cast! {
    NonZeroIsize,
    self => Value::Int(self.get() as _),
    v: i64 => v
        .try_into()
        .and_then(|v: isize| v.try_into())
        .map_err(|_| if v == 0 {
            "number must not be zero"
        } else {
            "number too large"
        })?,
}

cast! {
    NonZeroU64,
    self => Value::Int(self.get() as _),
    v: i64 => v
        .try_into()
        .and_then(|v: u64| v.try_into())
        .map_err(|_| if v <= 0 {
            "number must be positive"
        } else {
            "number too large"
        })?,
}

cast! {
    NonZeroUsize,
    self => Value::Int(self.get() as _),
    v: i64 => v
        .try_into()
        .and_then(|v: usize| v.try_into())
        .map_err(|_| if v <= 0 {
            "number must be positive"
        } else {
            "number too large"
        })?,
}

fn as_u64(num: i64) -> StrResult<u64> {
    Ok(num.try_into().map_err(|_| {
        if num == 0 {
            "number must not be zero"
        } else {
            "number too large"
        }
    })?)
}

/// The error message when the result is too large to be represented.
#[cold]
fn too_large() -> &'static str {
    "the result is too large"
}
