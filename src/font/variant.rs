use std::fmt::{self, Debug, Formatter};

use az::{Az as _, SaturatingAs as _};
use serde::{Deserialize, Serialize};

use crate::eval::{cast_from_value, cast_to_value, Cast, Value};
use crate::geom::Ratio;

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
    #[must_use]
    pub fn new(style: FontStyle, weight: FontWeight, stretch: FontStretch) -> Self {
        Self { style, weight, stretch }
    }
}

impl Debug for FontVariant {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}-{:?}-{:?}", self.style, self.weight, self.stretch)
    }
}

/// The style of a font.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[derive(Serialize, Deserialize, Cast)]
#[serde(rename_all = "kebab-case")]
pub enum FontStyle {
    /// The default, typically upright style.
    Normal,
    /// A cursive style with custom letterform.
    Italic,
    /// Just a slanted version of the normal style.
    Oblique,
}

impl FontStyle {
    /// The conceptual distance between the styles, expressed as a number.
    #[must_use]
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

    const MIN: u16 = 100;
    const MAX: u16 = 900;

    /// Create a font weight from a number between 100 and 900, clamping it if
    /// necessary.
    #[must_use]
    pub fn from_number(weight: u16) -> Self {
        Self(weight.max(Self::MIN).min(Self::MAX))
    }

    /// The number between 100 and 900.
    #[must_use]
    pub fn to_number(self) -> u16 {
        self.0
    }

    /// Add (or remove) weight, saturating at the boundaries of 100 and 900.
    #[must_use]
    pub fn thicken(self, delta: i16) -> Self {
        Self(
            self.0
                .az::<i16>()
                .saturating_add(delta)
                .max(Self::MIN.az())
                .min(Self::MAX.az())
                .az(),
        )
    }

    /// The absolute number distance between this and another font weight.
    #[must_use]
    pub fn distance(self, other: Self) -> u16 {
        (self.0.az::<i16>() - other.0.az::<i16>()).unsigned_abs()
    }
}

impl Default for FontWeight {
    fn default() -> Self {
        Self::REGULAR
    }
}

impl Debug for FontWeight {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

cast_from_value! {
    FontWeight,
    v: i64 => Self::from_number(v.saturating_as()),
    /// Thin weight (100).
    "thin" => Self::THIN,
    /// Extra light weight (200).
    "extralight" => Self::EXTRALIGHT,
    /// Light weight (300).
    "light" => Self::LIGHT,
    /// Regular weight (400).
    "regular" => Self::REGULAR,
    /// Medium weight (500).
    "medium" => Self::MEDIUM,
    /// Semibold weight (600).
    "semibold" => Self::SEMIBOLD,
    /// Bold weight (700).
    "bold" => Self::BOLD,
    /// Extrabold weight (800).
    "extrabold" => Self::EXTRABOLD,
    /// Black weight (900).
    "black" => Self::BLACK,
}

cast_to_value! {
    v: FontWeight => Value::from(match v {
        FontWeight::THIN => "thin",
        FontWeight::EXTRALIGHT => "extralight",
        FontWeight::LIGHT => "light",
        FontWeight::REGULAR => "regular",
        FontWeight::MEDIUM => "medium",
        FontWeight::SEMIBOLD => "semibold",
        FontWeight::BOLD => "bold",
        FontWeight::EXTRABOLD => "extrabold",
        FontWeight::BLACK => "black",
        _ => return v.to_number().into(),
    })
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
    #[must_use]
    pub fn from_ratio(ratio: Ratio) -> Self {
        Self((ratio.get().max(0.5).min(2.0) * 1000.0).az())
    }

    /// Create a font stretch from an OpenType-style number between 1 and 9,
    /// clamping it if necessary.
    #[must_use]
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
    #[must_use]
    pub fn to_ratio(self) -> Ratio {
        Ratio::new(f64::from(self.0) / 1000.0)
    }

    /// The absolute ratio distance between this and another font stretch.
    #[must_use]
    pub fn distance(self, other: Self) -> Ratio {
        (self.to_ratio() - other.to_ratio()).abs()
    }
}

impl Default for FontStretch {
    fn default() -> Self {
        Self::NORMAL
    }
}

impl Debug for FontStretch {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.to_ratio().fmt(f)
    }
}

cast_from_value! {
    FontStretch,
    v: Ratio => Self::from_ratio(v),
}

cast_to_value! {
    v: FontStretch => v.to_ratio().into()
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
        assert_eq!(format!("{:?}", FontStretch::EXPANDED), "125%");
    }
}
