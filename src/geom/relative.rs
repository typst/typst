use super::*;

/// A relative length.
#[derive(Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Relative {
    /// The relative part.
    pub rel: Ratio,
    /// The absolute part.
    pub abs: Length,
}

impl Relative {
    /// The zero relative length.
    pub const fn zero() -> Self {
        Self { rel: Ratio::zero(), abs: Length::zero() }
    }

    /// A relative length with a ratio of `100%` and no absolute part.
    pub const fn one() -> Self {
        Self { rel: Ratio::one(), abs: Length::zero() }
    }

    /// Create a new relative length from its parts.
    pub const fn new(rel: Ratio, abs: Length) -> Self {
        Self { rel, abs }
    }

    /// Resolve this length relative to the given `length`.
    pub fn resolve(self, length: Length) -> Length {
        self.rel.resolve(length) + self.abs
    }

    /// Whether both parts are zero.
    pub fn is_zero(self) -> bool {
        self.rel.is_zero() && self.abs.is_zero()
    }

    /// Whether there is a relative part.
    pub fn is_relative(self) -> bool {
        !self.rel.is_zero()
    }
}

impl Debug for Relative {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?} + {:?}", self.rel, self.abs)
    }
}

impl From<Length> for Relative {
    fn from(abs: Length) -> Self {
        Self { rel: Ratio::zero(), abs }
    }
}

impl From<Ratio> for Relative {
    fn from(rel: Ratio) -> Self {
        Self { rel, abs: Length::zero() }
    }
}

impl Neg for Relative {
    type Output = Self;

    fn neg(self) -> Self {
        Self { rel: -self.rel, abs: -self.abs }
    }
}

impl Add for Relative {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            rel: self.rel + other.rel,
            abs: self.abs + other.abs,
        }
    }
}

impl Add<Ratio> for Length {
    type Output = Relative;

    fn add(self, other: Ratio) -> Relative {
        Relative { rel: other, abs: self }
    }
}

impl Add<Length> for Ratio {
    type Output = Relative;

    fn add(self, other: Length) -> Relative {
        other + self
    }
}

impl Add<Length> for Relative {
    type Output = Self;

    fn add(self, other: Length) -> Self {
        Self { rel: self.rel, abs: self.abs + other }
    }
}

impl Add<Relative> for Length {
    type Output = Relative;

    fn add(self, other: Relative) -> Relative {
        other + self
    }
}

impl Add<Ratio> for Relative {
    type Output = Self;

    fn add(self, other: Ratio) -> Self {
        Self { rel: self.rel + other, abs: self.abs }
    }
}

impl Add<Relative> for Ratio {
    type Output = Relative;

    fn add(self, other: Relative) -> Relative {
        other + self
    }
}

sub_impl!(Relative - Relative -> Relative);
sub_impl!(Length - Ratio -> Relative);
sub_impl!(Ratio - Length -> Relative);
sub_impl!(Relative - Length -> Relative);
sub_impl!(Length - Relative -> Relative);
sub_impl!(Relative - Ratio -> Relative);
sub_impl!(Ratio - Relative -> Relative);

impl Mul<f64> for Relative {
    type Output = Self;

    fn mul(self, other: f64) -> Self {
        Self {
            rel: self.rel * other,
            abs: self.abs * other,
        }
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
        Self {
            rel: self.rel / other,
            abs: self.abs / other,
        }
    }
}

assign_impl!(Relative += Relative);
assign_impl!(Relative += Length);
assign_impl!(Relative += Ratio);
assign_impl!(Relative -= Relative);
assign_impl!(Relative -= Length);
assign_impl!(Relative -= Ratio);
assign_impl!(Relative *= f64);
assign_impl!(Relative /= f64);
