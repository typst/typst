//! Font handling.

use std::cmp::Reverse;
use std::collections::{hash_map::Entry, BTreeMap, HashMap};
use std::fmt::{self, Debug, Formatter};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use ttf_parser::{name_id, GlyphId, PlatformId, Tag};
use unicode_segmentation::UnicodeSegmentation;

use crate::geom::Em;
use crate::loading::{FileHash, Loader};
use crate::util::decode_mac_roman;

/// A unique identifier for a loaded font face.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
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
    loader: Arc<dyn Loader>,
    failed: Vec<bool>,
    faces: Vec<Option<Face>>,
    families: BTreeMap<String, Vec<FaceId>>,
    buffers: HashMap<FileHash, Arc<Vec<u8>>>,
}

impl FontStore {
    /// Create a new, empty font store.
    pub fn new(loader: Arc<dyn Loader>) -> Self {
        let mut faces = vec![];
        let mut failed = vec![];
        let mut families = BTreeMap::<String, Vec<FaceId>>::new();

        let infos = loader.faces();
        for (i, info) in infos.iter().enumerate() {
            let id = FaceId(i as u32);
            faces.push(None);
            failed.push(false);
            families.entry(info.family.to_lowercase()).or_default().push(id);
        }

        for faces in families.values_mut() {
            faces.sort_by_key(|id| infos[id.0 as usize].variant);
            faces.dedup_by_key(|id| infos[id.0 as usize].variant);
        }

        Self {
            loader,
            faces,
            failed,
            families,
            buffers: HashMap::new(),
        }
    }

    /// Try to find and load a font face from the given `family` that matches
    /// the given `variant` as closely as possible.
    pub fn select(&mut self, family: &str, variant: FontVariant) -> Option<FaceId> {
        let ids = self.families.get(family)?;
        let id = self.find_best_variant(None, variant, ids.iter().copied())?;
        self.load(id)
    }

    /// Try to find and load a fallback font that
    /// - is as close as possible to the face `like` (if any)
    /// - is as close as possible to the given `variant`
    /// - is suitable for shaping the given `text`
    pub fn select_fallback(
        &mut self,
        like: Option<FaceId>,
        variant: FontVariant,
        text: &str,
    ) -> Option<FaceId> {
        // Find the faces that contain the text's first char ...
        let c = text.chars().next()?;
        let ids = self
            .loader
            .faces()
            .iter()
            .enumerate()
            .filter(|(_, info)| info.coverage.contains(c as u32))
            .map(|(i, _)| FaceId(i as u32));

        // ... and find the best variant among them.
        let id = self.find_best_variant(like, variant, ids)?;
        self.load(id)
    }

    /// Find the face in the passed iterator that
    /// - is closest to the face `like` (if any)
    /// - is closest to the given `variant`
    ///
    /// To do that we compute a key for all variants and select the one with the
    /// minimal key. This key prioritizes:
    /// - If `like` is some other face:
    ///   - Are both faces (not) monospaced.
    ///   - Do both faces (not) have serifs.
    ///   - How many words do the families share in their prefix? E.g. "Noto
    ///     Sans" and "Noto Sans Arabic" share two words, whereas "IBM Plex
    ///     Arabic" shares none with "Noto Sans", so prefer "Noto Sans Arabic"
    ///     if `like` is "Noto Sans". In case there are two equally good
    ///     matches, we prefer the shorter one because it is less special (e.g.
    ///     if `like` is "Noto Sans Arabic", we prefer "Noto Sans" over "Noto
    ///     Sans CJK HK".)
    /// - The style (normal / italic / oblique). If we want italic or oblique
    ///   but it doesn't exist, the other one of the two is still better than
    ///   normal.
    /// - The absolute distance to the target stretch.
    /// - The absolute distance to the target weight.
    fn find_best_variant(
        &self,
        like: Option<FaceId>,
        variant: FontVariant,
        ids: impl IntoIterator<Item = FaceId>,
    ) -> Option<FaceId> {
        let infos = self.loader.faces();
        let like = like.map(|id| &infos[id.0 as usize]);

        let mut best = None;
        let mut best_key = None;

        // Find the best matching variant of this font.
        for id in ids {
            let current = &infos[id.0 as usize];

            let key = (
                like.map(|like| {
                    (
                        current.monospaced != like.monospaced,
                        like.serif.is_some() && current.serif != like.serif,
                        Reverse(shared_prefix_words(&current.family, &like.family)),
                        current.family.len(),
                    )
                }),
                current.variant.style.distance(variant.style),
                current.variant.stretch.distance(variant.stretch),
                current.variant.weight.distance(variant.weight),
            );

            if best_key.map_or(true, |b| key < b) {
                best = Some(id);
                best_key = Some(key);
            }
        }

        best
    }

    /// Load the face with the given id.
    ///
    /// Returns `Some(id)` if the face was loaded successfully.
    fn load(&mut self, id: FaceId) -> Option<FaceId> {
        let idx = id.0 as usize;
        let slot = &mut self.faces[idx];
        if slot.is_some() {
            return Some(id);
        }

        if self.failed[idx] {
            return None;
        }

        let FaceInfo { ref path, index, .. } = self.loader.faces()[idx];
        self.failed[idx] = true;

        // Check the buffer cache since multiple faces may
        // refer to the same data (font collection).
        let hash = self.loader.resolve(path).ok()?;
        let buffer = match self.buffers.entry(hash) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let buffer = self.loader.load(path).ok()?;
                entry.insert(Arc::new(buffer))
            }
        };

        let face = Face::new(Arc::clone(buffer), index)?;
        *slot = Some(face);
        self.failed[idx] = false;

        Some(id)
    }

    /// Get a reference to a loaded face.
    ///
    /// This panics if the face with this `id` was not loaded. This function
    /// should only be called with ids returned by this store's
    /// [`select()`](Self::select) and
    /// [`select_fallback()`](Self::select_fallback) methods.
    #[track_caller]
    pub fn get(&self, id: FaceId) -> &Face {
        self.faces[id.0 as usize].as_ref().expect("font face was not loaded")
    }

    /// An ordered iterator over all font families this loader knows and details
    /// about the faces that are part of them.
    pub fn families(
        &self,
    ) -> impl Iterator<Item = (&str, impl Iterator<Item = &FaceInfo>)> + '_ {
        // Since the keys are lowercased, we instead use the family field of the
        // first face's info.
        let faces = self.loader.faces();
        self.families.values().map(|ids| {
            let family = faces[ids[0].0 as usize].family.as_str();
            let infos = ids.iter().map(|&id| &faces[id.0 as usize]);
            (family, infos)
        })
    }
}

/// How many words the two strings share in their prefix.
fn shared_prefix_words(left: &str, right: &str) -> usize {
    left.unicode_words()
        .zip(right.unicode_words())
        .take_while(|(l, r)| l == r)
        .count()
}

/// A font face.
pub struct Face {
    /// The raw face data, possibly shared with other faces from the same
    /// collection. Must stay alive put, because `ttf` points into it using
    /// unsafe code.
    buffer: Arc<Vec<u8>>,
    /// The face's index in the collection (zero if not a collection).
    index: u32,
    /// The underlying ttf-parser/rustybuzz face.
    ttf: rustybuzz::Face<'static>,
    /// The faces metrics.
    metrics: FaceMetrics,
}

impl Face {
    /// Parse a font face from a buffer and collection index.
    pub fn new(buffer: Arc<Vec<u8>>, index: u32) -> Option<Self> {
        // Safety:
        // - The slices's location is stable in memory:
        //   - We don't move the underlying vector
        //   - Nobody else can move it since we have a strong ref to the `Arc`.
        // - The internal static lifetime is not leaked because its rewritten
        //   to the self-lifetime in `ttf()`.
        let slice: &'static [u8] =
            unsafe { std::slice::from_raw_parts(buffer.as_ptr(), buffer.len()) };

        let ttf = rustybuzz::Face::from_slice(slice, index)?;
        let metrics = FaceMetrics::from_ttf(&ttf);

        Some(Self { buffer, index, ttf, metrics })
    }

    /// The underlying buffer.
    pub fn buffer(&self) -> &Arc<Vec<u8>> {
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

    /// The number of units per em.
    pub fn units_per_em(&self) -> f64 {
        self.metrics.units_per_em
    }

    /// Access the face's metrics.
    pub fn metrics(&self) -> &FaceMetrics {
        &self.metrics
    }

    /// Convert from font units to an em length.
    pub fn to_em(&self, units: impl Into<f64>) -> Em {
        Em::from_units(units, self.units_per_em())
    }

    /// Look up the horizontal advance width of a glyph.
    pub fn advance(&self, glyph: u16) -> Option<Em> {
        self.ttf
            .glyph_hor_advance(GlyphId(glyph))
            .map(|units| self.to_em(units))
    }
}

/// Metrics for a font face.
#[derive(Debug, Copy, Clone)]
pub struct FaceMetrics {
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

impl FaceMetrics {
    /// Extract the face's metrics.
    pub fn from_ttf(ttf: &ttf_parser::Face) -> Self {
        let units_per_em = f64::from(ttf.units_per_em().unwrap_or(0));
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

        Self {
            units_per_em,
            ascender,
            cap_height,
            x_height,
            descender,
            strikethrough,
            underline,
            overline,
        }
    }

    /// Look up a vertical metric at the given font size.
    pub fn vertical(&self, metric: VerticalFontMetric) -> Em {
        match metric {
            VerticalFontMetric::Ascender => self.ascender,
            VerticalFontMetric::CapHeight => self.cap_height,
            VerticalFontMetric::XHeight => self.x_height,
            VerticalFontMetric::Baseline => Em::zero(),
            VerticalFontMetric::Descender => self.descender,
        }
    }
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
    pub variant: FontVariant,
    /// Whether the face is monospaced.
    pub monospaced: bool,
    /// Whether the face has serifs (if known).
    pub serif: Option<bool>,
    /// The unicode coverage of the face.
    pub coverage: Coverage,
}

impl FaceInfo {
    /// Compute metadata for all faces in the given data.
    pub fn from_data<'a>(
        path: &'a Path,
        data: &'a [u8],
    ) -> impl Iterator<Item = FaceInfo> + 'a {
        let count = ttf_parser::fonts_in_collection(data).unwrap_or(1);
        (0 .. count).filter_map(move |index| {
            let face = ttf_parser::Face::from_slice(data, index).ok()?;
            Self::from_ttf(path, index, &face)
        })
    }

    /// Compute metadata for a single ttf-parser face.
    pub fn from_ttf(path: &Path, index: u32, ttf: &ttf_parser::Face) -> Option<Self> {
        // We cannot use Name ID 16 "Typographic Family", because for some
        // fonts it groups together more than just Style / Weight / Stretch
        // variants (e.g. Display variants of Noto fonts) and then some
        // variants become inaccessible from Typst. And even though the
        // fsSelection bit WWS should help us decide whether that is the
        // case, it's wrong for some fonts (e.g. for some faces of "Noto
        // Sans Display").
        //
        // So, instead we use Name ID 1 "Family" and trim many common
        // suffixes for which know that they just describe styling (e.g.
        // "ExtraBold").
        //
        // Also, for Noto fonts we use Name ID 4 "Full Name" instead,
        // because Name ID 1 "Family" sometimes contains "Display" and
        // sometimes doesn't for the Display variants and that mixes things
        // up.
        let family = {
            let mut family = find_name(ttf, name_id::FAMILY)?;
            if family.starts_with("Noto") {
                family = find_name(ttf, name_id::FULL_NAME)?;
            }
            trim_styles(&family).to_string()
        };

        let variant = {
            let mut full = find_name(ttf, name_id::FULL_NAME).unwrap_or_default();
            full.make_ascii_lowercase();

            // Some fonts miss the relevant bits for italic or oblique, so
            // we also try to infer that from the full name.
            let italic = ttf.is_italic() || full.contains("italic");
            let oblique =
                ttf.is_oblique() || full.contains("oblique") || full.contains("slanted");

            let style = match (italic, oblique) {
                (false, false) => FontStyle::Normal,
                (true, _) => FontStyle::Italic,
                (_, true) => FontStyle::Oblique,
            };

            let weight = FontWeight::from_number(ttf.weight().to_number());
            let stretch = FontStretch::from_number(ttf.width().to_number());

            FontVariant { style, weight, stretch }
        };

        // Determine the unicode coverage.
        let mut codepoints = vec![];
        for subtable in ttf.character_mapping_subtables() {
            if subtable.is_unicode() {
                subtable.codepoints(|c| codepoints.push(c));
            }
        }

        // Determine whether this is a serif or sans-serif font.
        let mut serif = None;
        if let Some(panose) = ttf
            .table_data(Tag::from_bytes(b"OS/2"))
            .and_then(|os2| os2.get(32 .. 45))
        {
            match panose {
                [2, 2 ..= 10, ..] => serif = Some(true),
                [2, 11 ..= 15, ..] => serif = Some(false),
                _ => {}
            }
        }

        Some(FaceInfo {
            path: path.to_owned(),
            index,
            family,
            variant,
            monospaced: ttf.is_monospaced(),
            serif,
            coverage: Coverage::from_vec(codepoints),
        })
    }
}

/// Try to find and decode the name with the given id.
pub fn find_name(ttf: &ttf_parser::Face, name_id: u16) -> Option<String> {
    ttf.names().find_map(|entry| {
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

/// Trim style naming from a family name.
fn trim_styles(mut family: &str) -> &str {
    // Separators between names, modifiers and styles.
    const SEPARATORS: [char; 3] = [' ', '-', '_'];

    // Modifiers that can appear in combination with suffixes.
    const MODIFIERS: &[&str] = &[
        "extra", "ext", "ex", "x", "semi", "sem", "sm", "demi", "dem", "ultra",
    ];

    // Style suffixes.
    #[rustfmt::skip]
    const SUFFIXES: &[&str] = &[
        "normal", "italic", "oblique", "slanted",
        "thin", "th", "hairline", "light", "lt", "regular", "medium", "med",
        "md", "bold", "bd", "demi", "extb", "black", "blk", "bk", "heavy",
        "narrow", "condensed", "cond", "cn", "cd", "compressed", "expanded", "exp"
    ];

    // Trim spacing and weird leading dots in Apple fonts.
    family = family.trim().trim_start_matches('.');

    // Lowercase the string so that the suffixes match case-insensitively.
    let lower = family.to_ascii_lowercase();
    let mut len = usize::MAX;
    let mut trimmed = lower.as_str();

    // Trim style suffixes repeatedly.
    while trimmed.len() < len {
        len = trimmed.len();

        // Find style suffix.
        let mut t = match SUFFIXES.iter().find_map(|s| trimmed.strip_suffix(s)) {
            Some(t) => t,
            None => break,
        };

        // Strip optional separator.
        if let Some(s) = t.strip_suffix(SEPARATORS) {
            trimmed = s;
            t = s;
        }

        // Also allow an extra modifier, but apply it only if it is separated it
        // from the text before it (to prevent false positives).
        if let Some(t) = MODIFIERS.iter().find_map(|s| t.strip_suffix(s)) {
            if let Some(stripped) = t.strip_suffix(SEPARATORS) {
                trimmed = stripped;
            }
        }
    }

    &family[.. len]
}

/// Properties that distinguish a face from other faces in the same family.
#[derive(Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
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

/// A compactly encoded set of codepoints.
///
/// The set is represented by alternating specifications of how many codepoints
/// are not in the set and how many are in the set.
///
/// For example, for the set `{2, 3, 4, 9, 10, 11, 15, 18, 19}`, there are:
/// - 2 codepoints not inside (0, 1)
/// - 3 codepoints inside (2, 3, 4)
/// - 4 codepoints not inside (5, 6, 7, 8)
/// - 3 codepoints inside (9, 10, 11)
/// - 3 codepoints not inside (12, 13, 14)
/// - 1 codepoint inside (15)
/// - 2 codepoints not inside (16, 17)
/// - 2 codepoints inside (18, 19)
///
/// So the resulting encoding is `[2, 3, 4, 3, 3, 1, 2, 2]`.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Coverage(Vec<u32>);

impl Coverage {
    /// Encode a vector of codepoints.
    pub fn from_vec(mut codepoints: Vec<u32>) -> Self {
        codepoints.sort();
        codepoints.dedup();

        let mut runs = Vec::new();
        let mut next = 0;

        for c in codepoints {
            if let Some(run) = runs.last_mut().filter(|_| c == next) {
                *run += 1;
            } else {
                runs.push(c - next);
                runs.push(1);
            }

            next = c + 1;
        }

        Self(runs)
    }

    /// Whether the codepoint is covered.
    pub fn contains(&self, c: u32) -> bool {
        let mut inside = false;
        let mut cursor = 0;

        for &run in &self.0 {
            if (cursor .. cursor + run).contains(&c) {
                return inside;
            }
            cursor += run;
            inside = !inside;
        }

        false
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

    #[test]
    fn test_trim_styles() {
        assert_eq!(trim_styles("Atma Light"), "Atma");
        assert_eq!(trim_styles("eras bold"), "eras");
        assert_eq!(trim_styles("footlight mt light"), "footlight mt");
        assert_eq!(trim_styles("times new roman"), "times new roman");
        assert_eq!(trim_styles("noto sans mono cond sembd"), "noto sans mono");
        assert_eq!(trim_styles("noto serif SEMCOND sembd"), "noto serif");
        assert_eq!(trim_styles("crimson text"), "crimson text");
        assert_eq!(trim_styles("footlight light"), "footlight");
        assert_eq!(trim_styles("Noto Sans"), "Noto Sans");
        assert_eq!(trim_styles("Noto Sans Light"), "Noto Sans");
        assert_eq!(trim_styles("Noto Sans Semicondensed Heavy"), "Noto Sans");
        assert_eq!(trim_styles("Familx"), "Familx");
        assert_eq!(trim_styles("Font Ultra"), "Font Ultra");
        assert_eq!(trim_styles("Font Ultra Bold"), "Font");
    }

    #[test]
    fn test_coverage() {
        #[track_caller]
        fn test(set: &[u32], runs: &[u32]) {
            let coverage = Coverage::from_vec(set.to_vec());
            assert_eq!(coverage.0, runs);

            let max = 5 + set.iter().copied().max().unwrap_or_default();
            for c in 0 .. max {
                assert_eq!(set.contains(&c), coverage.contains(c));
            }
        }

        test(&[], &[]);
        test(&[0], &[0, 1]);
        test(&[1], &[1, 1]);
        test(&[0, 1], &[0, 2]);
        test(&[0, 1, 3], &[0, 2, 1, 1]);
        test(
            // [2, 3, 4, 9, 10, 11, 15, 18, 19]
            &[18, 19, 2, 4, 9, 11, 15, 3, 3, 10],
            &[2, 3, 4, 3, 3, 1, 2, 2],
        )
    }
}
