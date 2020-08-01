//! A length type with a unit.

use std::fmt::{self, Debug, Display, Formatter};
use std::str::FromStr;

/// A length with a unit.
#[derive(Copy, Clone, PartialEq)]
pub struct Length {
    /// The length in the given unit.
    pub val: f64,
    /// The unit of measurement.
    pub unit: Unit,
}

/// Different units of measurement.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum Unit {
    /// Points.
    Pt,
    /// Millimeters.
    Mm,
    /// Centimeters.
    Cm,
    /// Inches.
    In,
    /// Raw units (the implicit unit of all bare `f64` lengths).
    Raw,
}

impl Length {
    /// Create a length from a value with a unit.
    pub const fn new(val: f64, unit: Unit) -> Self {
        Self { val, unit }
    }

    /// Create a length from a number of points.
    pub const fn pt(pt: f64) -> Self {
        Self::new(pt, Unit::Pt)
    }

    /// Create a length from a number of millimeters.
    pub const fn mm(mm: f64) -> Self {
        Self::new(mm, Unit::Mm)
    }

    /// Create a length from a number of centimeters.
    pub const fn cm(cm: f64) -> Self {
        Self::new(cm, Unit::Cm)
    }

    /// Create a length from a number of inches.
    pub const fn inches(inches: f64) -> Self {
        Self::new(inches, Unit::In)
    }

    /// Create a length from a number of raw units.
    pub const fn raw(raw: f64) -> Self {
        Self::new(raw, Unit::Raw)
    }

    /// Convert this to a number of points.
    pub fn as_pt(self) -> f64 {
        self.with_unit(Unit::Pt).val
    }

    /// Convert this to a number of millimeters.
    pub fn as_mm(self) -> f64 {
        self.with_unit(Unit::Mm).val
    }

    /// Convert this to a number of centimeters.
    pub fn as_cm(self) -> f64 {
        self.with_unit(Unit::Cm).val
    }

    /// Convert this to a number of inches.
    pub fn as_inches(self) -> f64 {
        self.with_unit(Unit::In).val
    }

    /// Get the value of this length in raw units.
    pub fn as_raw(self) -> f64 {
        self.with_unit(Unit::Raw).val
    }

    /// Convert this to a length with a different unit.
    pub fn with_unit(self, unit: Unit) -> Length {
        Self {
            val: self.val * self.unit.raw_scale() / unit.raw_scale(),
            unit,
        }
    }
}

impl Unit {
    /// How many raw units correspond to a value of `1.0` in this unit.
    fn raw_scale(self) -> f64 {
        match self {
            Unit::Pt => 1.0,
            Unit::Mm => 2.83465,
            Unit::Cm => 28.3465,
            Unit::In => 72.0,
            Unit::Raw => 1.0,
        }
    }
}

impl Display for Length {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:.2}{}", self.val, self.unit)
    }
}

impl Debug for Length {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for Unit {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(match self {
            Unit::Mm => "mm",
            Unit::Pt => "pt",
            Unit::Cm => "cm",
            Unit::In => "in",
            Unit::Raw => "rw",
        })
    }
}

impl Debug for Unit {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl FromStr for Length {
    type Err = ParseLengthError;

    fn from_str(src: &str) -> Result<Self, Self::Err> {
        let len = src.len();

        // We need at least some number and the unit.
        if len <= 2 {
            return Err(ParseLengthError);
        }

        // We can view the string as bytes since a multibyte UTF-8 char cannot
        // have valid ASCII chars as subbytes.
        let split = len - 2;
        let bytes = src.as_bytes();
        let unit = match &bytes[split..] {
            b"pt" => Unit::Pt,
            b"mm" => Unit::Mm,
            b"cm" => Unit::Cm,
            b"in" => Unit::In,
            _ => return Err(ParseLengthError),
        };

        src[..split]
            .parse::<f64>()
            .map(|val| Length::new(val, unit))
            .map_err(|_| ParseLengthError)
    }
}

/// The error when parsing a length fails.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct ParseLengthError;

impl std::error::Error for ParseLengthError {}

impl Display for ParseLengthError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad("invalid string for length")
    }
}

/// Either an absolute length or a factor of some entity.
#[derive(Copy, Clone, PartialEq)]
#[allow(missing_docs)]
pub enum ScaleLength {
    Absolute(Length),
    Scaled(f64),
}

impl ScaleLength {
    /// Use the absolute value or scale the entity.
    pub fn raw_scaled(&self, entity: f64) -> f64 {
        match *self {
            ScaleLength::Absolute(l) => l.as_raw(),
            ScaleLength::Scaled(s) => s * entity,
        }
    }
}

impl Display for ScaleLength {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ScaleLength::Absolute(length) => write!(f, "{}", length),
            ScaleLength::Scaled(scale) => write!(f, "{}%", scale * 100.0),
        }
    }
}

impl Debug for ScaleLength {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(self, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_length_from_str_parses_correct_value_and_unit() {
        assert_eq!(Length::from_str("2.5cm"), Ok(Length::cm(2.5)));
    }

    #[test]
    fn test_length_from_str_works_with_non_ascii_chars() {
        assert_eq!(Length::from_str("123🚚"), Err(ParseLengthError));
    }

    #[test]
    fn test_length_formats_correctly() {
        assert_eq!(Length::cm(12.728).to_string(), "12.73cm".to_string());
    }

    #[test]
    fn test_length_unit_conversion() {
        assert!((Length::mm(150.0).as_cm() - 15.0) < 1e-4);
    }
}
