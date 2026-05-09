use std::cmp::Reverse;
use std::collections::BTreeMap;
use std::fmt::{self, Debug, Formatter};
use std::ops::RangeInclusive;

use serde::{Deserialize, Serialize};
use ttf_parser::{PlatformId, Tag, name_id};
use unicode_segmentation::UnicodeSegmentation;

use super::InstanceParameters;
use super::exceptions::find_exception;
use super::variant::{Field, OpticalSizeAxis, SlantAxis, StaticField, VariableField};
use crate::text::{
    Font, FontStretch, FontStyle, FontVariant, FontVariantCoverage, FontWeight,
    is_default_ignorable,
};

/// A key that identifies a specific font instance.
///
/// For static fonts, this is just an index. For variable fonts, this also
/// includes the instance parameters (axis values) for the specific variant.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct FontKey {
    /// The index of the font in the font book.
    pub index: usize,
    /// The instance parameters for variable fonts.
    pub instance_params: InstanceParameters,
}

impl FontKey {
    /// Create a new font key with no instance parameters.
    pub fn new(index: usize) -> Self {
        Self { index, instance_params: InstanceParameters::new() }
    }

    /// Create a new font key with instance parameters.
    pub fn with_params(index: usize, instance_params: InstanceParameters) -> Self {
        Self { index, instance_params }
    }
}

/// Metadata about a collection of fonts.
#[derive(Debug, Default, Clone, Hash)]
pub struct FontBook {
    /// Maps from lowercased family names to font indices.
    families: BTreeMap<String, Vec<usize>>,
    /// Metadata about each font in the collection.
    infos: Vec<FontInfo>,
}

impl FontBook {
    /// Create a new, empty font book.
    pub fn new() -> Self {
        Self { families: BTreeMap::new(), infos: vec![] }
    }

    /// Create a font book from a collection of font infos.
    pub fn from_infos(infos: impl IntoIterator<Item = FontInfo>) -> Self {
        let mut book = Self::new();
        for info in infos {
            book.push(info);
        }
        book
    }

    /// Create a font book for a collection of fonts.
    pub fn from_fonts<'a>(fonts: impl IntoIterator<Item = &'a Font>) -> Self {
        Self::from_infos(fonts.into_iter().map(|font| font.info().clone()))
    }

    /// Insert metadata into the font book.
    pub fn push(&mut self, info: FontInfo) {
        let index = self.infos.len();
        let family = info.family.to_lowercase();
        self.families.entry(family).or_default().push(index);
        self.infos.push(info);
    }

    /// Get the font info for the given index.
    pub fn info(&self, index: usize) -> Option<&FontInfo> {
        self.infos.get(index)
    }

    /// Returns true if the book contains a font family with the given name.
    pub fn contains_family(&self, family: &str) -> bool {
        self.families.contains_key(family)
    }

    /// An ordered iterator over all font families this book knows and the
    /// font indices that belong to them.
    pub fn families(
        &self,
    ) -> impl Iterator<Item = (&str, impl Iterator<Item = usize>)> + '_ {
        // Since the keys are lowercased, we instead use the family field of the
        // first face's info.
        self.families.values().map(|ids| {
            let family = self.infos[ids[0]].family.as_str();
            (family, ids.iter().copied())
        })
    }

    /// Try to find a font from the given `family` that matches the given
    /// `variant` as closely as possible.
    ///
    /// The `family` should be all lowercase.
    ///
    /// For variable fonts, the returned `FontKey` includes the instance
    /// parameters needed to instantiate the font at the requested variant.
    ///
    /// If `optical_size` is provided (in points), variable fonts with an `opsz`
    /// axis will be instantiated at that optical size.
    ///
    /// If `custom_axes` is provided, those axes will be applied on top of the
    /// automatically-determined values, allowing user override of axis values.
    pub fn select(
        &self,
        family: &str,
        variant: FontVariant,
        optical_size: Option<f32>,
        custom_axes: Option<&[(ttf_parser::Tag, f32)]>,
    ) -> Option<FontKey> {
        let ids = self.families.get(family)?;
        self.find_best_variant(
            None,
            variant,
            optical_size,
            custom_axes,
            ids.iter().copied(),
        )
    }

    /// Iterate over all variants of a family.
    pub fn select_family(&self, family: &str) -> impl Iterator<Item = usize> + '_ {
        self.families
            .get(family)
            .map(|vec| vec.as_slice())
            .unwrap_or_default()
            .iter()
            .copied()
    }

    /// Try to find and load a fallback font that
    /// - is as close as possible to the font `like` (if any)
    /// - is as close as possible to the given `variant`
    /// - is suitable for shaping the given `text`
    ///
    /// If `optical_size` is provided (in points), variable fonts with an `opsz`
    /// axis will be instantiated at that optical size.
    ///
    /// If `custom_axes` is provided, those axes will be applied on top of the
    /// automatically-determined values, allowing user override of axis values.
    pub fn select_fallback(
        &self,
        like: Option<&FontInfo>,
        variant: FontVariant,
        text: &str,
        optical_size: Option<f32>,
        custom_axes: Option<&[(ttf_parser::Tag, f32)]>,
    ) -> Option<FontKey> {
        // Find the fonts that contain the text's first non-space and
        // non-ignorable char ...
        let c = text
            .chars()
            .find(|&c| !c.is_whitespace() && !is_default_ignorable(c))?;

        let ids = self
            .infos
            .iter()
            .enumerate()
            .filter(|(_, info)| info.coverage.contains(c as u32))
            .map(|(index, _)| index);

        // ... and find the best variant among them.
        self.find_best_variant(like, variant, optical_size, custom_axes, ids)
    }

    /// Find the font in the passed iterator that
    /// - is closest to the font `like` (if any)
    /// - is closest to the given `variant`
    ///
    /// To do that we compute a key for all variants and select the one with the
    /// minimal key. This key prioritizes:
    /// - If `like` is some other font:
    ///   - Are both fonts (not) monospaced?
    ///   - Do both fonts (not) have serifs?
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
    ///
    /// For variable fonts, if the requested value is within the font's range,
    /// the distance is 0, and instance parameters will be included in the key.
    ///
    /// If `custom_axes` is provided, those axes will be applied on top of the
    /// automatically-determined values after the font is selected.
    fn find_best_variant(
        &self,
        like: Option<&FontInfo>,
        variant: FontVariant,
        optical_size: Option<f32>,
        custom_axes: Option<&[(ttf_parser::Tag, f32)]>,
        ids: impl IntoIterator<Item = usize>,
    ) -> Option<FontKey> {
        let mut best = None;
        let mut best_key = None;

        for id in ids {
            let current = &self.infos[id];
            let (style_dist, stretch_dist, weight_dist) =
                current.variant_coverage.distance(&variant);
            let key = (
                like.map(|like| {
                    (
                        current.flags.contains(FontFlags::MONOSPACE)
                            != like.flags.contains(FontFlags::MONOSPACE),
                        current.flags.contains(FontFlags::SERIF)
                            != like.flags.contains(FontFlags::SERIF),
                        Reverse(shared_prefix_words(&current.family, &like.family)),
                        current.family.len(),
                    )
                }),
                style_dist,
                stretch_dist,
                weight_dist,
            );

            if best_key.is_none_or(|b| key < b) {
                best = Some(id);
                best_key = Some(key);
            }
        }

        // Build the FontKey with instance parameters if it's a variable font
        best.map(|id| {
            let info = &self.infos[id];
            let mut instance_params = InstanceParameters::new();

            // If this is a variable font, set the instance parameters
            if info.variant_coverage.is_variable() {
                // Set weight if the font has a variable weight axis
                // Clamp to the font's supported range
                if let Field::Variable(v) = &info.variant_coverage.weight {
                    let clamped_weight = clamp_to_range(&variant.weight, &v.range);
                    instance_params.set_weight(clamped_weight);
                }

                // Set stretch if the font has a variable stretch axis
                // Clamp to the font's supported range
                if let Field::Variable(v) = &info.variant_coverage.stretch {
                    let clamped_stretch = clamp_to_range(&variant.stretch, &v.range);
                    instance_params.set_stretch(clamped_stretch);
                }

                // Set slant/italic axis based on the requested style
                match &info.variant_coverage.slant_axis {
                    SlantAxis::Slnt { min, max, default } => {
                        // For slnt axis: negative values = italic/oblique (right-leaning)
                        // Use the minimum value for italic/oblique, default for normal
                        let slant_value = match variant.style {
                            FontStyle::Normal => *default as f32,
                            FontStyle::Italic | FontStyle::Oblique => {
                                // Use the most italic value (usually the minimum, which is negative)
                                // Clamp to the font's range
                                (*min).min(*max) as f32
                            }
                        };
                        instance_params.set_slant(slant_value);
                    }
                    SlantAxis::Ital { .. } => {
                        // For ital axis: 0 = upright, 1 = italic
                        let is_italic = matches!(
                            variant.style,
                            FontStyle::Italic | FontStyle::Oblique
                        );
                        instance_params.set_italic(is_italic);
                    }
                    SlantAxis::None => {}
                }

                // Set optical size axis based on the text size (in points)
                // This enables automatic optical sizing for variable fonts
                if let OpticalSizeAxis::Opsz { min, max, default } =
                    &info.variant_coverage.optical_size_axis
                {
                    // Use the provided optical size, or fall back to the font's default
                    let opsz_value = optical_size.unwrap_or(*default);
                    // Clamp to the font's supported range
                    let clamped_opsz = opsz_value.clamp(*min, *max);
                    instance_params.set_optical_size(clamped_opsz);
                }
            }

            // Apply any custom axes specified by the user.
            // These are applied last so they can override the automatically-determined values.
            if let Some(axes) = custom_axes {
                instance_params.apply_custom_axes(axes);
            }

            FontKey::with_params(id, instance_params)
        })
    }
}

/// Properties of a single font.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct FontInfo {
    /// The typographic font family this font is part of.
    pub family: String,
    /// Properties that distinguish this font from other fonts in the same
    /// family. For variable fonts, this includes axis ranges.
    pub variant_coverage: FontVariantCoverage,
    /// Properties of the font.
    pub flags: FontFlags,
    /// The unicode coverage of the font.
    pub coverage: Coverage,
}

impl FontInfo {
    /// Get the default variant for this font.
    ///
    /// For static fonts, this returns the fixed variant.
    /// For variable fonts, this returns the default values of the axes.
    pub fn variant(&self) -> FontVariant {
        self.variant_coverage.default_variant()
    }
}

bitflags::bitflags! {
    /// Bitflags describing characteristics of a font.
    #[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
    #[derive(Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct FontFlags: u32 {
        /// All glyphs have the same width.
        const MONOSPACE = 1 << 0;
        /// Glyphs have short strokes at their stems.
        const SERIF = 1 << 1;
        /// Font face has a MATH table
        const MATH = 1 << 2;
        /// Font face has an fvar table
        const VARIABLE = 1 << 3;
    }
}

impl FontInfo {
    /// Compute metadata for font at the `index` of the given data.
    pub fn new(data: &[u8], index: u32) -> Option<Self> {
        let ttf = ttf_parser::Face::parse(data, index).ok()?;
        Self::from_ttf(&ttf)
    }

    /// Compute metadata for all fonts in the given data.
    pub fn iter(data: &[u8]) -> impl Iterator<Item = FontInfo> + '_ {
        let count = ttf_parser::fonts_in_collection(data).unwrap_or(1);
        (0..count).filter_map(move |index| Self::new(data, index))
    }

    /// Compute metadata for a single ttf-parser face.
    pub(super) fn from_ttf(ttf: &ttf_parser::Face) -> Option<Self> {
        let ps_name = find_name(ttf, name_id::POST_SCRIPT_NAME);
        let exception = ps_name.as_deref().and_then(find_exception);
        // We cannot use Name ID 16 "Typographic Family", because for some
        // fonts it groups together more than just Style / Weight / Stretch
        // variants (e.g. Display variants of Noto fonts) and then some
        // variants become inaccessible from Typst. And even though the
        // fsSelection bit WWS should help us decide whether that is the
        // case, it's wrong for some fonts (e.g. for certain variants of "Noto
        // Sans Display").
        //
        // So, instead we use Name ID 1 "Family" and trim many common
        // suffixes for which know that they just describe styling (e.g.
        // "ExtraBold").
        let family =
            exception.and_then(|c| c.family.map(str::to_string)).or_else(|| {
                let family = find_name(ttf, name_id::FAMILY)?;
                Some(typographic_family(&family).to_string())
            })?;

        let variant_coverage = {
            let style = exception.and_then(|c| c.style).unwrap_or_else(|| {
                let mut full = find_name(ttf, name_id::FULL_NAME).unwrap_or_default();
                full.make_ascii_lowercase();

                // Some fonts miss the relevant bits for italic or oblique, so
                // we also try to infer that from the full name.
                //
                // We do not use `ttf.is_italic()` because that also checks the
                // italic angle which leads to false positives for some oblique
                // fonts.
                //
                // See <https://github.com/typst/typst/issues/7479>.
                let italic =
                    ttf.style() == ttf_parser::Style::Italic || full.contains("italic");
                let oblique = ttf.is_oblique()
                    || full.contains("oblique")
                    || full.contains("slanted");

                match (italic, oblique) {
                    (false, false) => FontStyle::Normal,
                    (true, _) => FontStyle::Italic,
                    (_, true) => FontStyle::Oblique,
                }
            });

            // Get weight from exception or font, then check for variable axis
            let base_weight = exception.and_then(|c| c.weight).unwrap_or_else(|| {
                let number = ttf.weight().to_number();
                FontWeight::from_number(number)
            });

            // Get stretch from exception or font, then check for variable axis
            let base_stretch = exception
                .and_then(|c| c.stretch)
                .unwrap_or_else(|| FontStretch::from_number(ttf.width().to_number()));

            // Build weight and stretch fields, checking for variable axes
            let mut weight = Field::Static(StaticField(base_weight));
            let mut stretch = Field::Static(StaticField(base_stretch));
            let mut slant_axis = SlantAxis::None;
            let mut optical_size_axis = OpticalSizeAxis::None;

            // Check for variable font axes
            if ttf.is_variable() {
                for axis in ttf.variation_axes() {
                    // wght axis (weight)
                    if axis.tag == Tag::from_bytes(b"wght") {
                        let min = FontWeight::from_number(axis.min_value.floor() as u16);
                        let max = FontWeight::from_number(axis.max_value.ceil() as u16);
                        let default =
                            FontWeight::from_number(axis.def_value.round() as u16);
                        weight =
                            Field::Variable(VariableField { range: min..=max, default });
                    }
                    // wdth axis (width/stretch)
                    // Note: OpenType wdth is in percentage (100 = normal)
                    // FontStretch stores as permille (1000 = normal)
                    if axis.tag == Tag::from_bytes(b"wdth") {
                        let min = FontStretch::from_ratio(crate::layout::Ratio::new(
                            axis.min_value as f64 / 100.0,
                        ));
                        let max = FontStretch::from_ratio(crate::layout::Ratio::new(
                            axis.max_value as f64 / 100.0,
                        ));
                        let default = FontStretch::from_ratio(crate::layout::Ratio::new(
                            axis.def_value as f64 / 100.0,
                        ));
                        stretch =
                            Field::Variable(VariableField { range: min..=max, default });
                    }
                    // slnt axis (slant) - continuous slant in degrees
                    // Negative values are right-leaning (italic/oblique)
                    if axis.tag == Tag::from_bytes(b"slnt") {
                        slant_axis = SlantAxis::Slnt {
                            min: axis.min_value.floor() as i16,
                            max: axis.max_value.ceil() as i16,
                            default: axis.def_value.round() as i16,
                        };
                    }
                    // ital axis (italic) - binary toggle (0 = upright, 1 = italic)
                    if axis.tag == Tag::from_bytes(b"ital") {
                        slant_axis =
                            SlantAxis::Ital { default_italic: axis.def_value > 0.5 };
                    }
                    // opsz axis (optical size) - continuous, typically in points
                    if axis.tag == Tag::from_bytes(b"opsz") {
                        optical_size_axis = OpticalSizeAxis::Opsz {
                            min: axis.min_value,
                            max: axis.max_value,
                            default: axis.def_value,
                        };
                    }
                }
            }

            FontVariantCoverage::with_axes(
                style,
                weight,
                stretch,
                slant_axis,
                optical_size_axis,
            )
        };

        // Determine the unicode coverage.
        let mut codepoints = vec![];
        for subtable in ttf.tables().cmap.into_iter().flat_map(|table| table.subtables) {
            if subtable.is_unicode() {
                subtable.codepoints(|c| codepoints.push(c));
            }
        }

        let mut flags = FontFlags::empty();
        flags.set(FontFlags::MONOSPACE, ttf.is_monospaced());
        flags.set(FontFlags::MATH, ttf.tables().math.is_some());
        flags.set(FontFlags::VARIABLE, ttf.is_variable());

        // Determine whether this is a serif or sans-serif font.
        if let Some(panose) = ttf
            .raw_face()
            .table(Tag::from_bytes(b"OS/2"))
            .and_then(|os2| os2.get(32..45))
            && matches!(panose, [2, 2..=10, ..])
        {
            flags.insert(FontFlags::SERIF);
        }

        Some(FontInfo {
            family,
            variant_coverage,
            flags,
            coverage: Coverage::from_vec(codepoints),
        })
    }

    /// Whether this is the macOS LastResort font. It can yield tofus with
    /// glyph ID != 0.
    pub fn is_last_resort(&self) -> bool {
        self.family == "LastResort"
    }
}

/// Try to find and decode the name with the given id.
pub(super) fn find_name(ttf: &ttf_parser::Face, name_id: u16) -> Option<String> {
    ttf.names().into_iter().find_map(|entry| {
        if entry.name_id == name_id {
            if let Some(string) = entry.to_string() {
                return Some(string);
            }

            if entry.platform_id == PlatformId::Macintosh && entry.encoding_id == 0 {
                return Some(decode_mac_roman(entry.name));
            }
        }

        None
    })
}

/// Decode mac roman encoded bytes into a string.
fn decode_mac_roman(coded: &[u8]) -> String {
    #[rustfmt::skip]
    const TABLE: [char; 128] = [
        'Ä', 'Å', 'Ç', 'É', 'Ñ', 'Ö', 'Ü', 'á', 'à', 'â', 'ä', 'ã', 'å', 'ç', 'é', 'è',
        'ê', 'ë', 'í', 'ì', 'î', 'ï', 'ñ', 'ó', 'ò', 'ô', 'ö', 'õ', 'ú', 'ù', 'û', 'ü',
        '†', '°', '¢', '£', '§', '•', '¶', 'ß', '®', '©', '™', '´', '¨', '≠', 'Æ', 'Ø',
        '∞', '±', '≤', '≥', '¥', 'µ', '∂', '∑', '∏', 'π', '∫', 'ª', 'º', 'Ω', 'æ', 'ø',
        '¿', '¡', '¬', '√', 'ƒ', '≈', '∆', '«', '»', '…', '\u{a0}', 'À', 'Ã', 'Õ', 'Œ', 'œ',
        '–', '—', '“', '”', '‘', '’', '÷', '◊', 'ÿ', 'Ÿ', '⁄', '€', '‹', '›', 'ﬁ', 'ﬂ',
        '‡', '·', '‚', '„', '‰', 'Â', 'Ê', 'Á', 'Ë', 'È', 'Í', 'Î', 'Ï', 'Ì', 'Ó', 'Ô',
        '\u{f8ff}', 'Ò', 'Ú', 'Û', 'Ù', 'ı', 'ˆ', '˜', '¯', '˘', '˙', '˚', '¸', '˝', '˛', 'ˇ',
    ];

    fn char_from_mac_roman(code: u8) -> char {
        if code < 128 { code as char } else { TABLE[(code - 128) as usize] }
    }

    coded.iter().copied().map(char_from_mac_roman).collect()
}

/// Trim style naming from a family name and fix bad names.
fn typographic_family(mut family: &str) -> &str {
    // Separators between names, modifiers and styles.
    const SEPARATORS: [char; 3] = [' ', '-', '_'];

    // Modifiers that can appear in combination with suffixes.
    const MODIFIERS: &[&str] =
        &["extra", "ext", "ex", "x", "semi", "sem", "sm", "demi", "dem", "ultra"];

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
        let mut t = trimmed;
        let mut shortened = false;
        while let Some(s) = SUFFIXES.iter().find_map(|s| t.strip_suffix(s)) {
            shortened = true;
            t = s;
        }

        if !shortened {
            break;
        }

        // Strip optional separator.
        if let Some(s) = t.strip_suffix(SEPARATORS) {
            trimmed = s;
            t = s;
        }

        // Also allow an extra modifier, but apply it only if it is separated it
        // from the text before it (to prevent false positives).
        if let Some(t) = MODIFIERS.iter().find_map(|s| t.strip_suffix(s))
            && let Some(stripped) = t.strip_suffix(SEPARATORS)
        {
            trimmed = stripped;
        }
    }

    // Apply style suffix trimming.
    family = &family[..len];

    family
}

/// How many words the two strings share in their prefix.
fn shared_prefix_words(left: &str, right: &str) -> usize {
    left.unicode_words()
        .zip(right.unicode_words())
        .take_while(|(l, r)| l == r)
        .count()
}

/// Clamp a value to the range, returning the boundary value if outside.
fn clamp_to_range<T: Ord + Copy>(value: &T, range: &RangeInclusive<T>) -> T {
    if value < range.start() {
        *range.start()
    } else if value > range.end() {
        *range.end()
    } else {
        *value
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
#[derive(Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
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
            if (cursor..cursor + run).contains(&c) {
                return inside;
            }
            cursor += run;
            inside = !inside;
        }

        false
    }

    /// Iterate over all covered codepoints.
    pub fn iter(&self) -> impl Iterator<Item = u32> + '_ {
        let mut inside = false;
        let mut cursor = 0;
        self.0.iter().flat_map(move |run| {
            let range = if inside { cursor..cursor + run } else { 0..0 };
            inside = !inside;
            cursor += run;
            range
        })
    }
}

impl Debug for Coverage {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad("Coverage(..)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trim_styles() {
        assert_eq!(typographic_family("Atma Light"), "Atma");
        assert_eq!(typographic_family("eras bold"), "eras");
        assert_eq!(typographic_family("footlight mt light"), "footlight mt");
        assert_eq!(typographic_family("times new roman"), "times new roman");
        assert_eq!(typographic_family("noto sans mono cond sembd"), "noto sans mono");
        assert_eq!(typographic_family("noto serif SEMCOND sembd"), "noto serif");
        assert_eq!(typographic_family("crimson text"), "crimson text");
        assert_eq!(typographic_family("footlight light"), "footlight");
        assert_eq!(typographic_family("Noto Sans"), "Noto Sans");
        assert_eq!(typographic_family("Noto Sans Light"), "Noto Sans");
        assert_eq!(typographic_family("Noto Sans Semicondensed Heavy"), "Noto Sans");
        assert_eq!(typographic_family("Familx"), "Familx");
        assert_eq!(typographic_family("Font Ultra"), "Font Ultra");
        assert_eq!(typographic_family("Font Ultra Bold"), "Font");
    }

    #[test]
    fn test_coverage() {
        #[track_caller]
        fn test(set: &[u32], runs: &[u32]) {
            let coverage = Coverage::from_vec(set.to_vec());
            assert_eq!(coverage.0, runs);

            let max = 5 + set.iter().copied().max().unwrap_or_default();
            for c in 0..max {
                assert_eq!(set.contains(&c), coverage.contains(c));
            }
        }

        test(&[], &[]);
        test(&[0], &[0, 1]);
        test(&[1], &[1, 1]);
        test(&[0, 1], &[0, 2]);
        test(&[0, 1, 3], &[0, 2, 1, 1]);
        test(
            // {2, 3, 4, 9, 10, 11, 15, 18, 19}
            &[18, 19, 2, 4, 9, 11, 15, 3, 3, 10],
            &[2, 3, 4, 3, 3, 1, 2, 2],
        )
    }

    #[test]
    fn test_coverage_iter() {
        let codepoints = vec![2, 3, 7, 8, 9, 14, 15, 19, 21];
        let coverage = Coverage::from_vec(codepoints.clone());
        assert_eq!(coverage.iter().collect::<Vec<_>>(), codepoints);
    }
}
