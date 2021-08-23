use super::*;

/// A fractional length.
#[derive(Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Fractional(N64);

impl Fractional {
    /// Takes up zero space: `0fr`.
    pub fn zero() -> Self {
        Self(N64::from(0.0))
    }

    /// Takes up as much space as all other items with this fractional size: `1fr`.
    pub fn one() -> Self {
        Self(N64::from(1.0))
    }

    /// Create a new fractional value.
    pub fn new(ratio: f64) -> Self {
        Self(N64::from(ratio))
    }

    /// Get the underlying ratio.
    pub fn get(self) -> f64 {
        self.0.into()
    }

    /// Whether the ratio is zero.
    pub fn is_zero(self) -> bool {
        self.0 == 0.0
    }
}

impl Debug for Fractional {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for Fractional {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}fr", self.get())
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
