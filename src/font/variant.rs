use std::fmt::{self, Debug, Formatter};

use serde::{Deserialize, Serialize};

/// Properties that distinguish a font from other fonts in the same family.
#[derive(Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[derive(Serialize, Deserialize)]
pub struct FontVariant {
    /// The style of the font (normal / italic / oblique).
    pub style: FontStyle,
    /// How heavy the font is (100 - 900).
    pub weight: FontWeight,
    /// How condensed or expanded the font is (0.5 - 2.0).
    pub stretch: FontStretch,
}

impl FontVariant {
    /// Create a variant from its three components.
    pub fn new(style: FontStyle, weight: FontWeight, stretch: FontStretch) -> Self {
        Self { style, weight, stretch }
    }
}

impl Debug for FontVariant {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}-{:?}-{:?}", self.style, self.weight, self.stretch)
    }
}

/// The style of a font.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FontStyle {
    /// The default style.
    Normal,
    /// A cursive style.
    Italic,
    /// A slanted style.
    Oblique,
}

impl FontStyle {
    /// The conceptual distance between the styles, expressed as a number.
    pub fn distance(self, other: Self) -> u16 {
        if self == other {
            0
        } else if self != Self::Normal && other != Self::Normal {
            1
        } else {
            2
        }
    }
}

impl Default for FontStyle {
    fn default() -> Self {
        Self::Normal
    }
}

/// The weight of a font.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct FontWeight(u16);

impl FontWeight {
    /// Thin weight (100).
    pub const THIN: Self = Self(100);

    /// Extra light weight (200).
    pub const EXTRALIGHT: Self = Self(200);

    /// Light weight (300).
    pub const LIGHT: Self = Self(300);

    /// Regular weight (400).
    pub const REGULAR: Self = Self(400);

    /// Medium weight (500).
    pub const MEDIUM: Self = Self(500);

    /// Semibold weight (600).
    pub const SEMIBOLD: Self = Self(600);

    /// Bold weight (700).
    pub const BOLD: Self = Self(700);

    /// Extrabold weight (800).
    pub const EXTRABOLD: Self = Self(800);

    /// Black weight (900).
    pub const BLACK: Self = Self(900);

    /// Create a font weight from a number between 100 and 900, clamping it if
    /// necessary.
    pub fn from_number(weight: u16) -> Self {
        Self(weight.max(100).min(900))
    }

    /// The number between 100 and 900.
    pub fn to_number(self) -> u16 {
        self.0
    }

    /// Add (or remove) weight, saturating at the boundaries of 100 and 900.
    pub fn thicken(self, delta: i16) -> Self {
        Self((self.0 as i16).saturating_add(delta).max(100).min(900) as u16)
    }

    /// The absolute number distance between this and another font weight.
    pub fn distance(self, other: Self) -> u16 {
        (self.0 as i16 - other.0 as i16).abs() as u16
    }
}

impl Default for FontWeight {
    fn default() -> Self {
        Self::REGULAR
    }
}

impl Debug for FontWeight {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The width of a font.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct FontStretch(u16);

impl FontStretch {
    /// Ultra-condensed stretch (50%).
    pub const ULTRA_CONDENSED: Self = Self(500);

    /// Extra-condensed stretch weight (62.5%).
    pub const EXTRA_CONDENSED: Self = Self(625);

    /// Condensed stretch (75%).
    pub const CONDENSED: Self = Self(750);

    /// Semi-condensed stretch (87.5%).
    pub const SEMI_CONDENSED: Self = Self(875);

    /// Normal stretch (100%).
    pub const NORMAL: Self = Self(1000);

    /// Semi-expanded stretch (112.5%).
    pub const SEMI_EXPANDED: Self = Self(1125);

    /// Expanded stretch (125%).
    pub const EXPANDED: Self = Self(1250);

    /// Extra-expanded stretch (150%).
    pub const EXTRA_EXPANDED: Self = Self(1500);

    /// Ultra-expanded stretch (200%).
    pub const ULTRA_EXPANDED: Self = Self(2000);

    /// Create a font stretch from a ratio between 0.5 and 2.0, clamping it if
    /// necessary.
    pub fn from_ratio(ratio: f32) -> Self {
        Self((ratio.max(0.5).min(2.0) * 1000.0) as u16)
    }

    /// Create a font stretch from an OpenType-style number between 1 and 9,
    /// clamping it if necessary.
    pub fn from_number(stretch: u16) -> Self {
        match stretch {
            0 | 1 => Self::ULTRA_CONDENSED,
            2 => Self::EXTRA_CONDENSED,
            3 => Self::CONDENSED,
            4 => Self::SEMI_CONDENSED,
            5 => Self::NORMAL,
            6 => Self::SEMI_EXPANDED,
            7 => Self::EXPANDED,
            8 => Self::EXTRA_EXPANDED,
            _ => Self::ULTRA_EXPANDED,
        }
    }

    /// The ratio between 0.5 and 2.0 corresponding to this stretch.
    pub fn to_ratio(self) -> f32 {
        self.0 as f32 / 1000.0
    }

    /// The absolute ratio distance between this and another font stretch.
    pub fn distance(self, other: Self) -> f32 {
        (self.to_ratio() - other.to_ratio()).abs()
    }
}

impl Default for FontStretch {
    fn default() -> Self {
        Self::NORMAL
    }
}

impl Debug for FontStretch {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}%", 100.0 * self.to_ratio())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_weight_distance() {
        let d = |a, b| FontWeight(a).distance(FontWeight(b));
        assert_eq!(d(500, 200), 300);
        assert_eq!(d(500, 500), 0);
        assert_eq!(d(500, 900), 400);
        assert_eq!(d(10, 100), 90);
    }

    #[test]
    fn test_font_stretch_debug() {
        assert_eq!(format!("{:?}", FontStretch::EXPANDED), "125%")
    }
}
