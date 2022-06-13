use super::*;

/// A fraction of remaining space.
#[derive(Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Fraction(Scalar);

impl Fraction {
    /// Takes up zero space: `0fr`.
    pub const fn zero() -> Self {
        Self(Scalar(0.0))
    }

    /// Takes up as much space as all other items with this fraction: `1fr`.
    pub const fn one() -> Self {
        Self(Scalar(1.0))
    }

    /// Create a new fraction.
    pub const fn new(ratio: f64) -> Self {
        Self(Scalar(ratio))
    }

    /// Get the underlying number.
    pub const fn get(self) -> f64 {
        (self.0).0
    }

    /// The absolute value of this fraction.
    pub fn abs(self) -> Self {
        Self::new(self.get().abs())
    }

    /// Determine this fraction's share in the remaining space.
    pub fn share(self, total: Self, remaining: Length) -> Length {
        let ratio = self / total;
        if ratio.is_finite() && remaining.is_finite() {
            ratio * remaining
        } else {
            Length::zero()
        }
    }
}

impl Numeric for Fraction {
    fn zero() -> Self {
        Self::zero()
    }

    fn is_finite(self) -> bool {
        self.0.is_finite()
    }
}

impl Debug for Fraction {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}fr", round_2(self.get()))
    }
}

impl Neg for Fraction {
    type Output = Self;

    fn neg(self) -> Self {
        Self(-self.0)
    }
}

impl Add for Fraction {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

sub_impl!(Fraction - Fraction -> Fraction);

impl Mul<f64> for Fraction {
    type Output = Self;

    fn mul(self, other: f64) -> Self {
        Self(self.0 * other)
    }
}

impl Mul<Fraction> for f64 {
    type Output = Fraction;

    fn mul(self, other: Fraction) -> Fraction {
        other * self
    }
}

impl Div<f64> for Fraction {
    type Output = Self;

    fn div(self, other: f64) -> Self {
        Self(self.0 / other)
    }
}

impl Div for Fraction {
    type Output = f64;

    fn div(self, other: Self) -> f64 {
        self.get() / other.get()
    }
}

assign_impl!(Fraction += Fraction);
assign_impl!(Fraction -= Fraction);
assign_impl!(Fraction *= f64);
assign_impl!(Fraction /= f64);

impl Sum for Fraction {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Self(iter.map(|s| s.0).sum())
    }
}
