use super::*;

/// A length that is relative to the font size.
///
/// `1em` is the same as the font size.
#[derive(Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Em(Scalar);

impl Em {
    /// The zero length.
    pub const fn zero() -> Self {
        Self(Scalar(0.0))
    }

    /// The font size.
    pub const fn one() -> Self {
        Self(Scalar(1.0))
    }

    /// Create an font-relative length.
    pub const fn new(em: f64) -> Self {
        Self(Scalar(em))
    }

    /// Create font units at the given units per em.
    pub fn from_units(units: impl Into<f64>, units_per_em: f64) -> Self {
        Self(Scalar(units.into() / units_per_em))
    }

    /// Convert to a length at the given font size.
    pub fn resolve(self, font_size: Length) -> Length {
        self.get() * font_size
    }

    /// The number of em units.
    pub const fn get(self) -> f64 {
        (self.0).0
    }

    /// Whether the length is zero.
    pub fn is_zero(self) -> bool {
        self.0 == 0.0
    }
}

impl Debug for Em {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}em", self.get())
    }
}

impl Neg for Em {
    type Output = Self;

    fn neg(self) -> Self {
        Self(-self.0)
    }
}

impl Add for Em {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

sub_impl!(Em - Em -> Em);

impl Mul<f64> for Em {
    type Output = Self;

    fn mul(self, other: f64) -> Self {
        Self(self.0 * other)
    }
}

impl Mul<Em> for f64 {
    type Output = Em;

    fn mul(self, other: Em) -> Em {
        other * self
    }
}

impl Div<f64> for Em {
    type Output = Self;

    fn div(self, other: f64) -> Self {
        Self(self.0 / other)
    }
}

impl Div for Em {
    type Output = f64;

    fn div(self, other: Self) -> f64 {
        self.get() / other.get()
    }
}

assign_impl!(Em += Em);
assign_impl!(Em -= Em);
assign_impl!(Em *= f64);
assign_impl!(Em /= f64);
