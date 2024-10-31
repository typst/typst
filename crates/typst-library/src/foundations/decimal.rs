use std::fmt::{self, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::Neg;
use std::str::FromStr;

use ecow::{eco_format, EcoString};
use rust_decimal::MathematicalOps;
use typst_syntax::{ast, Span, Spanned};

use crate::diag::{warning, At, SourceResult};
use crate::engine::Engine;
use crate::foundations::{cast, func, repr, scope, ty, Repr, Str};
use crate::World;

/// A fixed-point decimal number type.
///
/// This type should be used for precise arithmetic operations on numbers
/// represented in base 10. A typical use case is representing currency.
///
/// # Example
/// ```example
/// Decimal: #(decimal("0.1") + decimal("0.2")) \
/// Float: #(0.1 + 0.2)
/// ```
///
/// # Construction and casts
/// To create a decimal number, use the `{decimal(string)}` constructor, such as
/// in `{decimal("3.141592653")}` **(note the double quotes!)**. This
/// constructor preserves all given fractional digits, provided they are
/// representable as per the limits specified below (otherwise, an error is
/// raised).
///
/// You can also convert any [integer]($int) to a decimal with the
/// `{decimal(int)}` constructor, e.g. `{decimal(59)}`. However, note that
/// constructing a decimal from a [floating-point number]($float), while
/// supported, **is an imprecise conversion and therefore discouraged.** A
/// warning will be raised if Typst detects that there was an accidental `float`
/// to `decimal` cast through its constructor, e.g. if writing `{decimal(3.14)}`
/// (note the lack of double quotes, indicating this is an accidental `float`
/// cast and therefore imprecise). It is recommended to use strings for
/// constant decimal values instead (e.g. `{decimal("3.14")}`).
///
/// The precision of a `float` to `decimal` cast can be slightly improved by
/// rounding the result to 15 digits with [`calc.round`]($calc.round), but there
/// are still no precision guarantees for that kind of conversion.
///
/// # Operations
/// Basic arithmetic operations are supported on two decimals and on pairs of
/// decimals and integers.
///
/// Built-in operations between `float` and `decimal` are not supported in order
/// to guard against accidental loss of precision. They will raise an error
/// instead.
///
/// Certain `calc` functions, such as trigonometric functions and power between
/// two real numbers, are also only supported for `float` (although raising
/// `decimal` to integer exponents is supported). You can opt into potentially
/// imprecise operations with the `{float(decimal)}` constructor, which casts
/// the `decimal` number into a `float`, allowing for operations without
/// precision guarantees.
///
/// # Displaying decimals
/// To display a decimal, simply insert the value into the document. To only
/// display a certain number of digits, [round]($calc.round) the decimal first.
/// Localized formatting of decimals and other numbers is not yet supported, but
/// planned for the future.
///
/// You can convert decimals to strings using the [`str`] constructor. This way,
/// you can post-process the displayed representation, e.g. to replace the
/// period with a comma (as a stand-in for proper built-in localization to
/// languages that use the comma).
///
/// # Precision and limits
/// A `decimal` number has a limit of 28 to 29 significant base-10 digits. This
/// includes the sum of digits before and after the decimal point. As such,
/// numbers with more fractional digits have a smaller range. The maximum and
/// minimum `decimal` numbers have a value of `{79228162514264337593543950335}`
/// and `{-79228162514264337593543950335}` respectively. In contrast with
/// [`float`], this type does not support infinity or NaN, so overflowing or
/// underflowing operations will raise an error.
///
/// Typical operations between `decimal` numbers, such as addition,
/// multiplication, and [power]($calc.pow) to an integer, will be highly precise
/// due to their fixed-point representation. Note, however, that multiplication
/// and division may not preserve all digits in some edge cases: while they are
/// considered precise, digits past the limits specified above are rounded off
/// and lost, so some loss of precision beyond the maximum representable digits
/// is possible. Note that this behavior can be observed not only when dividing,
/// but also when multiplying by numbers between 0 and 1, as both operations can
/// push a number's fractional digits beyond the limits described above, leading
/// to rounding. When those two operations do not surpass the digit limits, they
/// are fully precise.
#[ty(scope, cast)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Decimal(rust_decimal::Decimal);

impl Decimal {
    pub const ZERO: Self = Self(rust_decimal::Decimal::ZERO);
    pub const ONE: Self = Self(rust_decimal::Decimal::ONE);
    pub const MIN: Self = Self(rust_decimal::Decimal::MIN);
    pub const MAX: Self = Self(rust_decimal::Decimal::MAX);

    /// Whether this decimal value is zero.
    pub const fn is_zero(self) -> bool {
        self.0.is_zero()
    }

    /// Whether this decimal value is negative.
    pub const fn is_negative(self) -> bool {
        self.0.is_sign_negative()
    }

    /// Whether this decimal has fractional part equal to zero (is an integer).
    pub fn is_integer(self) -> bool {
        self.0.is_integer()
    }

    /// Computes the absolute value of this decimal.
    pub fn abs(self) -> Self {
        Self(self.0.abs())
    }

    /// Computes the largest integer less than or equal to this decimal.
    ///
    /// A decimal is returned as this may not be within `i64`'s range of
    /// values.
    pub fn floor(self) -> Self {
        Self(self.0.floor())
    }

    /// Computes the smallest integer greater than or equal to this decimal.
    ///
    /// A decimal is returned as this may not be within `i64`'s range of
    /// values.
    pub fn ceil(self) -> Self {
        Self(self.0.ceil())
    }

    /// Returns the integer part of this decimal.
    pub fn trunc(self) -> Self {
        Self(self.0.trunc())
    }

    /// Returns the fractional part of this decimal (with the integer part set
    /// to zero).
    pub fn fract(self) -> Self {
        Self(self.0.fract())
    }

    /// Rounds this decimal up to the specified amount of digits with the
    /// traditional rounding rules, using the "midpoint away from zero"
    /// strategy (6.5 -> 7, -6.5 -> -7).
    ///
    /// If given a negative amount of digits, rounds to integer digits instead
    /// with the same rounding strategy. For example, rounding to -3 digits
    /// will turn 34567.89 into 35000.00 and -34567.89 into -35000.00.
    ///
    /// Note that this can return `None` when using negative digits where the
    /// rounded number would overflow the available range for decimals.
    pub fn round(self, digits: i32) -> Option<Self> {
        // Positive digits can be handled by just rounding with rust_decimal.
        if let Ok(positive_digits) = u32::try_from(digits) {
            return Some(Self(self.0.round_dp_with_strategy(
                positive_digits,
                rust_decimal::RoundingStrategy::MidpointAwayFromZero,
            )));
        }

        // We received negative digits, so we round to integer digits.
        let mut num = self.0;
        let old_scale = num.scale();
        let digits = -digits as u32;

        let (Ok(_), Some(ten_to_digits)) = (
            // Same as dividing by 10^digits.
            num.set_scale(old_scale + digits),
            rust_decimal::Decimal::TEN.checked_powi(digits as i64),
        ) else {
            // Scaling more than any possible amount of integer digits.
            let mut zero = rust_decimal::Decimal::ZERO;
            zero.set_sign_negative(self.is_negative());
            return Some(Self(zero));
        };

        // Round to this integer digit.
        num = num.round_dp_with_strategy(
            0,
            rust_decimal::RoundingStrategy::MidpointAwayFromZero,
        );

        // Multiply by 10^digits again, which can overflow and fail.
        num.checked_mul(ten_to_digits).map(Self)
    }

    /// Attempts to add two decimals.
    ///
    /// Returns `None` on overflow or underflow.
    pub fn checked_add(self, other: Self) -> Option<Self> {
        self.0.checked_add(other.0).map(Self)
    }

    /// Attempts to subtract a decimal from another.
    ///
    /// Returns `None` on overflow or underflow.
    pub fn checked_sub(self, other: Self) -> Option<Self> {
        self.0.checked_sub(other.0).map(Self)
    }

    /// Attempts to multiply two decimals.
    ///
    /// Returns `None` on overflow or underflow.
    pub fn checked_mul(self, other: Self) -> Option<Self> {
        self.0.checked_mul(other.0).map(Self)
    }

    /// Attempts to divide two decimals.
    ///
    /// Returns `None` if `other` is zero, as well as on overflow or underflow.
    pub fn checked_div(self, other: Self) -> Option<Self> {
        self.0.checked_div(other.0).map(Self)
    }

    /// Attempts to obtain the quotient of Euclidean division between two
    /// decimals. Implemented similarly to [`f64::div_euclid`].
    ///
    /// The returned quotient is truncated and adjusted if the remainder was
    /// negative.
    ///
    /// Returns `None` if `other` is zero, as well as on overflow or underflow.
    pub fn checked_div_euclid(self, other: Self) -> Option<Self> {
        let q = self.0.checked_div(other.0)?.trunc();
        if self
            .0
            .checked_rem(other.0)
            .as_ref()
            .is_some_and(rust_decimal::Decimal::is_sign_negative)
        {
            return if other.0.is_sign_positive() {
                q.checked_sub(rust_decimal::Decimal::ONE).map(Self)
            } else {
                q.checked_add(rust_decimal::Decimal::ONE).map(Self)
            };
        }
        Some(Self(q))
    }

    /// Attempts to obtain the remainder of Euclidean division between two
    /// decimals. Implemented similarly to [`f64::rem_euclid`].
    ///
    /// The returned decimal `r` is non-negative within the range
    /// `0.0 <= r < other.abs()`.
    ///
    /// Returns `None` if `other` is zero, as well as on overflow or underflow.
    pub fn checked_rem_euclid(self, other: Self) -> Option<Self> {
        let r = self.0.checked_rem(other.0)?;
        Some(Self(if r.is_sign_negative() { r.checked_add(other.0.abs())? } else { r }))
    }

    /// Attempts to calculate the remainder of the division of two decimals.
    ///
    /// Returns `None` if `other` is zero, as well as on overflow or underflow.
    pub fn checked_rem(self, other: Self) -> Option<Self> {
        self.0.checked_rem(other.0).map(Self)
    }

    /// Attempts to take one decimal to the power of an integer.
    ///
    /// Returns `None` for invalid operands, as well as on overflow or
    /// underflow.
    pub fn checked_powi(self, other: i64) -> Option<Self> {
        self.0.checked_powi(other).map(Self)
    }
}

#[scope]
impl Decimal {
    /// Converts a value to a `decimal`.
    ///
    /// It is recommended to use a string to construct the decimal number, or an
    /// [integer]($int) (if desired). The string must contain a number in the
    /// format `{"3.14159"}` (or `{"-3.141519"}` for negative numbers). The
    /// fractional digits are fully preserved; if that's not possible due to the
    /// limit of significant digits (around 28 to 29) having been reached, an
    /// error is raised as the given decimal number wouldn't be representable.
    ///
    /// While this constructor can be used with [floating-point numbers]($float)
    /// to cast them to `decimal`, doing so is **discouraged** as **this cast is
    /// inherently imprecise.** It is easy to accidentally perform this cast by
    /// writing `{decimal(1.234)}` (note the lack of double quotes), which is
    /// why Typst will emit a warning in that case. Please write
    /// `{decimal("1.234")}` instead for that particular case (initialization of
    /// a constant decimal). Also note that floats that are NaN or infinite
    /// cannot be cast to decimals and will raise an error.
    ///
    /// ```example
    /// #decimal("1.222222222222222")
    /// ```
    #[func(constructor)]
    pub fn construct(
        engine: &mut Engine,
        /// The value that should be converted to a decimal.
        value: Spanned<ToDecimal>,
    ) -> SourceResult<Decimal> {
        match value.v {
            ToDecimal::Str(str) => Self::from_str(&str.replace(repr::MINUS_SIGN, "-"))
                .map_err(|_| eco_format!("invalid decimal: {str}"))
                .at(value.span),
            ToDecimal::Int(int) => Ok(Self::from(int)),
            ToDecimal::Float(float) => {
                warn_on_float_literal(engine, value.span);
                Self::try_from(float)
                    .map_err(|_| {
                        eco_format!(
                            "float is not a valid decimal: {}",
                            repr::format_float(float, None, true, "")
                        )
                    })
                    .at(value.span)
            }
        }
    }
}

/// Emits a warning when a decimal is constructed from a float literal.
fn warn_on_float_literal(engine: &mut Engine, span: Span) -> Option<()> {
    let id = span.id()?;
    let source = engine.world.source(id).ok()?;
    let node = source.find(span)?;
    if node.is::<ast::Float>() {
        engine.sink.warn(warning!(
            span,
            "creating a decimal using imprecise float literal";
            hint: "use a string in the decimal constructor to avoid loss \
                   of precision: `decimal({})`",
            node.text().repr()
        ));
    }
    Some(())
}

impl FromStr for Decimal {
    type Err = rust_decimal::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        rust_decimal::Decimal::from_str_exact(s).map(Self)
    }
}

impl From<i64> for Decimal {
    fn from(value: i64) -> Self {
        Self(rust_decimal::Decimal::from(value))
    }
}

impl TryFrom<f64> for Decimal {
    type Error = ();

    /// Attempts to convert a Decimal to a float.
    ///
    /// This can fail if the float is infinite or NaN, or otherwise cannot be
    /// represented by a decimal number.
    fn try_from(value: f64) -> Result<Self, Self::Error> {
        rust_decimal::Decimal::from_f64_retain(value).map(Self).ok_or(())
    }
}

impl TryFrom<Decimal> for f64 {
    type Error = rust_decimal::Error;

    /// Attempts to convert a Decimal to a float.
    ///
    /// This should in principle be infallible according to the implementation,
    /// but we mirror the decimal implementation's API either way.
    fn try_from(value: Decimal) -> Result<Self, Self::Error> {
        value.0.try_into()
    }
}

impl TryFrom<Decimal> for i64 {
    type Error = rust_decimal::Error;

    /// Attempts to convert a Decimal to an integer.
    ///
    /// Returns an error if the decimal has a fractional part, or if there
    /// would be overflow or underflow.
    fn try_from(value: Decimal) -> Result<Self, Self::Error> {
        value.0.try_into()
    }
}

impl Display for Decimal {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.0.is_sign_negative() {
            f.write_str(repr::MINUS_SIGN)?;
        }
        self.0.abs().fmt(f)
    }
}

impl Repr for Decimal {
    fn repr(&self) -> EcoString {
        eco_format!("decimal({})", eco_format!("{}", self.0).repr())
    }
}

impl Neg for Decimal {
    type Output = Self;

    fn neg(self) -> Self {
        Self(-self.0)
    }
}

impl Hash for Decimal {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // `rust_decimal`'s Hash implementation normalizes decimals before
        // hashing them. This means decimals with different scales but
        // equivalent value not only compare equal but also hash equally. Here,
        // we hash all bytes explicitly to ensure the scale is also considered.
        // This means that 123.314 == 123.31400, but 123.314.hash() !=
        // 123.31400.hash().
        //
        // Note that this implies that equal decimals can have different hashes,
        // which might generate problems with certain data structures, such as
        // HashSet and HashMap.
        self.0.serialize().hash(state);
    }
}

/// A value that can be cast to a decimal.
pub enum ToDecimal {
    /// A string with the decimal's representation.
    Str(EcoString),
    /// An integer to be converted to the equivalent decimal.
    Int(i64),
    /// A float to be converted to the equivalent decimal.
    Float(f64),
}

cast! {
    ToDecimal,
    v: i64 => Self::Int(v),
    v: f64 => Self::Float(v),
    v: Str => Self::Str(EcoString::from(v)),
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use typst_utils::hash128;

    use super::Decimal;

    #[test]
    fn test_decimals_with_equal_scales_hash_identically() {
        let a = Decimal::from_str("3.14").unwrap();
        let b = Decimal::from_str("3.14").unwrap();
        assert_eq!(a, b);
        assert_eq!(hash128(&a), hash128(&b));
    }

    #[test]
    fn test_decimals_with_different_scales_hash_differently() {
        let a = Decimal::from_str("3.140").unwrap();
        let b = Decimal::from_str("3.14000").unwrap();
        assert_eq!(a, b);
        assert_ne!(hash128(&a), hash128(&b));
    }

    #[track_caller]
    fn test_round(value: &str, digits: i32, expected: &str) {
        assert_eq!(
            Decimal::from_str(value).unwrap().round(digits),
            Some(Decimal::from_str(expected).unwrap()),
        );
    }

    #[test]
    fn test_decimal_positive_round() {
        test_round("312.55553", 0, "313.00000");
        test_round("312.55553", 3, "312.556");
        test_round("312.5555300000", 3, "312.556");
        test_round("-312.55553", 3, "-312.556");
        test_round("312.55553", 28, "312.55553");
        test_round("312.55553", 2341, "312.55553");
        test_round("-312.55553", 2341, "-312.55553");
    }

    #[test]
    fn test_decimal_negative_round() {
        test_round("4596.55553", -1, "4600");
        test_round("4596.555530000000", -1, "4600");
        test_round("-4596.55553", -3, "-5000");
        test_round("4596.55553", -28, "0");
        test_round("-4596.55553", -2341, "0");
        assert_eq!(Decimal::MAX.round(-1), None);
    }
}
