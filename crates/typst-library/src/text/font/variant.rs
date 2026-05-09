use std::fmt::{self, Debug, Formatter};
use std::ops::RangeInclusive;

use ecow::EcoString;
use serde::{Deserialize, Serialize};

use crate::foundations::{Cast, IntoValue, Repr, cast};
use crate::layout::Ratio;

/// A static (fixed) field value for non-variable fonts.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[derive(Serialize, Deserialize)]
pub struct StaticField<T>(pub T);

/// A variable field with a range and default value for variable fonts.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[derive(Serialize, Deserialize)]
pub struct VariableField<T> {
    /// The supported range of values.
    pub range: RangeInclusive<T>,
    /// The default value within the range.
    pub default: T,
}

/// A field that can be either static or variable (for variable fonts).
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[derive(Serialize, Deserialize)]
pub enum Field<T> {
    /// A static (fixed) value.
    Static(StaticField<T>),
    /// A variable value with a range.
    Variable(VariableField<T>),
}

impl<T> Field<T> {
    /// Get the default value for this field.
    pub fn default_value(&self) -> &T {
        match self {
            Field::Static(s) => &s.0,
            Field::Variable(v) => &v.default,
        }
    }

    /// Check if this field is variable.
    pub fn is_variable(&self) -> bool {
        matches!(self, Field::Variable(_))
    }
}

impl<T: Default> Default for Field<T> {
    fn default() -> Self {
        Self::Static(StaticField(T::default()))
    }
}

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

/// Information about a variable font's slant/italic axis.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Default)]
#[derive(Serialize, Deserialize)]
pub enum SlantAxis {
    /// No slant axis (static font or variable font without slnt/ital).
    #[default]
    None,
    /// Has a slnt (slant) axis with the given range in degrees.
    /// Negative values = right-leaning (italic/oblique), positive = left-leaning.
    Slnt {
        /// Minimum slant value (usually negative for italic).
        min: i16,
        /// Maximum slant value (usually 0 for upright).
        max: i16,
        /// Default slant value.
        default: i16,
    },
    /// Has an ital (italic) axis (binary: 0 = upright, 1 = italic).
    Ital {
        /// Whether the font defaults to italic.
        default_italic: bool,
    },
}

/// Information about a variable font's optical size (opsz) axis.
#[derive(Debug, Clone, Default)]
#[derive(Serialize, Deserialize)]
pub enum OpticalSizeAxis {
    /// No optical size axis (static font or variable font without opsz).
    #[default]
    None,
    /// Has an opsz (optical size) axis with the given range.
    /// Values are typically in points (e.g., 8-144).
    Opsz {
        /// Minimum optical size value.
        min: f32,
        /// Maximum optical size value.
        max: f32,
        /// Default optical size value.
        default: f32,
    },
}

impl PartialEq for OpticalSizeAxis {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::None, Self::None) => true,
            (
                Self::Opsz { min: min1, max: max1, default: default1 },
                Self::Opsz { min: min2, max: max2, default: default2 },
            ) => {
                min1.to_bits() == min2.to_bits()
                    && max1.to_bits() == max2.to_bits()
                    && default1.to_bits() == default2.to_bits()
            }
            _ => false,
        }
    }
}

impl Eq for OpticalSizeAxis {}

impl std::hash::Hash for OpticalSizeAxis {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        if let Self::Opsz { min, max, default } = self {
            min.to_bits().hash(state);
            max.to_bits().hash(state);
            default.to_bits().hash(state);
        }
    }
}

/// Properties describing the coverage of a font variant, supporting variable fonts.
///
/// For static fonts, each property is a single value.
/// For variable fonts, weight and stretch can have ranges.
#[derive(Default, Clone, Eq, PartialEq, Hash)]
#[derive(Serialize, Deserialize)]
pub struct FontVariantCoverage {
    /// The style of the font (normal / italic / oblique).
    pub style: FontStyle,
    /// The weight coverage: either a static value or a variable range.
    pub weight: Field<FontWeight>,
    /// The stretch coverage: either a static value or a variable range.
    pub stretch: Field<FontStretch>,
    /// Information about the slant/italic axis for variable fonts.
    pub slant_axis: SlantAxis,
    /// Information about the optical size axis for variable fonts.
    pub optical_size_axis: OpticalSizeAxis,
}

impl FontVariantCoverage {
    /// Create a new variant coverage from its components.
    pub fn new(
        style: FontStyle,
        weight: Field<FontWeight>,
        stretch: Field<FontStretch>,
    ) -> Self {
        Self {
            style,
            weight,
            stretch,
            slant_axis: SlantAxis::None,
            optical_size_axis: OpticalSizeAxis::None,
        }
    }

    /// Create a new variant coverage with slant axis information.
    pub fn with_slant(
        style: FontStyle,
        weight: Field<FontWeight>,
        stretch: Field<FontStretch>,
        slant_axis: SlantAxis,
    ) -> Self {
        Self {
            style,
            weight,
            stretch,
            slant_axis,
            optical_size_axis: OpticalSizeAxis::None,
        }
    }

    /// Create a new variant coverage with slant and optical size axis information.
    pub fn with_axes(
        style: FontStyle,
        weight: Field<FontWeight>,
        stretch: Field<FontStretch>,
        slant_axis: SlantAxis,
        optical_size_axis: OpticalSizeAxis,
    ) -> Self {
        Self {
            style,
            weight,
            stretch,
            slant_axis,
            optical_size_axis,
        }
    }

    /// Check if this font has a variable slant or italic axis.
    pub fn has_slant_axis(&self) -> bool {
        !matches!(self.slant_axis, SlantAxis::None)
    }

    /// Check if this font supports the requested variant.
    ///
    /// Returns true if the style matches and the weight/stretch are within range.
    pub fn supports(&self, variant: &FontVariant) -> bool {
        if self.style != variant.style {
            return false;
        }

        let weight_ok = match &self.weight {
            Field::Static(s) => s.0 == variant.weight,
            Field::Variable(v) => v.range.contains(&variant.weight),
        };

        let stretch_ok = match &self.stretch {
            Field::Static(s) => s.0 == variant.stretch,
            Field::Variable(v) => v.range.contains(&variant.stretch),
        };

        weight_ok && stretch_ok
    }

    /// Compute the distance between this coverage and a requested variant.
    ///
    /// For variable fonts, if the requested value is within range, the distance is 0.
    /// Otherwise, it returns the distance to the nearest edge of the range.
    pub fn distance(&self, variant: &FontVariant) -> (u16, Ratio, u16) {
        // For style distance, if the font has a slant/ital axis, it can produce
        // italic/oblique styles, so the distance should be 0 for those requests.
        let style_dist = match &self.slant_axis {
            SlantAxis::Slnt { min, max, .. } => {
                // A slnt axis can produce oblique/italic if it has negative values
                let can_produce_slant = *min < 0 || *max < 0;
                match (self.style, variant.style) {
                    // Same style = distance 0
                    (a, b) if a == b => 0,
                    // Font is normal, user wants italic/oblique, and we have slant axis
                    (FontStyle::Normal, FontStyle::Italic | FontStyle::Oblique)
                        if can_produce_slant =>
                    {
                        0
                    }
                    // Otherwise use the regular distance
                    _ => self.style.distance(variant.style),
                }
            }
            SlantAxis::Ital { .. } => {
                // An ital axis can toggle between normal and italic
                match (self.style, variant.style) {
                    // Same style = distance 0
                    (a, b) if a == b => 0,
                    // Font is normal, user wants italic/oblique
                    (FontStyle::Normal, FontStyle::Italic | FontStyle::Oblique) => 0,
                    // Font is italic, user wants normal
                    (FontStyle::Italic, FontStyle::Normal) => 0,
                    // Otherwise use the regular distance
                    _ => self.style.distance(variant.style),
                }
            }
            SlantAxis::None => self.style.distance(variant.style),
        };

        let weight_dist = match &self.weight {
            Field::Static(s) => s.0.distance(variant.weight),
            Field::Variable(v) => {
                if v.range.contains(&variant.weight) {
                    0
                } else if variant.weight < *v.range.start() {
                    v.range.start().distance(variant.weight)
                } else {
                    v.range.end().distance(variant.weight)
                }
            }
        };

        let stretch_dist = match &self.stretch {
            Field::Static(s) => s.0.distance(variant.stretch),
            Field::Variable(v) => {
                if v.range.contains(&variant.stretch) {
                    Ratio::zero()
                } else if variant.stretch < *v.range.start() {
                    v.range.start().distance(variant.stretch)
                } else {
                    v.range.end().distance(variant.stretch)
                }
            }
        };

        (style_dist, stretch_dist, weight_dist)
    }

    /// Get the default variant for this coverage.
    pub fn default_variant(&self) -> FontVariant {
        FontVariant {
            style: self.style,
            weight: *self.weight.default_value(),
            stretch: *self.stretch.default_value(),
        }
    }

    /// Check if this font has an optical size axis.
    pub fn has_optical_size_axis(&self) -> bool {
        !matches!(self.optical_size_axis, OpticalSizeAxis::None)
    }

    /// Check if this is a variable font (has variable weight, stretch, slant, or optical size).
    pub fn is_variable(&self) -> bool {
        self.weight.is_variable()
            || self.stretch.is_variable()
            || self.has_slant_axis()
            || self.has_optical_size_axis()
    }
}

impl Debug for FontVariantCoverage {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}-{:?}-{:?}", self.style, self.weight, self.stretch)
    }
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
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[derive(Cast, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FontStyle {
    /// The default, typically upright style.
    #[default]
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
