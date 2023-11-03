use super::*;

/// Defines how the the remaining space in a layout is distributed.
///
/// Each fractionally sized element gets space based on the ratio of its
/// fraction to the sum of all fractions.
///
/// For more details, also see the [h]($h) and [v]($v) functions and the
/// [grid function]($grid).
///
/// # Example
/// ```example
/// Left #h(1fr) Left-ish #h(2fr) Right
/// ```
#[ty(name = "fraction")]
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Fr(Scalar);

impl Fr {
    /// Takes up zero space: `0fr`.
    pub const fn zero() -> Self {
        Self(Scalar::ZERO)
    }

    /// Takes up as much space as all other items with this fraction: `1fr`.
    pub const fn one() -> Self {
        Self(Scalar::ONE)
    }

    /// Create a new fraction.
    pub const fn new(ratio: f64) -> Self {
        Self(Scalar::new(ratio))
    }

    /// Get the underlying number.
    pub const fn get(self) -> f64 {
        (self.0).get()
    }

    /// The absolute value of this fraction.
    pub fn abs(self) -> Self {
        Self::new(self.get().abs())
    }

    /// Determine this fraction's share in the remaining space.
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
    fn zero() -> Self {
        Self::zero()
    }

    fn is_finite(self) -> bool {
        self.0.is_finite()
    }
}

impl Repr for Fr {
    fn repr(&self) -> EcoString {
        format_float(self.get(), Some(2), "fr")
    }
}

impl Neg for Fr {
    type Output = Self;

    fn neg(self) -> Self {
        Self(-self.0)
    }
}

impl Add for Fr {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

sub_impl!(Fr - Fr -> Fr);

impl Mul<f64> for Fr {
    type Output = Self;

    fn mul(self, other: f64) -> Self {
        Self(self.0 * other)
    }
}

impl Mul<Fr> for f64 {
    type Output = Fr;

    fn mul(self, other: Fr) -> Fr {
        other * self
    }
}

impl Div for Fr {
    type Output = f64;

    fn div(self, other: Self) -> f64 {
        self.get() / other.get()
    }
}

impl Div<f64> for Fr {
    type Output = Self;

    fn div(self, other: f64) -> Self {
        Self(self.0 / other)
    }
}

assign_impl!(Fr += Fr);
assign_impl!(Fr -= Fr);
assign_impl!(Fr *= f64);
assign_impl!(Fr /= f64);

impl Sum for Fr {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Self(iter.map(|s| s.0).sum())
    }
}
