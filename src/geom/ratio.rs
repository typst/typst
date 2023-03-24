#[allow(clippy::wildcard_imports /* this module exists to reduce file size, not to introduce a new scope */)]
use super::*;

/// A ratio of a whole.
///
/// _Note_: `50%` is represented as `0.5` here, but stored as `50.0` in the
/// corresponding [literal](crate::syntax::ast::Numeric).
#[derive(Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Ratio(Scalar);

impl Ratio {
    /// A ratio of `0%` represented as `0.0`.
    #[must_use]
    #[inline]
    pub const fn zero() -> Self {
        Self(Scalar(0.0))
    }

    /// A ratio of `100%` represented as `1.0`.
    #[must_use]
    #[inline]
    pub const fn one() -> Self {
        Self(Scalar(1.0))
    }

    /// Create a new ratio from a value, where `1.0` means `100%`.
    #[must_use]
    #[inline]
    pub const fn new(ratio: f64) -> Self {
        Self(Scalar(ratio))
    }

    /// Get the underlying ratio.
    #[must_use]
    #[inline]
    pub const fn get(self) -> f64 {
        (self.0).0
    }

    /// Whether the ratio is zero.
    #[must_use]
    #[inline]
    pub fn is_zero(self) -> bool {
        self.0 == 0.0
    }

    /// Whether the ratio is one.
    #[must_use]
    #[inline]
    pub fn is_one(self) -> bool {
        (self.0 .0 - 1.0).abs() < f64::EPSILON
    }

    /// The absolute value of this ratio.
    #[must_use]
    #[inline]
    pub fn abs(self) -> Self {
        Self::new(self.get().abs())
    }

    /// Return the ratio of the given `whole`.
    #[must_use]
    #[inline]
    pub fn of<T: Numeric>(self, whole: T) -> T {
        let resolved = whole * self.get();
        if resolved.is_finite() {
            resolved
        } else {
            T::zero()
        }
    }
}

impl Debug for Ratio {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}%", round_2(100.0 * self.get()))
    }
}

impl Neg for Ratio {
    type Output = Self;

    #[must_use]
    #[inline]
    fn neg(self) -> Self {
        Self(-self.0)
    }
}

impl Add for Ratio {
    type Output = Self;

    #[must_use]
    #[inline]
    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

sub_impl!(Ratio - Ratio -> Ratio);

impl Mul for Ratio {
    type Output = Self;

    #[must_use]
    #[inline]
    fn mul(self, other: Self) -> Self {
        Self(self.0 * other.0)
    }
}

impl Mul<f64> for Ratio {
    type Output = Self;

    #[must_use]
    #[inline]
    fn mul(self, other: f64) -> Self {
        Self(self.0 * other)
    }
}

impl Mul<Ratio> for f64 {
    type Output = Ratio;

    #[must_use]
    #[inline]
    fn mul(self, other: Ratio) -> Ratio {
        other * self
    }
}

impl Div<f64> for Ratio {
    type Output = Self;

    #[must_use]
    #[inline]
    fn div(self, other: f64) -> Self {
        Self(self.0 / other)
    }
}

impl Div for Ratio {
    type Output = f64;

    #[must_use]
    #[inline]
    fn div(self, other: Self) -> f64 {
        self.get() / other.get()
    }
}

assign_impl!(Ratio += Ratio);
assign_impl!(Ratio -= Ratio);
assign_impl!(Ratio *= Ratio);
assign_impl!(Ratio *= f64);
assign_impl!(Ratio /= f64);
