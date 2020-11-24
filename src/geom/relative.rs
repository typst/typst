use super::*;

/// A relative length.
///
/// _Note_: `50%` is represented as `0.5` here, but stored as `50.0` in the
/// corresponding [literal](crate::syntax::Lit::Percent).
#[derive(Default, Copy, Clone, PartialEq, PartialOrd)]
pub struct Relative(f64);

impl Relative {
    /// A ratio of `0%` represented as `0.0`.
    pub const ZERO: Self = Self(0.0);

    /// A ratio of `100%` represented as `1.0`.
    pub const ONE: Self = Self(1.0);

    /// Create a new relative value.
    pub fn new(ratio: f64) -> Self {
        Self(ratio)
    }

    /// Get the underlying ratio.
    pub fn get(self) -> f64 {
        self.0
    }

    /// Resolve this relative to the given `length`.
    pub fn resolve(self, length: Length) -> Length {
        self.get() * length
    }
}

impl Display for Relative {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:.2}%", self.0)
    }
}

impl Debug for Relative {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl Neg for Relative {
    type Output = Self;

    fn neg(self) -> Self {
        Self(-self.0)
    }
}

impl Add for Relative {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

sub_impl!(Relative - Relative -> Relative);

impl Mul<f64> for Relative {
    type Output = Self;

    fn mul(self, other: f64) -> Self {
        Self(self.0 * other)
    }
}

impl Mul<Relative> for f64 {
    type Output = Relative;

    fn mul(self, other: Relative) -> Relative {
        other * self
    }
}

impl Div<f64> for Relative {
    type Output = Self;

    fn div(self, other: f64) -> Self {
        Self(self.0 / other)
    }
}

assign_impl!(Relative += Relative);
assign_impl!(Relative -= Relative);
assign_impl!(Relative *= f64);
assign_impl!(Relative /= f64);
