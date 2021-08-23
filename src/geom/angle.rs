use super::*;

/// An angle.
#[derive(Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[derive(Serialize, Deserialize)]
pub struct Angle(N64);

impl Angle {
    /// The zero angle.
    pub fn zero() -> Self {
        Self(N64::from(0.0))
    }

    /// Create an angle from a number of radians.
    pub fn rad(rad: f64) -> Self {
        Self::with_unit(rad, AngularUnit::Rad)
    }

    /// Create an angle from a number of degrees.
    pub fn deg(deg: f64) -> Self {
        Self::with_unit(deg, AngularUnit::Deg)
    }

    /// Create an angle from a number of raw units.
    pub fn raw(raw: f64) -> Self {
        Self(N64::from(raw))
    }

    /// Convert this to a number of radians.
    pub fn to_rad(self) -> f64 {
        self.to_unit(AngularUnit::Rad)
    }

    /// Convert this to a number of degrees.
    pub fn to_deg(self) -> f64 {
        self.to_unit(AngularUnit::Deg)
    }

    /// Get the value of this angle in raw units.
    pub fn to_raw(self) -> f64 {
        self.0.into()
    }

    /// Create an angle from a value in a unit.
    pub fn with_unit(val: f64, unit: AngularUnit) -> Self {
        Self(N64::from(val * unit.raw_scale()))
    }

    /// Get the value of this length in unit.
    pub fn to_unit(self, unit: AngularUnit) -> f64 {
        self.to_raw() / unit.raw_scale()
    }
}

impl Debug for Angle {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let unit = AngularUnit::Deg;
        write!(f, "{}{}", self.to_unit(unit), unit)
    }
}

impl Display for Angle {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        // Format with the unit that yields the shortest output, preferring
        // degrees when tied.
        let unit = [AngularUnit::Deg, AngularUnit::Rad]
            .iter()
            .copied()
            .min_by_key(|&unit| self.to_unit(unit).to_string().len())
            .unwrap();

        write!(f, "{}{}", self.to_unit(unit), unit)
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
        iter.fold(Angle::zero(), Add::add)
    }
}
/// Different units of angular measurement.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum AngularUnit {
    /// Radians.
    Rad,
    /// Degrees.
    Deg,
}

impl AngularUnit {
    /// How many raw units correspond to a value of `1.0` in this unit.
    fn raw_scale(self) -> f64 {
        match self {
            Self::Rad => 1.0,
            Self::Deg => PI / 180.0,
        }
    }
}

impl Display for AngularUnit {
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
        assert!((Angle::deg(45.0).to_rad() - 0.7854) < 1e-4);
    }
}
