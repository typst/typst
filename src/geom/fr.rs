use super::*;

/// A fractional length.
#[derive(Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Fractional(Scalar);

impl Fractional {
    /// Takes up zero space: `0fr`.
    pub const fn zero() -> Self {
        Self(Scalar(0.0))
    }

    /// Takes up as much space as all other items with this fractional size: `1fr`.
    pub const fn one() -> Self {
        Self(Scalar(1.0))
    }

    /// Create a new fractional value.
    pub const fn new(ratio: f64) -> Self {
        Self(Scalar(ratio))
    }

    /// Get the underlying ratio.
    pub const fn get(self) -> f64 {
        (self.0).0
    }

    /// Whether the ratio is zero.
    pub fn is_zero(self) -> bool {
        self.0 == 0.0
    }

    /// The absolute value of the this fractional.
    pub fn abs(self) -> Self {
        Self::new(self.get().abs())
    }

    /// Resolve this fractionals share in the remaining space.
    pub fn resolve(self, total: Self, remaining: Length) -> Length {
        let ratio = self / total;
        if ratio.is_finite() && remaining.is_finite() {
            ratio * remaining
        } else {
            Length::zero()
        }
    }
}

impl Debug for Fractional {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}fr", round_2(self.get()))
    }
}

impl Neg for Fractional {
    type Output = Self;

    fn neg(self) -> Self {
        Self(-self.0)
    }
}

impl Add for Fractional {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

sub_impl!(Fractional - Fractional -> Fractional);

impl Mul<f64> for Fractional {
    type Output = Self;

    fn mul(self, other: f64) -> Self {
        Self(self.0 * other)
    }
}

impl Mul<Fractional> for f64 {
    type Output = Fractional;

    fn mul(self, other: Fractional) -> Fractional {
        other * self
    }
}

impl Div<f64> for Fractional {
    type Output = Self;

    fn div(self, other: f64) -> Self {
        Self(self.0 / other)
    }
}

impl Div for Fractional {
    type Output = f64;

    fn div(self, other: Self) -> f64 {
        self.get() / other.get()
    }
}

assign_impl!(Fractional += Fractional);
assign_impl!(Fractional -= Fractional);
assign_impl!(Fractional *= f64);
assign_impl!(Fractional /= f64);
