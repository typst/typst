use std::cmp::Reverse;
use std::collections::BTreeMap;

use unicode_segmentation::UnicodeSegmentation;

use super::{Font, FontInfo, FontVariant};
use crate::text::{FontFlags, is_default_ignorable};

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
        self.find_best_variant(like, variant, ids)
    }

    /// Find the font in the passed iterator that
    /// - is closest to the font `like` (if any), and
    /// - is closest to the given `variant`
    ///
    /// To do that we compute a score for all variants and select the one with the
    /// higher score. This score prioritizes:
    /// - If `like` is some other font:
    ///   - Are both fonts monospaced?
    ///   - Do both fonts have serifs?
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
        let mut best_score = None;

        for id in ids {
            let current = &self.infos[id];
            let score = (
                like.map(|like| similarity(current, like)),
                Reverse(distance(current, variant)),
            );

            if best_score.is_none_or(|b| score > b) {
                best = Some(id);
                best_score = Some(score);
            }
        }

        best
    }
}

/// Determines a metric that scores higher if `other` is similar to `self`.
/// This is used to pick a closely matching face during font fallback.
fn similarity(left: &FontInfo, right: &FontInfo) -> impl Ord + Copy {
    (
        // Most importantly, we want a font of a similar kind (monospace,
        // serif, etc.).
        left.flags.contains(FontFlags::MONOSPACE)
            == right.flags.contains(FontFlags::MONOSPACE),
        left.flags.contains(FontFlags::SERIF) == right.flags.contains(FontFlags::SERIF),
        // We prefer fonts that have more words shared in their name. E.g.
        // "Noto Sans" and "Noto Sans Arabic" share two words, whereas "IBM
        // Plex Arabic" shares none with "Noto Sans", so prefer "Noto Sans
        // Arabic" if `like` is "Noto Sans".
        shared_prefix_words(&left.family, &right.family),
        // In case there are two equally good matches, we prefer the shorter
        // one because it is less special (e.g. if `like` is "Noto Sans
        // Arabic", we prefer "Noto Sans" over "Noto Sans CJK HK".)
        Reverse(left.family.len()),
    )
}

/// Determines a distance metric from the given variant to
/// - this font's variant (if static)
/// - this font's closest instance (if variable)
///
/// Used to pick the most suitable font in a family.
fn distance(info: &FontInfo, variant: FontVariant) -> impl Ord + Copy {
    (
        info.variant.style.distance(variant.style),
        info.variant.stretch.distance(variant.stretch),
        info.variant.weight.distance(variant.weight),
    )
}

/// How many words the two strings share in their prefix.
fn shared_prefix_words(left: &str, right: &str) -> usize {
    left.unicode_words()
        .zip(right.unicode_words())
        .take_while(|(l, r)| l == r)
        .count()
}
