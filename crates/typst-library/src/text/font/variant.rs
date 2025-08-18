use std::fmt::{self, Debug, Formatter};

use ecow::EcoString;
use serde::{Deserialize, Serialize};

use crate::foundations::{Cast, IntoValue, Repr, cast};
use crate::layout::Ratio;

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

impl From<usvg::FontStyle> for FontStyle {
    fn from(style: usvg::FontStyle) -> Self {
        match style {
            usvg::FontStyle::Normal => Self::Normal,
            usvg::FontStyle::Italic => Self::Italic,
            usvg::FontStyle::Oblique => Self::Oblique,
        }
    }
}

/// The weight of a font.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct FontWeight(pub(super) u16);

/// Font weight names and numbers.
/// See `<https://developer.mozilla.org/en-US/docs/Web/CSS/@font-face/font-weight#common_weight_name_mapping>`
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
        Self(weight.clamp(100, 900))
    }

    /// The number between 100 and 900.
    pub fn to_number(self) -> u16 {
        self.0
    }

    /// Add (or remove) weight, saturating at the boundaries of 100 and 900.
    pub fn thicken(self, delta: i16) -> Self {
        Self((self.0 as i16).saturating_add(delta).clamp(100, 900) as u16)
    }

    /// The absolute number distance between this and another font weight.
    pub fn distance(self, other: Self) -> u16 {
        (self.0 as i16 - other.0 as i16).unsigned_abs()
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

impl From<fontdb::Weight> for FontWeight {
    fn from(weight: fontdb::Weight) -> Self {
        Self::from_number(weight.0)
    }
}

cast! {
    FontWeight,
    self => IntoValue::into_value(match self {
        FontWeight::THIN => "thin",
        FontWeight::EXTRALIGHT => "extralight",
        FontWeight::LIGHT => "light",
        FontWeight::REGULAR => "regular",
        FontWeight::MEDIUM => "medium",
        FontWeight::SEMIBOLD => "semibold",
        FontWeight::BOLD => "bold",
        FontWeight::EXTRABOLD => "extrabold",
        FontWeight::BLACK => "black",
        _ => return self.to_number().into_value(),
    }),
    v: i64 => Self::from_number(v.clamp(0, u16::MAX as i64) as u16),
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

/// The width of a font.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct FontStretch(pub(super) u16);

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
    pub fn from_ratio(ratio: Ratio) -> Self {
        Self((ratio.get().clamp(0.5, 2.0) * 1000.0) as u16)
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
    pub fn to_ratio(self) -> Ratio {
        Ratio::new(self.0 as f64 / 1000.0)
    }

    /// Round to one of the pre-defined variants.
    pub fn round(self) -> Self {
        match self.0 {
            ..=562 => Self::ULTRA_CONDENSED,
            563..=687 => Self::EXTRA_CONDENSED,
            688..=812 => Self::CONDENSED,
            813..=937 => Self::SEMI_CONDENSED,
            938..=1062 => Self::NORMAL,
            1063..=1187 => Self::SEMI_EXPANDED,
            1188..=1374 => Self::EXPANDED,
            1375..=1749 => Self::EXTRA_EXPANDED,
            1750.. => Self::ULTRA_EXPANDED,
        }
    }

    /// The absolute ratio distance between this and another font stretch.
    pub fn distance(self, other: Self) -> Ratio {
        (self.to_ratio() - other.to_ratio()).abs()
    }
}

impl Default for FontStretch {
    fn default() -> Self {
        Self::NORMAL
    }
}

impl Repr for FontStretch {
    fn repr(&self) -> EcoString {
        self.to_ratio().repr()
    }
}

impl From<usvg::FontStretch> for FontStretch {
    fn from(stretch: usvg::FontStretch) -> Self {
        match stretch {
            usvg::FontStretch::UltraCondensed => Self::ULTRA_CONDENSED,
            usvg::FontStretch::ExtraCondensed => Self::EXTRA_CONDENSED,
            usvg::FontStretch::Condensed => Self::CONDENSED,
            usvg::FontStretch::SemiCondensed => Self::SEMI_CONDENSED,
            usvg::FontStretch::Normal => Self::NORMAL,
            usvg::FontStretch::SemiExpanded => Self::SEMI_EXPANDED,
            usvg::FontStretch::Expanded => Self::EXPANDED,
            usvg::FontStretch::ExtraExpanded => Self::EXTRA_EXPANDED,
            usvg::FontStretch::UltraExpanded => Self::ULTRA_EXPANDED,
        }
    }
}

cast! {
    FontStretch,
    self => self.to_ratio().into_value(),
    v: Ratio => Self::from_ratio(v),
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
        assert_eq!(FontStretch::EXPANDED.repr(), "125%")
    }
}
