use super::*;

/// A combined relative and absolute length.
#[derive(Default, Copy, Clone, PartialEq, Hash)]
pub struct Linear {
    /// The relative part.
    pub rel: Relative,
    /// The absolute part.
    pub abs: Length,
}

impl Linear {
    /// The zero linear.
    pub fn zero() -> Self {
        Self {
            rel: Relative::zero(),
            abs: Length::zero(),
        }
    }

    /// The linear with a relative part of `100%` and no absolute part.
    pub fn one() -> Self {
        Self {
            rel: Relative::one(),
            abs: Length::zero(),
        }
    }

    /// Create a new linear.
    pub fn new(rel: Relative, abs: Length) -> Self {
        Self { rel, abs }
    }

    /// Resolve this relative to the given `length`.
    pub fn resolve(self, length: Length) -> Length {
        self.rel.resolve(length) + self.abs
    }

    /// Whether both parts are zero.
    pub fn is_zero(self) -> bool {
        self.rel.is_zero() && self.abs.is_zero()
    }
}

impl Display for Linear {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{} + {}", self.rel, self.abs)
    }
}

impl Debug for Linear {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?} + {:?}", self.rel, self.abs)
    }
}

impl From<Length> for Linear {
    fn from(abs: Length) -> Self {
        Self { rel: Relative::zero(), abs }
    }
}

impl From<Relative> for Linear {
    fn from(rel: Relative) -> Self {
        Self { rel, abs: Length::zero() }
    }
}

impl Neg for Linear {
    type Output = Self;

    fn neg(self) -> Self {
        Self { rel: -self.rel, abs: -self.abs }
    }
}

impl Add for Linear {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            rel: self.rel + other.rel,
            abs: self.abs + other.abs,
        }
    }
}

impl Add<Relative> for Length {
    type Output = Linear;

    fn add(self, other: Relative) -> Linear {
        Linear { rel: other, abs: self }
    }
}

impl Add<Length> for Relative {
    type Output = Linear;

    fn add(self, other: Length) -> Linear {
        other + self
    }
}

impl Add<Length> for Linear {
    type Output = Self;

    fn add(self, other: Length) -> Self {
        Self { rel: self.rel, abs: self.abs + other }
    }
}

impl Add<Linear> for Length {
    type Output = Linear;

    fn add(self, other: Linear) -> Linear {
        other + self
    }
}

impl Add<Relative> for Linear {
    type Output = Self;

    fn add(self, other: Relative) -> Self {
        Self { rel: self.rel + other, abs: self.abs }
    }
}

impl Add<Linear> for Relative {
    type Output = Linear;

    fn add(self, other: Linear) -> Linear {
        other + self
    }
}

sub_impl!(Linear - Linear -> Linear);
sub_impl!(Length - Relative -> Linear);
sub_impl!(Relative - Length -> Linear);
sub_impl!(Linear - Length -> Linear);
sub_impl!(Length - Linear -> Linear);
sub_impl!(Linear - Relative -> Linear);
sub_impl!(Relative - Linear -> Linear);

impl Mul<f64> for Linear {
    type Output = Self;

    fn mul(self, other: f64) -> Self {
        Self {
            rel: self.rel * other,
            abs: self.abs * other,
        }
    }
}

impl Mul<Linear> for f64 {
    type Output = Linear;

    fn mul(self, other: Linear) -> Linear {
        Linear {
            rel: self * other.rel,
            abs: self * other.abs,
        }
    }
}

impl Div<f64> for Linear {
    type Output = Self;

    fn div(self, other: f64) -> Self {
        Self {
            rel: self.rel / other,
            abs: self.abs / other,
        }
    }
}

assign_impl!(Linear += Linear);
assign_impl!(Linear += Length);
assign_impl!(Linear += Relative);
assign_impl!(Linear -= Linear);
assign_impl!(Linear -= Length);
assign_impl!(Linear -= Relative);
assign_impl!(Linear *= f64);
assign_impl!(Linear /= f64);
