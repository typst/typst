use std::cmp::Reverse;
use std::collections::BTreeMap;
use std::fmt::{self, Debug, Formatter};

use serde::{Deserialize, Serialize};
use ttf_parser::{name_id, PlatformId, Tag};
use unicode_segmentation::UnicodeSegmentation;

use super::exceptions::find_exception;
use crate::text::{Font, FontStretch, FontStyle, FontVariant, FontWeight};

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

    /// An ordered iterator over all font families this book knows and details
    /// about the fonts that are part of them.
    pub fn families(
        &self,
    ) -> impl Iterator<Item = (&str, impl Iterator<Item = &FontInfo>)> + '_ {
        // Since the keys are lowercased, we instead use the family field of the
        // first face's info.
        self.families.values().map(|ids| {
            let family = self.infos[ids[0]].family.as_str();
            let infos = ids.iter().map(|&id| &self.infos[id]);
            (family, infos)
        })
    }

    /// Try to find a font from the given `family` that matches the given
    /// `variant` as closely as possible.
    ///
    /// The `family` should be all lowercase.
    pub fn select(&self, family: &str, variant: FontVariant) -> Option<usize> {
        let ids = self.families.get(family)?;
        self.find_best_variant(None, variant, ids.iter().copied())
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
    pub fn select_fallback(
        &self,
        like: Option<&FontInfo>,
        variant: FontVariant,
        text: &str,
    ) -> Option<usize> {
        // Find the fonts that contain the text's first non-space char ...
        let c = text.chars().find(|c| !c.is_whitespace())?;
        let ids = self
            .infos
            .iter()
            .enumerate()
            .filter(|(_, info)| info.coverage.contains(c as u32))
            .map(|(index, _)| index);

        // ... and find the best variant among them.
        self.find_best_variant(like, variant, ids)
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
    fn find_best_variant(
        &self,
        like: Option<&FontInfo>,
        variant: FontVariant,
        ids: impl IntoIterator<Item = usize>,
    ) -> Option<usize> {
        let mut best = None;
        let mut best_key = None;

        for id in ids {
            let current = &self.infos[id];
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
                current.variant.style.distance(variant.style),
                current.variant.stretch.distance(variant.stretch),
                current.variant.weight.distance(variant.weight),
            );

            if best_key.is_none_or(|b| key < b) {
                best = Some(id);
                best_key = Some(key);
            }
        }

        best
    }
}

/// Properties of a single font.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct FontInfo {
    /// The typographic font family this font is part of.
    pub family: String,
    /// Properties that distinguish this font from other fonts in the same
    /// family.
    pub variant: FontVariant,
    /// Properties of the font.
    pub flags: FontFlags,
    /// The unicode coverage of the font.
    pub coverage: Coverage,
}

bitflags::bitflags! {
    /// Bitflags describing characteristics of a font.
    #[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
    #[derive(Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct FontFlags: u32 {
        /// All glyphs have the same width.
        const MONOSPACE = 1 << 0;
        /// Glyphs have short strokes at their stems.
        const SERIF = 1 << 1;
        /// Font face has a MATH table
        const MATH = 1 << 2;
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

        let variant = {
            let style = exception.and_then(|c| c.style).unwrap_or_else(|| {
                let mut full = find_name(ttf, name_id::FULL_NAME).unwrap_or_default();
                full.make_ascii_lowercase();

                // Some fonts miss the relevant bits for italic or oblique, so
                // we also try to infer that from the full name.
                let italic = ttf.is_italic() || full.contains("italic");
                let oblique = ttf.is_oblique()
                    || full.contains("oblique")
                    || full.contains("slanted");

                match (italic, oblique) {
                    (false, false) => FontStyle::Normal,
                    (true, _) => FontStyle::Italic,
                    (_, true) => FontStyle::Oblique,
                }
            });

            let weight = exception.and_then(|c| c.weight).unwrap_or_else(|| {
                let number = ttf.weight().to_number();
                FontWeight::from_number(number)
            });

            let stretch = exception
                .and_then(|c| c.stretch)
                .unwrap_or_else(|| FontStretch::from_number(ttf.width().to_number()));

            FontVariant { style, weight, stretch }
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

        // Determine whether this is a serif or sans-serif font.
        if let Some(panose) = ttf
            .raw_face()
            .table(Tag::from_bytes(b"OS/2"))
            .and_then(|os2| os2.get(32..45))
        {
            if matches!(panose, [2, 2..=10, ..]) {
                flags.insert(FontFlags::SERIF);
            }
        }

        Some(FontInfo {
            family,
            variant,
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
        if code < 128 {
            code as char
        } else {
            TABLE[(code - 128) as usize]
        }
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
        if let Some(t) = MODIFIERS.iter().find_map(|s| t.strip_suffix(s)) {
            if let Some(stripped) = t.strip_suffix(SEPARATORS) {
                trimmed = stripped;
            }
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
