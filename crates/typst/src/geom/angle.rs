use super::*;

/// An angle.
#[derive(Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Angle(Scalar);

impl Angle {
    /// The zero angle.
    pub const fn zero() -> Self {
        Self(Scalar(0.0))
    }

    /// Create an angle from a number of raw units.
    pub const fn raw(raw: f64) -> Self {
        Self(Scalar(raw))
    }

    /// Create an angle from a value in a unit.
    pub fn with_unit(val: f64, unit: AngleUnit) -> Self {
        Self(Scalar(val * unit.raw_scale()))
    }

    /// Create an angle from a number of radians.
    pub fn rad(rad: f64) -> Self {
        Self::with_unit(rad, AngleUnit::Rad)
    }

    /// Create an angle from a number of degrees.
    pub fn deg(deg: f64) -> Self {
        Self::with_unit(deg, AngleUnit::Deg)
    }

    /// Get the value of this angle in raw units.
    pub const fn to_raw(self) -> f64 {
        (self.0).0
    }

    /// Get the value of this angle in a unit.
    pub fn to_unit(self, unit: AngleUnit) -> f64 {
        self.to_raw() / unit.raw_scale()
    }

    /// Convert this to a number of radians.
    pub fn to_rad(self) -> f64 {
        self.to_unit(AngleUnit::Rad)
    }

    /// Convert this to a number of degrees.
    pub fn to_deg(self) -> f64 {
        self.to_unit(AngleUnit::Deg)
    }

    /// The absolute value of the this angle.
    pub fn abs(self) -> Self {
        Self::raw(self.to_raw().abs())
    }

    /// Get the sine of this angle in radians.
    pub fn sin(self) -> f64 {
        self.to_rad().sin()
    }

    /// Get the cosine of this angle in radians.
    pub fn cos(self) -> f64 {
        self.to_rad().cos()
    }

    /// Get the tangent of this angle in radians.
    pub fn tan(self) -> f64 {
        self.to_rad().tan()
    }
}

impl Numeric for Angle {
    fn zero() -> Self {
        Self::zero()
    }

    fn is_finite(self) -> bool {
        self.0.is_finite()
    }
}

impl Debug for Angle {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}deg", round_2(self.to_deg()))
    }
}

impl Neg for Angle {
    type Output = Self;

    fn neg(self) -> Self {
        Self(-self.0)
    }
}

impl Add for Angle {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

sub_impl!(Angle - Angle -> Angle);

impl Mul<f64> for Angle {
    type Output = Self;

    fn mul(self, other: f64) -> Self {
        Self(self.0 * other)
    }
}

impl Mul<Angle> for f64 {
    type Output = Angle;

    fn mul(self, other: Angle) -> Angle {
        other * self
    }
}

impl Div<f64> for Angle {
    type Output = Self;

    fn div(self, other: f64) -> Self {
        Self(self.0 / other)
    }
}

impl Div for Angle {
    type Output = f64;

    fn div(self, other: Self) -> f64 {
        self.to_raw() / other.to_raw()
    }
}

assign_impl!(Angle += Angle);
assign_impl!(Angle -= Angle);
assign_impl!(Angle *= f64);
assign_impl!(Angle /= f64);

impl Sum for Angle {
    fn sum<I: Iterator<Item = Angle>>(iter: I) -> Self {
        Self(iter.map(|s| s.0).sum())
    }
}

/// Different units of angular measurement.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub enum AngleUnit {
    /// Radians.
    Rad,
    /// Degrees.
    Deg,
}

impl AngleUnit {
    /// How many raw units correspond to a value of `1.0` in this unit.
    fn raw_scale(self) -> f64 {
        match self {
            Self::Rad => 1.0,
            Self::Deg => PI / 180.0,
        }
    }
}

impl Debug for AngleUnit {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(match self {
            Self::Rad => "rad",
            Self::Deg => "deg",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_angle_unit_conversion() {
        assert!((Angle::rad(2.0 * PI).to_deg() - 360.0) < 1e-4);
        assert!((Angle::deg(45.0).to_rad() - std::f64::consts::FRAC_PI_4) < 1e-4);
    }
}
