use std::fmt::{self, Display, Formatter};
use std::ops::Neg;
use std::str::FromStr;

use ecow::{eco_format, EcoString};
use rust_decimal::MathematicalOps;

use crate::diag::{warning, At, SourceResult};
use crate::foundations::{cast, func, repr, scope, ty, Engine, Repr, Str};
use crate::syntax::{ast, Span, Spanned};
use crate::World;

/// A fixed-point decimal number type.
///
/// This type should be used when highly precise arithmetic operations are
/// needed, such as for finance. Typical operations between `{decimal}`
/// numbers, such as addition, multiplication, and [power]($calc.pow) to an
/// integer, will be highly precise due to their fixed-point representation.
/// Note, however, that multiplication and division may not preserve all digits
/// in some edge cases: while they are considered precise, digits past the
/// limits specified below are rounded off and lost, so some loss of precision
/// beyond the maximum representable digits is possible. Note that this
/// behavior can be observed not only when dividing, but also when multiplying
/// by numbers between 0 and 1, as both operations can push a number's
/// fractional digits beyond the limits described below, leading to rounding.
/// When those two operations do not surpass the digit limits, they are fully
/// precise.
///
/// # Limits
/// A `{decimal}` number has a limit of 28 to 29 significant base-10 digits.
/// This includes the sum of digits before and after the decimal point. As
/// such, numbers with more fractional digits have a smaller range. The maximum
/// and minimum `{decimal}` numbers have a value of
/// `{79228162514264337593543950335}` and `{-79228162514264337593543950335}`
/// respectively. In contrast with [`{float}`]($float), this type does not
/// support infinity or NaN, so overflowing or underflowing operations will
/// raise an error.
///
/// # Construction and casts
/// To create a decimal number, use the `{decimal(string)}` constructor, such
/// as with `{decimal("3.141592653")}` **(note the double quotes!)**. This
/// constructor preserves all given fractional digits, provided they are
/// representable as per the limits above (otherwise, an error is raised). One
/// may also convert any [integer]($int) to a decimal with the
/// `{decimal(int)}` constructor, e.g. `{decimal(59)}`. However, note that
/// constructing a decimal from a [floating-point number]($float), while
/// supported, **is an imprecise conversion and therefore discouraged.** A
/// warning will be raised if Typst detects that there was an accidental
/// `{float}` to `{decimal}` cast through its constructor (e.g. if writing
/// `{decimal(3.14)}` - note the lack of double quotes, indicating this is
/// an accidental `{float}` cast and therefore imprecise). The precision of a
/// `{float}` to `{decimal}` cast can be slightly improved by rounding the
/// result to 15 digits with [`calc.round`]($calc.round), but there are still
/// no precision guarantees for that kind of conversion.
///
/// In order to guard against accidental loss of precision, built-in operations
/// between `{float}` and `{decimal}` are not supported and will raise an
/// error. Certain `calc` functions, such as trigonometric functions and power
/// between two real numbers, are also only supported for `{float}` (although
/// raising `{decimal}` to integer exponents is supported). You can opt into
/// potentially imprecise operations with the `{float(decimal)}` constructor,
/// which casts the `{decimal}` number into a `{float}`, allowing for
/// operations without precision guarantees.
///
/// # Example
/// ```example
/// #decimal("3.14159265358979323846264338") \
/// #(decimal("0.000000000000000000001") + decimal("0.000000000000000000002"))
/// #(decimal("0.00002") * decimal("49.25652565")) \
/// #(decimal("1") / 2048)
/// ```
#[ty(scope, cast)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Decimal(rust_decimal::Decimal);

impl Decimal {
    pub const ZERO: Self = Self(rust_decimal::Decimal::ZERO);
    pub const ONE: Self = Self(rust_decimal::Decimal::ONE);

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
    pub fn round(self, digits: u32) -> Self {
        Self(self.0.round_dp_with_strategy(
            digits,
            rust_decimal::RoundingStrategy::MidpointAwayFromZero,
        ))
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
    /// Converts a value to a `{decimal}`.
    ///
    /// It is recommended to use a string to construct the decimal number, or
    /// an [integer]($int) (if desired). The string must contain a number in
    /// the format `"3.14159"` (or `"-3.141519"` for negative numbers). The
    /// fractional digits are fully preserved; if that's not possible due to
    /// the limit of significant digits (around 28 to 29) having been reached,
    /// an error is raised as the given decimal number wouldn't be
    /// representable. For example, `{decimal("1.222222222222222")}` is a valid
    /// decimal number.
    ///
    /// While this constructor can be used with
    /// [floating-point numbers]($float) to cast them to `{decimal}`, doing so
    /// is **discouraged** as **this cast is inherently imprecise.** It is easy
    /// to accidentally perform this cast by writing `{decimal(1.234)}` (note
    /// the lack of double quotes), which is why Typst will emit a warning in
    /// that case. Please write `{decimal("1.234")}` instead for that
    /// particular case (initialization of a constant decimal). Also note that
    /// floats equal to NaN and infinity cannot be cast to decimals and will
    /// raise an error.
    #[func(constructor)]
    pub fn construct(
        engine: &mut Engine,
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
