use super::*;

/// An absolute length.
#[derive(Default, Copy, Clone, PartialEq, PartialOrd)]
pub struct Length {
    /// The length in raw units.
    raw: f64,
}

impl Length {
    /// The zero length.
    pub const ZERO: Self = Self { raw: 0.0 };

    /// Create a length from a number of points.
    pub fn pt(pt: f64) -> Self {
        Self::with_unit(pt, LengthUnit::Pt)
    }

    /// Create a length from a number of millimeters.
    pub fn mm(mm: f64) -> Self {
        Self::with_unit(mm, LengthUnit::Mm)
    }

    /// Create a length from a number of centimeters.
    pub fn cm(cm: f64) -> Self {
        Self::with_unit(cm, LengthUnit::Cm)
    }

    /// Create a length from a number of inches.
    pub fn inches(inches: f64) -> Self {
        Self::with_unit(inches, LengthUnit::In)
    }

    /// Create a length from a number of raw units.
    pub fn raw(raw: f64) -> Self {
        Self { raw }
    }

    /// Convert this to a number of points.
    pub fn to_pt(self) -> f64 {
        self.to_unit(LengthUnit::Pt)
    }

    /// Convert this to a number of millimeters.
    pub fn to_mm(self) -> f64 {
        self.to_unit(LengthUnit::Mm)
    }

    /// Convert this to a number of centimeters.
    pub fn to_cm(self) -> f64 {
        self.to_unit(LengthUnit::Cm)
    }

    /// Convert this to a number of inches.
    pub fn to_inches(self) -> f64 {
        self.to_unit(LengthUnit::In)
    }

    /// Get the value of this length in raw units.
    pub fn to_raw(self) -> f64 {
        self.raw
    }

    /// Create a length from a value in a unit.
    pub fn with_unit(val: f64, unit: LengthUnit) -> Self {
        Self { raw: val * unit.raw_scale() }
    }

    /// Get the value of this length in unit.
    pub fn to_unit(self, unit: LengthUnit) -> f64 {
        self.raw / unit.raw_scale()
    }

    /// The minimum of this and another length.
    pub fn min(self, other: Self) -> Self {
        Self { raw: self.raw.min(other.raw) }
    }

    /// The maximum of this and another length.
    pub fn max(self, other: Self) -> Self {
        Self { raw: self.raw.max(other.raw) }
    }

    /// Whether the length is `NaN`.
    pub fn is_nan(self) -> bool {
        self.raw.is_nan()
    }
}

impl Display for Length {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        // Format small lengths as points and large ones as centimeters.
        let (val, unit) = if self.to_pt().abs() < 25.0 {
            (self.to_pt(), LengthUnit::Pt)
        } else {
            (self.to_cm(), LengthUnit::Cm)
        };
        write!(f, "{}{}", (val * 100.0).round() / 100.0, unit)
    }
}

impl Debug for Length {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl Neg for Length {
    type Output = Self;

    fn neg(self) -> Self {
        Self { raw: -self.raw }
    }
}

impl Add for Length {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self { raw: self.raw + other.raw }
    }
}

sub_impl!(Length - Length -> Length);

impl Mul<f64> for Length {
    type Output = Self;

    fn mul(self, other: f64) -> Self {
        Self { raw: self.raw * other }
    }
}

impl Mul<Length> for f64 {
    type Output = Length;

    fn mul(self, other: Length) -> Length {
        other * self
    }
}

impl Div<f64> for Length {
    type Output = Self;

    fn div(self, other: f64) -> Self {
        Self { raw: self.raw / other }
    }
}

impl Div for Length {
    type Output = f64;

    fn div(self, other: Self) -> f64 {
        self.raw / other.raw
    }
}

assign_impl!(Length += Length);
assign_impl!(Length -= Length);
assign_impl!(Length *= f64);
assign_impl!(Length /= f64);

impl Sum for Length {
    fn sum<I: Iterator<Item = Length>>(iter: I) -> Self {
        iter.fold(Length::ZERO, Add::add)
    }
}

/// Different units of length measurement.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum LengthUnit {
    /// Points.
    Pt,
    /// Millimeters.
    Mm,
    /// Centimeters.
    Cm,
    /// Inches.
    In,
}

impl LengthUnit {
    /// How many raw units correspond to a value of `1.0` in this unit.
    fn raw_scale(self) -> f64 {
        match self {
            LengthUnit::Pt => 1.0,
            LengthUnit::Mm => 2.83465,
            LengthUnit::Cm => 28.3465,
            LengthUnit::In => 72.0,
        }
    }
}

impl Display for LengthUnit {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(match self {
            LengthUnit::Mm => "mm",
            LengthUnit::Pt => "pt",
            LengthUnit::Cm => "cm",
            LengthUnit::In => "in",
        })
    }
}

impl Debug for LengthUnit {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(self, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_length_unit_conversion() {
        assert!((Length::mm(150.0).to_cm() - 15.0) < 1e-4);
    }

    #[test]
    fn test_length_formatting() {
        assert_eq!(Length::pt(-28.34).to_string(), "-1cm".to_string());
        assert_eq!(Length::pt(23.0).to_string(), "23pt".to_string());
        assert_eq!(Length::cm(12.728).to_string(), "12.73cm".to_string());
    }
}
