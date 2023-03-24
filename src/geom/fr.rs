#[allow(clippy::wildcard_imports /* this module exists to reduce file size, not to introduce a new scope */)]
use super::*;

/// A fraction of remaining space.
#[derive(Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Fr(Scalar);

impl Fr {
    /// Takes up zero space: `0fr`.
    #[must_use]
    #[inline]
    pub const fn zero() -> Self {
        Self(Scalar(0.0))
    }

    /// Takes up as much space as all other items with this fraction: `1fr`.
    #[must_use]
    #[inline]
    pub const fn one() -> Self {
        Self(Scalar(1.0))
    }

    /// Create a new fraction.
    #[must_use]
    #[inline]
    pub const fn new(ratio: f64) -> Self {
        Self(Scalar(ratio))
    }

    /// Get the underlying number.
    #[must_use]
    #[inline]
    pub const fn get(self) -> f64 {
        (self.0).0
    }

    /// The absolute value of this fraction.
    #[must_use]
    #[inline]
    pub fn abs(self) -> Self {
        Self::new(self.get().abs())
    }

    /// Determine this fraction's share in the remaining space.
    #[must_use]
    #[inline]
    pub fn share(self, total: Self, remaining: Abs) -> Abs {
        let ratio = self / total;
        if ratio.is_finite() && remaining.is_finite() {
            (ratio * remaining).max(Abs::zero())
        } else {
            Abs::zero()
        }
    }
}

impl Numeric for Fr {
    #[inline]
    fn zero() -> Self {
        Self::zero()
    }

    #[inline]
    fn is_finite(self) -> bool {
        self.0.is_finite()
    }
}

impl Debug for Fr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}fr", round_2(self.get()))
    }
}

impl Neg for Fr {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        Self(-self.0)
    }
}

impl Add for Fr {
    type Output = Self;

    #[inline]
    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

sub_impl!(Fr - Fr -> Fr);

impl Mul<f64> for Fr {
    type Output = Self;

    #[inline]
    fn mul(self, other: f64) -> Self {
        Self(self.0 * other)
    }
}

impl Mul<Fr> for f64 {
    type Output = Fr;

    #[inline]
    fn mul(self, other: Fr) -> Fr {
        other * self
    }
}

impl Div<f64> for Fr {
    type Output = Self;

    #[inline]
    fn div(self, other: f64) -> Self {
        Self(self.0 / other)
    }
}

impl Div for Fr {
    type Output = f64;

    #[inline]
    fn div(self, other: Self) -> f64 {
        self.get() / other.get()
    }
}

assign_impl!(Fr += Fr);
assign_impl!(Fr -= Fr);
assign_impl!(Fr *= f64);
assign_impl!(Fr /= f64);

impl Sum for Fr {
    #[inline]
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Self(iter.map(|s| s.0).sum())
    }
}
