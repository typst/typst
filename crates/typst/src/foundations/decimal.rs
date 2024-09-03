use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};
use std::str::FromStr;

use ecow::{eco_format, EcoString};
use rust_decimal::MathematicalOps;

use crate::diag::StrResult;
use crate::foundations::{func, scope, ty, Repr};

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

    /// Attempts to calculate `e` to the power of `self`.
    ///
    /// Returns `None` on overflow.
    pub fn checked_exp(self) -> Option<Self> {
        self.0.checked_exp().map(Self)
    }

    /// Attempts to calculate this decimal number's square root.
    ///
    /// Returns `None` if negative.
    pub fn checked_sqrt(self) -> Option<Self> {
        self.0.sqrt().map(Self)
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

    /// Attempts to take one decimal to the power of an integer.
    ///
    /// Returns `None` for invalid operands, as well as on overflow or
    /// underflow.
    pub fn checked_powi(self, other: i64) -> Option<Self> {
        self.0.checked_powi(other).map(Self)
    }

    /// Attempts to take one decimal to the power of another.
    ///
    /// Returns `None` for invalid operands, as well as on overflow or
    /// underflow.
    pub fn checked_pow(self, other: Self) -> Option<Self> {
        self.0.checked_powd(other.0).map(Self)
    }

    /// Calculates the sine of this angle in radians.
    pub fn sin(self) -> Self {
        Self(self.0.sin())
    }

    /// Calculates the cosine of this angle in radians.
    pub fn cos(self) -> Self {
        Self(self.0.cos())
    }

    /// Calculates the tangent of this angle in radians.
    ///
    /// Returns `None` on overflow.
    pub fn checked_tan(self) -> Option<Self> {
        self.0.checked_tan().map(Self)
    }
}

#[scope]
impl Decimal {
    pub const E: Self = Self(rust_decimal::Decimal::E);
    pub const PI: Self = Self(rust_decimal::Decimal::PI);

    #[func(constructor)]
    pub fn construct(value: EcoString) -> StrResult<Decimal> {
        Self::from_str(&value).map_err(|_| eco_format!("invalid decimal"))
    }

    /// Display this decimal value with the given amount of decimals.
    #[func]
    pub fn display(
        &self,
        #[named]
        #[default(2)]
        decimals: u32,
    ) -> EcoString {
        eco_format!(
            "{}",
            self.0.round_dp_with_strategy(
                decimals,
                rust_decimal::RoundingStrategy::MidpointAwayFromZero,
            )
        )
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

impl Repr for Decimal {
    fn repr(&self) -> EcoString {
        eco_format!("decimal({})", (&*self.0.to_string()).repr())
    }
}

impl Add for Decimal {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl Add<i64> for Decimal {
    type Output = Self;

    fn add(self, rhs: i64) -> Self {
        Self(self.0 + rust_decimal::Decimal::from(rhs))
    }
}

impl Neg for Decimal {
    type Output = Self;

    fn neg(self) -> Self {
        Self(-self.0)
    }
}

impl Sub for Decimal {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0)
    }
}

impl Sub<i64> for Decimal {
    type Output = Self;

    fn sub(self, rhs: i64) -> Self {
        Self(self.0 - rust_decimal::Decimal::from(rhs))
    }
}

impl Sub<Decimal> for i64 {
    type Output = Decimal;

    fn sub(self, rhs: Decimal) -> Decimal {
        Decimal(rust_decimal::Decimal::from(self) - rhs.0)
    }
}

impl Mul for Decimal {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        Self(self.0 * rhs.0)
    }
}

impl Mul<i64> for Decimal {
    type Output = Self;

    fn mul(self, rhs: i64) -> Self {
        Self(self.0 * rust_decimal::Decimal::from(rhs))
    }
}

impl Div for Decimal {
    type Output = Self;

    fn div(self, rhs: Self) -> Self {
        Self(self.0 / rhs.0)
    }
}

impl Div<i64> for Decimal {
    type Output = Self;

    fn div(self, rhs: i64) -> Self {
        Self(self.0 / rust_decimal::Decimal::from(rhs))
    }
}

impl Div<Decimal> for i64 {
    type Output = Decimal;

    fn div(self, rhs: Decimal) -> Decimal {
        Decimal(rust_decimal::Decimal::from(self) / rhs.0)
    }
}

impl AddAssign for Decimal {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl SubAssign for Decimal {
    fn sub_assign(&mut self, other: Self) {
        self.0 -= other.0;
    }
}

impl MulAssign for Decimal {
    fn mul_assign(&mut self, other: Self) {
        self.0 *= other.0;
    }
}

impl DivAssign for Decimal {
    fn div_assign(&mut self, other: Self) {
        self.0 /= other.0;
    }
}
