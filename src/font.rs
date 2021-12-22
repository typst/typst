//! Font handling.

use std::collections::{hash_map::Entry, BTreeMap, HashMap};
use std::fmt::{self, Debug, Formatter};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use serde::{Deserialize, Serialize};
use ttf_parser::{name_id, GlyphId, PlatformId};

use crate::geom::{Em, Length, Linear};
use crate::loading::{FileHash, Loader};
use crate::util::decode_mac_roman;

/// A unique identifier for a loaded font face.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct FaceId(u32);

impl FaceId {
    /// Create a face id from the raw underlying value.
    ///
    /// This should only be called with values returned by
    /// [`into_raw`](Self::into_raw).
    pub const fn from_raw(v: u32) -> Self {
        Self(v)
    }

    /// Convert into the raw underlying value.
    pub const fn into_raw(self) -> u32 {
        self.0
    }
}

/// Storage for loaded and parsed font faces.
pub struct FontStore {
    loader: Rc<dyn Loader>,
    faces: Vec<Option<Face>>,
    families: BTreeMap<String, Vec<FaceId>>,
    buffers: HashMap<FileHash, Rc<Vec<u8>>>,
    on_load: Option<Box<dyn Fn(FaceId, &Face)>>,
}

impl FontStore {
    /// Create a new, empty font store.
    pub fn new(loader: Rc<dyn Loader>) -> Self {
        let mut faces = vec![];
        let mut families = BTreeMap::<String, Vec<FaceId>>::new();

        for (i, info) in loader.faces().iter().enumerate() {
            let id = FaceId(i as u32);
            faces.push(None);
            families.entry(info.family.to_lowercase()).or_default().push(id);
        }

        Self {
            loader,
            faces,
            families,
            buffers: HashMap::new(),
            on_load: None,
        }
    }

    /// Register a callback which is invoked each time a font face is loaded.
    pub fn on_load<F>(&mut self, f: F)
    where
        F: Fn(FaceId, &Face) + 'static,
    {
        self.on_load = Some(Box::new(f));
    }

    /// Query for and load the font face from the given `family` that most
    /// closely matches the given `variant`.
    pub fn select(&mut self, family: &str, variant: FontVariant) -> Option<FaceId> {
        // Check whether a family with this name exists.
        let ids = self.families.get(family)?;
        let infos = self.loader.faces();

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

        let id = best?;

        // Load the face if it's not already loaded.
        let idx = id.0 as usize;
        let slot = &mut self.faces[idx];
        if slot.is_none() {
            let FaceInfo { ref path, index, .. } = infos[idx];

            // Check the buffer cache since multiple faces may
            // refer to the same data (font collection).
            let hash = self.loader.resolve(path).ok()?;
            let buffer = match self.buffers.entry(hash) {
                Entry::Occupied(entry) => entry.into_mut(),
                Entry::Vacant(entry) => {
                    let buffer = self.loader.load(path).ok()?;
                    entry.insert(Rc::new(buffer))
                }
            };

            let face = Face::new(Rc::clone(buffer), index)?;
            if let Some(callback) = &self.on_load {
                callback(id, &face);
            }

            *slot = Some(face);
        }

        Some(id)
    }

    /// Get a reference to a loaded face.
    ///
    /// This panics if no face with this `id` was loaded. This function should
    /// only be called with ids returned by this store's
    /// [`select()`](Self::select) method.
    #[track_caller]
    pub fn get(&self, id: FaceId) -> &Face {
        self.faces[id.0 as usize].as_ref().expect("font face was not loaded")
    }

    /// Returns an ordered iterator over all font family names this loader
    /// knows.
    pub fn families(&self) -> impl Iterator<Item = &str> + '_ {
        // Since the keys are lowercased, we instead use the family field of the
        // first face's info.
        let faces = self.loader.faces();
        self.families
            .values()
            .map(move |id| faces[id[0].0 as usize].family.as_str())
    }
}

/// A font face.
pub struct Face {
    /// The raw face data, possibly shared with other faces from the same
    /// collection. Must stay alive put, because `ttf` points into it using
    /// unsafe code.
    buffer: Rc<Vec<u8>>,
    /// The face's index in the collection (zero if not a collection).
    index: u32,
    /// The underlying ttf-parser/rustybuzz face.
    ttf: rustybuzz::Face<'static>,
    /// How many font units represent one em unit.
    pub units_per_em: f64,
    /// The distance from the baseline to the typographic ascender.
    pub ascender: Em,
    /// The approximate height of uppercase letters.
    pub cap_height: Em,
    /// The approximate height of non-ascending lowercase letters.
    pub x_height: Em,
    /// The distance from the baseline to the typographic descender.
    pub descender: Em,
    /// Recommended metrics for a strikethrough line.
    pub strikethrough: LineMetrics,
    /// Recommended metrics for an underline.
    pub underline: LineMetrics,
    /// Recommended metrics for an overline.
    pub overline: LineMetrics,
}

/// Metrics for a decorative line.
#[derive(Debug, Copy, Clone)]
pub struct LineMetrics {
    /// The vertical offset of the line from the baseline. Positive goes
    /// upwards, negative downwards.
    pub position: Em,
    /// The thickness of the line.
    pub thickness: Em,
}

impl Face {
    /// Parse a font face from a buffer and collection index.
    pub fn new(buffer: Rc<Vec<u8>>, index: u32) -> Option<Self> {
        // Safety:
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

        let strikethrough = LineMetrics {
            position: strikeout.map_or(Em::new(0.25), |s| to_em(s.position)),
            thickness: strikeout
                .or(underline)
                .map_or(Em::new(0.06), |s| to_em(s.thickness)),
        };

        let underline = LineMetrics {
            position: underline.map_or(Em::new(-0.2), |s| to_em(s.position)),
            thickness: underline
                .or(strikeout)
                .map_or(Em::new(0.06), |s| to_em(s.thickness)),
        };

        let overline = LineMetrics {
            position: cap_height + Em::new(0.1),
            thickness: underline.thickness,
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

    /// Convert from font units to an em length.
    pub fn to_em(&self, units: impl Into<f64>) -> Em {
        Em::from_units(units, self.units_per_em)
    }

    /// Look up the horizontal advance width of a glyph.
    pub fn advance(&self, glyph: u16) -> Option<Em> {
        self.ttf
            .glyph_hor_advance(GlyphId(glyph))
            .map(|units| self.to_em(units))
    }

    /// Look up a vertical metric at the given font size.
    pub fn vertical_metric(&self, metric: VerticalFontMetric, size: Length) -> Length {
        match metric {
            VerticalFontMetric::Ascender => self.ascender.resolve(size),
            VerticalFontMetric::CapHeight => self.cap_height.resolve(size),
            VerticalFontMetric::XHeight => self.x_height.resolve(size),
            VerticalFontMetric::Baseline => Length::zero(),
            VerticalFontMetric::Descender => self.descender.resolve(size),
            VerticalFontMetric::Linear(v) => v.resolve(size),
        }
    }
}

/// Identifies a vertical metric of a font.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
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
    /// An font-size dependent distance from the baseline (positive goes up, negative
    /// down).
    Linear(Linear),
}

/// Properties of a single font face.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct FaceInfo {
    /// The path to the font file.
    pub path: PathBuf,
    /// The collection index in the font file.
    pub index: u32,
    /// The typographic font family this face is part of.
    pub family: String,
    /// Properties that distinguish this face from other faces in the same
    /// family.
    #[serde(flatten)]
    pub variant: FontVariant,
}

impl FaceInfo {
    /// Determine metadata about all faces that are found in the given data.
    pub fn parse<'a>(
        path: &'a Path,
        data: &'a [u8],
    ) -> impl Iterator<Item = FaceInfo> + 'a {
        let count = ttf_parser::fonts_in_collection(data).unwrap_or(1);
        (0 .. count).filter_map(move |index| {
            let face = ttf_parser::Face::from_slice(data, index).ok()?;
            let mut family = find_name(face.names(), name_id::TYPOGRAPHIC_FAMILY)
                .or_else(|| find_name(face.names(), name_id::FAMILY))?;

            // Remove weird leading dot appearing in some fonts.
            if let Some(undotted) = family.strip_prefix('.') {
                family = undotted.to_string();
            }

            let variant = FontVariant {
                style: match (face.is_italic(), face.is_oblique()) {
                    (false, false) => FontStyle::Normal,
                    (true, _) => FontStyle::Italic,
                    (_, true) => FontStyle::Oblique,
                },
                weight: FontWeight::from_number(face.weight().to_number()),
                stretch: FontStretch::from_number(face.width().to_number()),
            };

            Some(FaceInfo {
                path: path.to_owned(),
                index,
                family,
                variant,
            })
        })
    }
}

/// Find a decodable entry in a name table iterator.
pub fn find_name(mut names: ttf_parser::Names<'_>, name_id: u16) -> Option<String> {
    names.find_map(|entry| {
        if entry.name_id() == name_id {
            if let Some(string) = entry.to_string() {
                return Some(string);
            }

            if entry.platform_id() == PlatformId::Macintosh && entry.encoding_id() == 0 {
                return Some(decode_mac_roman(entry.name()));
            }
        }

        None
    })
}

/// Properties that distinguish a face from other faces in the same family.
#[derive(Default, Copy, Clone, Eq, PartialEq, Hash)]
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

impl Debug for FontVariant {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}-{:?}-{:?}", self.style, self.weight, self.stretch)
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

impl Default for FontStyle {
    fn default() -> Self {
        Self::Normal
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
