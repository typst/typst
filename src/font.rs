//! Font handling.

use std::collections::{hash_map::Entry, HashMap};
use std::fmt::{self, Debug, Display, Formatter};
use std::ops::Add;
use std::rc::Rc;

use decorum::N64;
use serde::{Deserialize, Serialize};

use crate::geom::Length;
use crate::loading::{FileId, Loader};

/// A font face.
pub struct Face {
    buffer: Rc<Vec<u8>>,
    index: u32,
    ttf: rustybuzz::Face<'static>,
    units_per_em: f64,
    pub ascender: Em,
    pub cap_height: Em,
    pub x_height: Em,
    pub descender: Em,
    pub strikethrough: LineMetrics,
    pub underline: LineMetrics,
    pub overline: LineMetrics,
}

/// Metrics for a decorative line.
pub struct LineMetrics {
    pub strength: Em,
    pub position: Em,
}

impl Face {
    /// Parse a font face from a buffer and collection index.
    pub fn new(buffer: Rc<Vec<u8>>, index: u32) -> Option<Self> {
        // SAFETY:
        // - The slices's location is stable in memory:
        //   - We don't move the underlying vector
        //   - Nobody else can move it since we have a strong ref to the `Rc`.
        // - The internal static lifetime is not leaked because its rewritten
        //   to the self-lifetime in `ttf()`.
        let slice: &'static [u8] =
            unsafe { std::slice::from_raw_parts(buffer.as_ptr(), buffer.len()) };

        let ttf = rustybuzz::Face::from_slice(slice, index)?;

        let units_per_em = f64::from(ttf.units_per_em());
        let to_em = |units| Em::from_units(units, units_per_em);

        let ascender = to_em(ttf.typographic_ascender().unwrap_or(ttf.ascender()));
        let cap_height = ttf.capital_height().filter(|&h| h > 0).map_or(ascender, to_em);
        let x_height = ttf.x_height().filter(|&h| h > 0).map_or(ascender, to_em);
        let descender = to_em(ttf.typographic_descender().unwrap_or(ttf.descender()));

        let strikeout = ttf.strikeout_metrics();
        let underline = ttf.underline_metrics();
        let default = Em::new(0.06);

        let strikethrough = LineMetrics {
            strength: strikeout.or(underline).map_or(default, |s| to_em(s.thickness)),
            position: strikeout.map_or(Em::new(0.25), |s| to_em(s.position)),
        };

        let underline = LineMetrics {
            strength: underline.or(strikeout).map_or(default, |s| to_em(s.thickness)),
            position: underline.map_or(Em::new(-0.2), |s| to_em(s.position)),
        };

        let overline = LineMetrics {
            strength: underline.strength,
            position: cap_height + Em::new(0.1),
        };

        Some(Self {
            buffer,
            index,
            ttf,
            units_per_em,
            ascender,
            cap_height,
            x_height,
            descender,
            strikethrough,
            underline,
            overline,
        })
    }

    /// The underlying buffer.
    pub fn buffer(&self) -> &Rc<Vec<u8>> {
        &self.buffer
    }

    /// The collection index.
    pub fn index(&self) -> u32 {
        self.index
    }

    /// A reference to the underlying `ttf-parser` / `rustybuzz` face.
    pub fn ttf(&self) -> &rustybuzz::Face<'_> {
        // We can't implement Deref because that would leak the internal 'static
        // lifetime.
        &self.ttf
    }

    /// Get the number of units per em.
    pub fn units_per_em(&self) -> f64 {
        self.units_per_em
    }

    /// Convert from font units to an em length.
    pub fn to_em(&self, units: impl Into<f64>) -> Em {
        Em::from_units(units, self.units_per_em)
    }

    /// Look up a vertical metric.
    pub fn vertical_metric(&self, metric: VerticalFontMetric) -> Em {
        match metric {
            VerticalFontMetric::Ascender => self.ascender,
            VerticalFontMetric::CapHeight => self.cap_height,
            VerticalFontMetric::XHeight => self.x_height,
            VerticalFontMetric::Baseline => Em::zero(),
            VerticalFontMetric::Descender => self.descender,
        }
    }
}

/// Identifies a vertical metric of a font.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum VerticalFontMetric {
    /// The distance from the baseline to the typographic ascender.
    ///
    /// Corresponds to the typographic ascender from the `OS/2` table if present
    /// and falls back to the ascender from the `hhea` table otherwise.
    Ascender,
    /// The approximate height of uppercase letters.
    CapHeight,
    /// The approximate height of non-ascending lowercase letters.
    XHeight,
    /// The baseline on which the letters rest.
    Baseline,
    /// The distance from the baseline to the typographic descender.
    ///
    /// Corresponds to the typographic descender from the `OS/2` table if
    /// present and falls back to the descender from the `hhea` table otherwise.
    Descender,
}

impl Display for VerticalFontMetric {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(match self {
            Self::Ascender => "ascender",
            Self::CapHeight => "cap-height",
            Self::XHeight => "x-height",
            Self::Baseline => "baseline",
            Self::Descender => "descender",
        })
    }
}

/// A length in em units.
///
/// `1em` is the same as the font size.
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, PartialOrd)]
pub struct Em(N64);

impl Em {
    /// The zero length.
    pub fn zero() -> Self {
        Self(N64::from(0.0))
    }

    /// Create an em length.
    pub fn new(em: f64) -> Self {
        Self(N64::from(em))
    }

    /// Convert units to an em length at the given units per em.
    pub fn from_units(units: impl Into<f64>, units_per_em: f64) -> Self {
        Self(N64::from(units.into() / units_per_em))
    }

    /// The number of em units.
    pub fn get(self) -> f64 {
        self.0.into()
    }

    /// Convert to a length at the given font size.
    pub fn to_length(self, font_size: Length) -> Length {
        self.get() * font_size
    }
}

impl Add for Em {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

/// Caches parsed font faces.
pub struct FontCache {
    faces: Vec<Option<Face>>,
    families: HashMap<String, Vec<FaceId>>,
    buffers: HashMap<FileId, Rc<Vec<u8>>>,
    on_load: Option<Box<dyn Fn(FaceId, &Face)>>,
}

impl FontCache {
    /// Create a new, empty font cache.
    pub fn new(loader: &dyn Loader) -> Self {
        let mut faces = vec![];
        let mut families = HashMap::<String, Vec<FaceId>>::new();

        for (i, info) in loader.faces().iter().enumerate() {
            let id = FaceId(i as u64);
            faces.push(None);
            families
                .entry(info.family.to_lowercase())
                .and_modify(|vec| vec.push(id))
                .or_insert_with(|| vec![id]);
        }

        Self {
            faces,
            families,
            buffers: HashMap::new(),
            on_load: None,
        }
    }

    /// Query for and load the font face from the given `family` that most
    /// closely matches the given `variant`.
    pub fn select(
        &mut self,
        loader: &mut dyn Loader,
        family: &str,
        variant: FontVariant,
    ) -> Option<FaceId> {
        // Check whether a family with this name exists.
        let ids = self.families.get(family)?;
        let infos = loader.faces();

        let mut best = None;
        let mut best_key = None;

        // Find the best matching variant of this font.
        for &id in ids {
            let current = infos[id.0 as usize].variant;

            // This is a perfect match, no need to search further.
            if current == variant {
                best = Some(id);
                break;
            }

            // If this is not a perfect match, we compute a key that we want to
            // minimize among all variants. This key prioritizes style, then
            // stretch distance and then weight distance.
            let key = (
                current.style != variant.style,
                current.stretch.distance(variant.stretch),
                current.weight.distance(variant.weight),
            );

            if best_key.map_or(true, |b| key < b) {
                best = Some(id);
                best_key = Some(key);
            }
        }

        // Load the face if it's not already loaded.
        let id = best?;
        let idx = id.0 as usize;
        let slot = &mut self.faces[idx];
        if slot.is_none() {
            let FaceInfo { file, index, .. } = infos[idx];

            // Check the buffer cache since multiple faces may
            // refer to the same data (font collection).
            let buffer = match self.buffers.entry(file) {
                Entry::Occupied(entry) => entry.into_mut(),
                Entry::Vacant(entry) => {
                    let buffer = loader.load_file(file)?;
                    entry.insert(Rc::new(buffer))
                }
            };

            let face = Face::new(Rc::clone(buffer), index)?;
            if let Some(callback) = &self.on_load {
                callback(id, &face);
            }

            *slot = Some(face);
        }

        best
    }

    /// Get a reference to a loaded face.
    ///
    /// This panics if no face with this id was loaded. This function should
    /// only be called with ids returned by [`select()`](Self::select).
    #[track_caller]
    pub fn get(&self, id: FaceId) -> &Face {
        self.faces[id.0 as usize].as_ref().expect("font face was not loaded")
    }

    /// Register a callback which is invoked each time a font face is loaded.
    pub fn on_load<F>(&mut self, f: F)
    where
        F: Fn(FaceId, &Face) + 'static,
    {
        self.on_load = Some(Box::new(f));
    }
}

/// A unique identifier for a loaded font face.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct FaceId(u64);

impl FaceId {
    /// Create a face id from the raw underlying value.
    ///
    /// This should only be called with values returned by
    /// [`into_raw`](Self::into_raw).
    pub fn from_raw(v: u64) -> Self {
        Self(v)
    }

    /// Convert into the raw underlying value.
    pub fn into_raw(self) -> u64 {
        self.0
    }
}

/// Properties of a single font face.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct FaceInfo {
    /// The font file.
    pub file: FileId,
    /// The collection index in the font file.
    pub index: u32,
    /// The typographic font family this face is part of.
    pub family: String,
    /// Properties that distinguish this face from other faces in the same
    /// family.
    #[serde(flatten)]
    pub variant: FontVariant,
}

/// Properties that distinguish a face from other faces in the same family.
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[derive(Serialize, Deserialize)]
pub struct FontVariant {
    /// The style of the face (normal / italic / oblique).
    pub style: FontStyle,
    /// How heavy the face is (100 - 900).
    pub weight: FontWeight,
    /// How condensed or expanded the face is (0.5 - 2.0).
    pub stretch: FontStretch,
}

impl FontVariant {
    /// Create a variant from its three components.
    pub fn new(style: FontStyle, weight: FontWeight, stretch: FontStretch) -> Self {
        Self { style, weight, stretch }
    }
}

/// The style of a font face.
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
    /// Create a font style from a lowercase name like `italic`.
    pub fn from_str(name: &str) -> Option<FontStyle> {
        Some(match name {
            "normal" => Self::Normal,
            "italic" => Self::Italic,
            "oblique" => Self::Oblique,
            _ => return None,
        })
    }

    /// The lowercase string representation of this style.
    pub fn to_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Italic => "italic",
            Self::Oblique => "oblique",
        }
    }
}

impl Default for FontStyle {
    fn default() -> Self {
        Self::Normal
    }
}

impl Display for FontStyle {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(self.to_str())
    }
}

/// The weight of a font face.
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

    /// Create a font weight from a lowercase name like `light`.
    pub fn from_str(name: &str) -> Option<Self> {
        Some(match name {
            "thin" => Self::THIN,
            "extralight" => Self::EXTRALIGHT,
            "light" => Self::LIGHT,
            "regular" => Self::REGULAR,
            "medium" => Self::MEDIUM,
            "semibold" => Self::SEMIBOLD,
            "bold" => Self::BOLD,
            "extrabold" => Self::EXTRABOLD,
            "black" => Self::BLACK,
            _ => return None,
        })
    }

    /// The number between 100 and 900.
    pub fn to_number(self) -> u16 {
        self.0
    }

    /// The lowercase string representation of this weight if it is divisible by
    /// 100.
    pub fn to_str(self) -> Option<&'static str> {
        Some(match self {
            Self::THIN => "thin",
            Self::EXTRALIGHT => "extralight",
            Self::LIGHT => "light",
            Self::REGULAR => "regular",
            Self::MEDIUM => "medium",
            Self::SEMIBOLD => "semibold",
            Self::BOLD => "bold",
            Self::EXTRABOLD => "extrabold",
            Self::BLACK => "black",
            _ => return None,
        })
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

impl Display for FontWeight {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self.to_str() {
            Some(name) => f.pad(name),
            None => write!(f, "{}", self.0),
        }
    }
}

impl Debug for FontWeight {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.pad(match *self {
            Self::THIN => "Thin",
            Self::EXTRALIGHT => "Extralight",
            Self::LIGHT => "Light",
            Self::REGULAR => "Regular",
            Self::MEDIUM => "Medium",
            Self::SEMIBOLD => "Semibold",
            Self::BOLD => "Bold",
            Self::EXTRABOLD => "Extrabold",
            Self::BLACK => "Black",
            _ => return write!(f, "{}", self.0),
        })
    }
}

/// The width of a font face.
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

    /// Create a font stretch from a lowercase name like `extra-expanded`.
    pub fn from_str(name: &str) -> Option<Self> {
        Some(match name {
            "ultra-condensed" => Self::ULTRA_CONDENSED,
            "extra-condensed" => Self::EXTRA_CONDENSED,
            "condensed" => Self::CONDENSED,
            "semi-condensed" => Self::SEMI_CONDENSED,
            "normal" => Self::NORMAL,
            "semi-expanded" => Self::SEMI_EXPANDED,
            "expanded" => Self::EXPANDED,
            "extra-expanded" => Self::EXTRA_EXPANDED,
            "ultra-expanded" => Self::ULTRA_EXPANDED,
            _ => return None,
        })
    }

    /// The ratio between 0.5 and 2.0 corresponding to this stretch.
    pub fn to_ratio(self) -> f32 {
        self.0 as f32 / 1000.0
    }

    /// The lowercase string representation of this stretch is one of the named
    /// ones.
    pub fn to_str(self) -> Option<&'static str> {
        Some(match self {
            Self::ULTRA_CONDENSED => "ultra-condensed",
            Self::EXTRA_CONDENSED => "extra-condensed",
            Self::CONDENSED => "condensed",
            Self::SEMI_CONDENSED => "semi-condensed",
            Self::NORMAL => "normal",
            Self::SEMI_EXPANDED => "semi-expanded",
            Self::EXPANDED => "expanded",
            Self::EXTRA_EXPANDED => "extra-expanded",
            Self::ULTRA_EXPANDED => "ultra-expanded",
            _ => return None,
        })
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

impl Display for FontStretch {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self.to_str() {
            Some(name) => f.pad(name),
            None => write!(f, "{}", self.to_ratio()),
        }
    }
}

impl Debug for FontStretch {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(match *self {
            s if s == Self::ULTRA_CONDENSED => "UltraCondensed",
            s if s == Self::EXTRA_CONDENSED => "ExtraCondensed",
            s if s == Self::CONDENSED => "Condensed",
            s if s == Self::SEMI_CONDENSED => "SemiCondensed",
            s if s == Self::NORMAL => "Normal",
            s if s == Self::SEMI_EXPANDED => "SemiExpanded",
            s if s == Self::EXPANDED => "Expanded",
            s if s == Self::EXTRA_EXPANDED => "ExtraExpanded",
            s if s == Self::ULTRA_EXPANDED => "UltraExpanded",
            _ => return write!(f, "{}", self.0),
        })
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
}
