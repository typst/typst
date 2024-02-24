use std::cmp::Ordering;
use std::num::ParseFloatError;

use ecow::{eco_format, EcoString};

use crate::diag::{bail, SourceResult, StrResult};
use crate::foundations::{cast, func, repr, scope, ty, Repr, Str, Value};
use crate::layout::{Angle, Ratio};
use crate::syntax::{Span, Spanned};

use super::calc::{minmax, Num};

/// A floating-point number.
///
/// A limited-precision representation of a real number. Typst uses 64 bits to
/// store floats. Wherever a float is expected, you can also pass an
/// [integer]($int).
///
/// You can convert a value to a float with this type's constructor.
///
/// # Example
/// ```example
/// #3.14 \
/// #1e4 \
/// #(10 / 4)
/// ```
#[ty(scope, cast, name = "float")]
type f64;

#[scope]
impl f64 {
    /// Converts a value to a float.
    ///
    /// - Booleans are converted to `0.0` or `1.0`.
    /// - Integers are converted to the closest 64-bit float.
    /// - Ratios are divided by 100%.
    /// - Strings are parsed in base 10 to the closest 64-bit float.
    ///   Exponential notation is supported.
    ///
    /// ```example
    /// #float(false) \
    /// #float(true) \
    /// #float(4) \
    /// #float(40%) \
    /// #float("2.7") \
    /// #float("1e5")
    /// ```
    #[func(constructor)]
    pub fn construct(
        /// The value that should be converted to a float.
        value: ToFloat,
    ) -> f64 {
        value.0
    }

    /// Checks if a float is not a number.
    ///
    /// In IEEE 754, more than one bit pattern represents a NaN. This function
    /// returns `true` if the float is any of those bit patterns.
    ///
    /// ```example
    /// #float.is-nan(0) \
    /// #float.is-nan(1) \
    /// #float.is-nan(calc.nan)
    /// ```
    #[func]
    pub fn is_nan(self) -> bool {
        f64::is_nan(self)
    }

    /// Checks if a float is infinite.
    ///
    /// For floats, there is positive and negative infinity. This function
    /// returns `true` if the float is either positive or negative infinity.
    ///
    /// ```example
    /// #float.is-infinite(0) \
    /// #float.is-infinite(1) \
    /// #float.is-infinite(calc.inf)
    /// ```
    #[func]
    pub fn is_infinite(self) -> bool {
        f64::is_infinite(self)
    }

    /// Calculates the sign of a floating point number.
    ///
    /// - If the number is positive (including `{+0.0}`), returns `{1.0}`.
    /// - If the number is negative (including `{-0.0}`), returns `{-1.0}`.
    /// - If the number is [`{calc.nan}`]($calc.nan), returns
    ///   [`{calc.nan}`]($calc.nan).
    ///
    /// ```example
    /// #(5.0).signum() \
    /// #(-5.0).signum() \
    /// #(0.0).signum() \
    /// ```
    #[func]
    pub fn signum(self) -> f64 {
        f64::signum(self)
    }

    /// Calculates the absolute value of a floating point number.
    ///
    /// ```example
    /// #(-5.6).abs() \
    /// #10.4.abs()
    /// ```
    #[func(title = "Absolute")]
    pub fn abs(self) -> f64 {
        f64::abs(self)
    }

    /// Raises a floating point number to some exponent.
    ///
    /// ```example
    /// #2.5.pow(3)
    /// ```
    #[func(title = "Power")]
    pub fn pow(
        self,
        /// The callsite span.
        span: Span,
        /// The exponent of the power.
        exponent: Spanned<Num>,
    ) -> SourceResult<f64> {
        match exponent.v {
            _ if exponent.v.float() == 0.0 && self == 0.0 => {
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

        let result = if self == std::f64::consts::E {
            exponent.v.float().exp()
        } else if self == 2.0 {
            exponent.v.float().exp2()
        } else if let Num::Int(exponent) = exponent.v {
            self.powi(exponent as i32)
        } else {
            self.powf(exponent.v.float())
        };

        if result.is_nan() {
            bail!(span, "the result is not a real number")
        }

        Ok(result)
    }

    /// Raises a floating point number to some exponent of e.
    ///
    /// ```example
    /// #2.0.exp()
    /// #3.0.exp()
    /// ```
    #[func(title = "Exponential")]
    pub fn exp_(
        self,
        /// The callsite span.
        span: Span,
    ) -> SourceResult<f64> {
        if !self.is_normal() && self != 0.0 {
            bail!(span, "exponent may not be infinite, subnormal, or NaN")
        }

        let result = self.exp();
        if result.is_nan() {
            bail!(span, "the result is not a real number")
        }

        Ok(result)
    }

    /// Calculates the square root of a floating point number. Must not be
    /// negative.
    ///
    /// ```example
    /// #2.5.sqrt()
    /// ```
    #[func(title = "Square Root")]
    pub fn sqrt_(self) -> StrResult<f64> {
        if self < 0.0 {
            bail!("cannot take square root of negative number");
        }
        Ok(self.sqrt())
    }

    /// Calculates the real nth root of a floating point number.
    ///
    /// If the number is negative, then n must be odd.
    ///
    /// ```example
    /// #16.0.root(4) \
    /// #27.0.root(3)
    /// ```
    #[func]
    pub fn root(
        self,
        /// Which root of the radicand to take
        index: Spanned<i64>,
    ) -> SourceResult<f64> {
        if index.v == 0 {
            bail!(index.span, "cannot take the 0th root of a number");
        } else if self < 0.0 {
            if index.v % 2 == 0 {
                bail!(
                    index.span,
                    "negative numbers do not have a real nth root when n is even"
                );
            } else {
                Ok(-(-self).powf(1.0 / index.v as f64))
            }
        } else {
            Ok(self.powf(1.0 / index.v as f64))
        }
    }

    /// Calculates the sine of an angle, interpreted as radians.
    ///
    /// ```example
    /// #1.5.sin()
    /// ```
    #[func(title = "Sine")]
    pub fn sin(self) -> f64 {
        f64::sin(self)
    }

    /// Calculates the cosine of an angle, interpreted as radians.
    ///
    /// ```example
    /// #1.5.cos()
    /// ```
    #[func(title = "Cosine")]
    pub fn cos(self) -> f64 {
        f64::cos(self)
    }

    /// Calculates the tangent of an angle, interpreted as radians.
    ///
    /// ```example
    /// #1.5.tan()
    /// ```
    #[func(title = "Tangent")]
    pub fn tan(self) -> f64 {
        f64::tan(self)
    }

    /// Calculates the arcsine of a floating point number, which must be
    /// between -1 and 1.
    ///
    /// ```example
    /// #0.0.asin() \
    /// #1.0.asin()
    /// ```
    #[func(title = "Arcsine")]
    pub fn asin(self) -> StrResult<Angle> {
        if self < -1.0 || self > 1.0 {
            bail!("value must be between -1 and 1");
        }
        Ok(Angle::rad(f64::asin(self)))
    }

    /// Calculates the arccosine of a floating point number, which must be
    /// between -1 and 1.
    ///
    /// ```example
    /// #0.0.acos() \
    /// #1.0.acos()
    /// ```
    #[func(title = "Arccosine")]
    pub fn acos(self) -> StrResult<Angle> {
        if self < -1.0 || self > 1.0 {
            bail!("value must be between -1 and 1");
        }
        Ok(Angle::rad(f64::acos(self)))
    }

    /// Calculates the arctangent of a floating point number.
    ///
    /// ```example
    /// #0.0.atan() \
    /// #1.0.atan()
    /// ```
    #[func(title = "Arctangent")]
    pub fn atan(self) -> Angle {
        Angle::rad(f64::atan(self))
    }

    /// Calculates the four-quadrant arctangent of a coordinate.
    ///
    /// The arguments are `(x, y)`, not `(y, x)`.
    ///
    /// ```example
    /// #float.atan2(1.0, 1.0) \
    /// #float.atan2(-2.0, -3.0)
    /// ```
    #[func(title = "Four-quadrant Arctangent")]
    pub fn atan2_(
        self,
        /// The Y coordinate.
        y: Num,
    ) -> Angle {
        Angle::rad(f64::atan2(y.float(), self))
    }

    /// Calculates the hyperbolic sine of a hyperbolic angle.
    ///
    /// ```example
    /// #1.5.sinh()
    /// ```
    #[func(title = "Hyperbolic Sine")]
    pub fn sinh(self) -> f64 {
        f64::sinh(self)
    }

    /// Calculates the hyperbolic cosine of a hyperbolic angle.
    ///
    /// ```example
    /// #1.5.cosh()
    /// ```
    #[func(title = "Hyperbolic Cosine")]
    pub fn cosh(self) -> f64 {
        f64::cosh(self)
    }

    /// Calculates the hyperbolic tangent of an hyperbolic angle.
    ///
    /// ```example
    /// #1.5.tanh()
    /// ```
    #[func(title = "Hyperbolic Tangent")]
    pub fn tanh(self) -> f64 {
        f64::tanh(self)
    }

    /// Calculates the logarithm of a floating point number, which must be
    /// strictly positive.
    ///
    /// If the base is not specified, the logarithm is calculated in base 10.
    ///
    /// ```example
    /// #100.0.log()
    /// ```
    #[func(title = "Logarithm")]
    pub fn log_(
        self,
        /// The callsite span.
        span: Span,
        /// The base of the logarithm. May not be zero.
        #[named]
        #[default(Spanned::new(10.0, Span::detached()))]
        base: Spanned<f64>,
    ) -> SourceResult<f64> {
        if self <= 0.0 {
            bail!(span, "value must be strictly positive")
        }

        if !base.v.is_normal() {
            bail!(base.span, "base may not be zero, NaN, infinite, or subnormal")
        }

        let result = if base.v == std::f64::consts::E {
            self.ln()
        } else if base.v == 2.0 {
            self.log2()
        } else if base.v == 10.0 {
            self.log10()
        } else {
            self.log(base.v)
        };

        if result.is_infinite() || result.is_nan() {
            bail!(span, "the result is not a real number")
        }

        Ok(result)
    }

    /// Calculates the natural logarithm of a number, which must be strictly
    /// positive.
    ///
    /// ```example
    /// #(1.0.exp()).ln()
    /// ```
    #[func(title = "Natural Logarithm")]
    pub fn ln_(
        self,
        /// The callsite span.
        span: Span,
    ) -> SourceResult<f64> {
        if self <= 0.0 {
            bail!(span, "value must be strictly positive")
        }

        let result = self.ln();
        if result.is_infinite() {
            bail!(span, "result close to -inf")
        }

        Ok(result)
    }

    /// Rounds a floating point number down to the nearest integer.
    ///
    /// ```example
    /// #assert.eq(3.14.floor(), 3)
    /// #assert.eq(3.0.floor(), 3)
    /// #500.1.floor()
    /// ```
    #[func]
    pub fn floor(self) -> i64 {
        f64::floor(self) as i64
    }

    /// Rounds a floating point number up to the nearest integer.
    ///
    /// ```example
    /// #assert.eq(3.14.ceil(), 4)
    /// #assert.eq(3.0.ceil(), 3)
    /// #500.1.ceil()
    /// ```
    #[func]
    pub fn ceil(self) -> i64 {
        f64::ceil(self) as i64
    }

    /// Returns the integer part of a floating point number.
    ///
    /// ```example
    /// #assert.eq(3.0.trunc(), 3)
    /// #assert.eq((-3.7).trunc(), -3)
    /// #15.9.trunc()
    /// ```
    #[func(title = "Truncate")]
    pub fn trunc(self) -> i64 {
        f64::trunc(self) as i64
    }

    /// Returns the fractional part of a floating point number.
    ///
    /// ```example
    /// #assert.eq(3.0.fract(), 0.0)
    /// #(-3.1).fract()
    /// ```
    #[func(title = "Fractional")]
    pub fn fract(self) -> f64 {
        f64::fract(self)
    }

    /// Rounds a floating point number to the nearest integer.
    ///
    /// Optionally, a number of decimal places can be specified.
    ///
    /// ```example
    /// #assert.eq(3.14.round(), 3)
    /// #assert.eq(3.5.round(), 4)
    /// #3.1415.round(digits: 2)
    /// ```
    #[func]
    pub fn round_(
        self,
        /// The number of decimal places.
        #[named]
        #[default(0)]
        digits: i64,
    ) -> f64 {
        let factor = 10.0_f64.powi(digits as i32);
        (self * factor).round() / factor
    }

    /// Clamps a floating point number between a minimum and maximum value.
    ///
    /// ```example
    /// #assert.eq(5.0.clamp(0.0, 10.0), 5.0)
    /// #assert.eq(5.0.clamp(6.2, 10.0), 6.2)
    /// #5.5.clamp(0.0, 4.0)
    /// ```
    #[func]
    pub fn clamp_(
        self,
        /// The inclusive minimum value.
        min: Num,
        /// The inclusive maximum value.
        max: Spanned<Num>,
    ) -> SourceResult<f64> {
        let min_float = min.float();
        let max_float = max.v.float();
        if max_float < min_float {
            bail!(max.span, "max must be greater than or equal to min")
        }
        Ok(self.clamp(min_float, max_float))
    }

    /// Determines the minimum of a sequence of numbers.
    ///
    /// ```example
    /// #float.min(1.5, -3.5, -5.5, 20.5, 3.5, 6.5)
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
        values.insert(0, Spanned::new(Value::Float(self), span));
        minmax(span, values, Ordering::Less)
    }

    /// Determines the maximum of a sequence of numbers.
    ///
    /// ```example
    /// #float.max(1.5, -3.5, -5.5, 20.5, 3.5, 6.5)
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
        values.insert(0, Spanned::new(Value::Float(self), span));
        minmax(span, values, Ordering::Greater)
    }

    /// Calculates the remainder of two numbers.
    ///
    /// The value `x.rem(y)` always has the same sign as `x`, and is smaller
    /// in magnitude than `y`.
    ///
    /// ```example
    /// #7.0.rem(3.0) \
    /// #7.0.rem(-3.0) \
    /// #(-7.0).rem(3.0) \
    /// #(-7.0).rem(-3.0) \
    /// #1.75.rem(0.5)
    /// ```
    #[func(title = "Remainder")]
    pub fn rem(
        self,
        /// The divisor of the remainder.
        divisor: Spanned<Num>,
    ) -> SourceResult<f64> {
        let divisor_float = divisor.v.float();
        if divisor_float == 0.0 {
            bail!(divisor.span, "divisor must not be zero");
        }
        Ok(self % divisor_float)
    }

    /// Performs euclidean division of two numbers.
    ///
    /// The result of this computation is that of a division rounded to the integer
    /// `{n}` such that the dividend is greater than or equal to `{n}` times the divisor.
    ///
    /// ```example
    /// #7.0.div-euclid(3.0) \
    /// #7.0.div-euclid(-3.0) \
    /// #(-7.0).div-euclid(3.0) \
    /// #(-7.0).div-euclid(-3.0) \
    /// #1.75.div-euclid(0.5)
    /// ```
    #[func(title = "Euclidean Division")]
    pub fn div_euclid_(
        self,
        /// The divisor of the division.
        divisor: Spanned<Num>,
    ) -> SourceResult<f64> {
        let divisor_float = divisor.v.float();
        if divisor_float == 0.0 {
            bail!(divisor.span, "divisor must not be zero");
        }
        Ok(self.div_euclid(divisor_float))
    }

    /// This calculates the least nonnegative remainder of a division.
    ///
    /// Warning: Due to a floating point round-off error, the remainder may equal the absolute
    /// value of the divisor if the dividend is much smaller in magnitude than the divisor
    /// and the dividend is negative. This only applies for floating point inputs.
    ///
    /// ```example
    /// #7.0.rem-euclid(3.0) \
    /// #7.0.rem-euclid(-3.0) \
    /// #(-7.0).rem-euclid(3.0) \
    /// #(-7.0).rem-euclid(-3.0) \
    /// #1.75.rem-euclid(0.5)
    /// ```
    #[func(title = "Euclidean Remainder")]
    pub fn rem_euclid_(
        self,
        /// The divisor of the remainder.
        divisor: Spanned<Num>,
    ) -> SourceResult<f64> {
        let divisor_float = divisor.v.float();
        if divisor_float == 0.0 {
            bail!(divisor.span, "divisor must not be zero");
        }
        Ok(self.rem_euclid(divisor_float))
    }

    /// Calculates the quotient (floored division) of two numbers.
    ///
    /// ```example
    /// $ "quo"(a, b) &= floor(a/b) \
    ///   "quo"(14, 5) &= #14.0.quo(5.0) \
    ///   "quo"(3.46, 0.5) &= #3.46.quo(0.5) $
    /// ```
    #[func(title = "Quotient")]
    pub fn quo(
        self,
        /// The divisor of the quotient.
        divisor: Spanned<Num>,
    ) -> SourceResult<i64> {
        let divisor_float = divisor.v.float();
        if divisor_float == 0.0 {
            bail!(divisor.span, "divisor must not be zero");
        }

        Ok(f64Ext::floor(self / divisor_float))
    }
}

impl Repr for f64 {
    fn repr(&self) -> EcoString {
        repr::format_float(*self, None, true, "")
    }
}

/// A value that can be cast to a float.
pub struct ToFloat(f64);

cast! {
    ToFloat,
    v: bool => Self(v as i64 as f64),
    v: i64 => Self(v as f64),
    v: Ratio => Self(v.get()),
    v: Str => Self(
        parse_float(v.clone().into())
            .map_err(|_| eco_format!("invalid float: {}", v))?
    ),
    v: f64 => Self(v),
}

fn parse_float(s: EcoString) -> Result<f64, ParseFloatError> {
    s.replace(repr::MINUS_SIGN, "-").parse()
}
