use std::fmt::{self, Display, Formatter};
use std::ops::Neg;
use std::str::FromStr;

use ecow::{eco_format, EcoString};
use rust_decimal::MathematicalOps;

use crate::diag::{warning, At, SourceResult};
use crate::foundations::{cast, func, repr, scope, ty, Engine, Repr, Str};
use crate::syntax::{ast, Spanned};
use crate::World;

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
    /// Constructs or converts a value to a decimal.
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
                if let Some(file) = value.span.id() {
                    if let Ok(source) = engine.world.source(file) {
                        if source.find(value.span).is_some_and(|v| v.is::<ast::Float>()) {
                            engine.sink.warn(
                                warning!(
                                    value.span,
                                    "creating a decimal using imprecise float literal";
                                    hint: "use a string in the decimal constructor, e.g. `decimal(\"3.14\")`, to avoid loss of precision"
                                )
                            );
                        }
                    }
                }

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
        eco_format!("decimal({})", (&*self.0.to_string()).repr())
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
