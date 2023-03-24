#[allow(clippy::wildcard_imports /* this module exists to reduce file size, not to introduce a new scope */)]
use super::*;

/// A length that is relative to the font size.
///
/// `1em` is the same as the font size.
#[derive(Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Em(Scalar);

impl Em {
    /// The zero em length.
    #[must_use]
    #[inline]
    pub const fn zero() -> Self {
        Self(Scalar(0.0))
    }

    /// The font size.
    #[must_use]
    #[inline]
    pub const fn one() -> Self {
        Self(Scalar(1.0))
    }

    /// Create a font-relative length.
    #[must_use]
    #[inline]
    pub const fn new(em: f64) -> Self {
        Self(Scalar(em))
    }

    /// Create an em length from font units at the given units per em.
    #[must_use]
    #[inline]
    pub fn from_units(units: impl Into<f64>, units_per_em: f64) -> Self {
        Self(Scalar(units.into() / units_per_em))
    }

    /// Create an em length from a length at the given font size.
    #[must_use]
    #[inline]
    pub fn from_length(length: Abs, font_size: Abs) -> Self {
        let result = length / font_size;
        if result.is_finite() {
            Self(Scalar(result))
        } else {
            Self::zero()
        }
    }

    /// The number of em units.
    #[must_use]
    #[inline]
    pub const fn get(self) -> f64 {
        (self.0).0
    }

    /// The absolute value of this em length.
    #[must_use]
    #[inline]
    pub fn abs(self) -> Self {
        Self::new(self.get().abs())
    }

    /// Convert to an absolute length at the given font size.
    #[must_use]
    #[inline]
    pub fn at(self, font_size: Abs) -> Abs {
        let resolved = font_size * self.get();
        if resolved.is_finite() {
            resolved
        } else {
            Abs::zero()
        }
    }
}

impl Numeric for Em {
    #[inline]
    fn zero() -> Self {
        Self::zero()
    }

    #[inline]
    fn is_finite(self) -> bool {
        self.0.is_finite()
    }
}

impl Debug for Em {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}em", self.get())
    }
}

impl Neg for Em {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        Self(-self.0)
    }
}

impl Add for Em {
    type Output = Self;

    #[inline]
    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

sub_impl!(Em - Em -> Em);

impl Mul<f64> for Em {
    type Output = Self;

    #[inline]
    fn mul(self, other: f64) -> Self {
        Self(self.0 * other)
    }
}

impl Mul<Em> for f64 {
    type Output = Em;

    #[inline]
    fn mul(self, other: Em) -> Em {
        other * self
    }
}

impl Div<f64> for Em {
    type Output = Self;

    #[inline]
    fn div(self, other: f64) -> Self {
        Self(self.0 / other)
    }
}

impl Div for Em {
    type Output = f64;

    #[inline]
    fn div(self, other: Self) -> f64 {
        self.get() / other.get()
    }
}

assign_impl!(Em += Em);
assign_impl!(Em -= Em);
assign_impl!(Em *= f64);
assign_impl!(Em /= f64);

impl Sum for Em {
    #[inline]
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Self(iter.map(|s| s.0).sum())
    }
}

cast_to_value! {
    v: Em => Value::Length(v.into())
}

impl Resolve for Em {
    type Output = Abs;

    #[inline]
    fn resolve(self, styles: StyleChain<'_>) -> Self::Output {
        if self.is_zero() {
            Abs::zero()
        } else {
            self.at(item!(em)(styles))
        }
    }
}
