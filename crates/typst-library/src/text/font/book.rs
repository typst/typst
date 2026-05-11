use std::cmp::Reverse;
use std::collections::BTreeMap;

use unicode_segmentation::UnicodeSegmentation as _;

use crate::text::{
    Font, FontFlags, FontInfo, FontStretch, FontStyle, FontVariant, FontWeight,
    StandardAxes, is_default_ignorable,
};

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
    /// - All else being equal, we prefer variable fonts over static ones.
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
                current.flags.contains(FontFlags::VARIABLE),
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
    // TODO: Potentially also consider optical size for the distance
    // computation. However, this would ideally also apply to non-variable
    // font and there are different mechanisms with which these advertise
    // their intended optical size range.

    let axes = StandardAxes::parse(&info.axes);

    let style_distance = {
        let mut dist = info.variant.style.distance(variant.style);
        if axes.ital.is_some() {
            dist = dist.min(FontStyle::Italic.distance(variant.style));
        }
        if axes.slnt.is_some() {
            dist = dist.min(FontStyle::Oblique.distance(variant.style));
        }
        dist
    };

    let stretch_distance = match axes.wdth {
        Some(axis) => {
            axis.distance(variant.stretch, FontStretch::from_wdth, FontStretch::distance)
        }
        None => info.variant.stretch.distance(variant.stretch),
    };

    let weight_distance = match axes.wght {
        Some(axis) => {
            axis.distance(variant.weight, FontWeight::from_wght, FontWeight::distance)
        }
        None => info.variant.weight.distance(variant.weight),
    };

    (style_distance, stretch_distance, weight_distance)
}

/// How many words the two strings share in their prefix.
fn shared_prefix_words(left: &str, right: &str) -> usize {
    left.unicode_words()
        .zip(right.unicode_words())
        .take_while(|(l, r)| l == r)
        .count()
}

#[cfg(test)]
mod tests {
    use crate::layout::Ratio;
    use crate::text::{
        AxisValue, Coverage, FontAxis, FontBook, FontFlags, FontInfo, FontStretch,
        FontStyle, FontVariant, FontWeight, Tag,
    };

    #[test]
    fn test_find_best_variant() {
        use FontStyle::*;

        let s = [
            info("s0", Normal, 400, 100.0, &[]),
            info("s1", Normal, 500, 100.0, &[]),
            info("s2", Normal, 800, 100.0, &[]),
            info("s3", Italic, 400, 100.0, &[]),
            info("s4", Italic, 800, 100.0, &[]),
            info("s5", Oblique, 800, 100.0, &[]),
            info("s6", Normal, 400, 110.0, &[]),
        ];

        #[rustfmt::skip]
        let v = [
           info("v0", Normal, 400, 100.0, &[("wght", 200.0, 700.0), ("ital", 0.0, 1.0)]),
           info("v1", Normal, 400, 100.0, &[("slnt", -40.0, 40.0), ("wdth", 70.0, 120.0)]),
        ];

        let book = FontBook::from_infos(s.iter().chain(&v).cloned());
        let count = s.len() + v.len();
        let pick = |style, weight, stretch| {
            let target = variant(style, weight, stretch);
            let id = book.find_best_variant(None, target, 0..count).unwrap();
            book.info(id).unwrap()
        };

        // Variable fonts are preferred ...
        assert_eq!(pick(Normal, 100, 100.0), &v[0]);
        assert_eq!(pick(Normal, 200, 100.0), &v[0]);
        assert_eq!(pick(Normal, 500, 100.0), &v[0]);
        assert_eq!(pick(Normal, 730, 100.0), &v[0]);
        assert_eq!(pick(Italic, 400, 100.0), &v[0]);
        assert_eq!(pick(Oblique, 400, 100.0), &v[1]);
        assert_eq!(pick(Normal, 400, 120.0), &v[1]);
        assert_eq!(pick(Normal, 400, 130.0), &v[1]);

        // ... but static variant are still picked if they are closer.
        assert_eq!(pick(Normal, 760, 100.0), &s[2]);
        assert_eq!(pick(Normal, 800, 100.0), &s[2]);
        assert_eq!(pick(Normal, 1000, 100.0), &s[2]);
        assert_eq!(pick(Italic, 800, 110.0), &s[4]);
        assert_eq!(pick(Oblique, 800, 100.0), &s[5]);
        assert_eq!(pick(Oblique, 800, 100.0), &s[5]);
    }

    fn info(
        family: &str,
        style: FontStyle,
        weight: u16,
        stretch: f64,
        axes: &[(&str, f32, f32)],
    ) -> FontInfo {
        FontInfo {
            family: family.into(),
            variant: variant(style, weight, stretch),
            flags: if axes.is_empty() { FontFlags::empty() } else { FontFlags::VARIABLE },
            axes: axes
                .iter()
                .map(|&(t, min, max)| FontAxis {
                    tag: Tag::from_bytes_lossy(t.as_bytes()),
                    min: AxisValue(min),
                    max: AxisValue(max),
                    default: AxisValue((min + max) / 2.0),
                })
                .collect(),
            coverage: Coverage::from_vec(vec![]),
        }
    }

    fn variant(style: FontStyle, weight: u16, stretch: f64) -> FontVariant {
        FontVariant {
            style,
            weight: FontWeight::from_number(weight),
            stretch: FontStretch::from_ratio(Ratio::new(stretch / 100.0)),
        }
    }
}
