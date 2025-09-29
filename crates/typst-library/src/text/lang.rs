use std::str::FromStr;

use ecow::{EcoString, eco_format};
use rustc_hash::FxHashMap;

use crate::diag::Hint;
use crate::foundations::{StyleChain, cast};
use crate::layout::Dir;
use crate::text::TextElem;

macro_rules! translation {
    ($lang:literal) => {
        ($lang, include_str!(concat!("../../translations/", $lang, ".txt")))
    };
}

const TRANSLATIONS: &[(&str, &str)] = &[
    translation!("ar"),
    translation!("bg"),
    translation!("ca"),
    translation!("cs"),
    translation!("da"),
    translation!("de"),
    translation!("el"),
    translation!("en"),
    translation!("es"),
    translation!("et"),
    translation!("eu"),
    translation!("fi"),
    translation!("fr"),
    translation!("gl"),
    translation!("he"),
    translation!("hr"),
    translation!("hu"),
    translation!("id"),
    translation!("is"),
    translation!("it"),
    translation!("ja"),
    translation!("la"),
    translation!("lt"),
    translation!("lv"),
    translation!("nb"),
    translation!("nl"),
    translation!("nn"),
    translation!("pl"),
    translation!("pt"),
    translation!("pt-PT"),
    translation!("ro"),
    translation!("ru"),
    translation!("sl"),
    translation!("sq"),
    translation!("sr"),
    translation!("sv"),
    translation!("tl"),
    translation!("tr"),
    translation!("uk"),
    translation!("vi"),
    translation!("zh"),
    translation!("zh-TW"),
];

/// A locale consisting of a language and an optional region.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Locale {
    pub lang: Lang,
    pub region: Option<Region>,
}

impl Default for Locale {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl Locale {
    pub const DEFAULT: Self = Self::new(Lang::ENGLISH, None);

    pub const fn new(lang: Lang, region: Option<Region>) -> Self {
        Locale { lang, region }
    }

    pub fn get_in(styles: StyleChain) -> Self {
        Locale::new(styles.get(TextElem::lang), styles.get(TextElem::region))
    }

    pub fn rfc_3066(self) -> EcoString {
        let mut buf = EcoString::from(self.lang.as_str());
        if let Some(region) = self.region {
            buf.push('-');
            buf.push_str(region.as_str());
        }
        buf
    }
}

/// An identifier for a natural language.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Lang([u8; 3], u8);

impl Lang {
    pub const ALBANIAN: Self = Self(*b"sq ", 2);
    pub const ARABIC: Self = Self(*b"ar ", 2);
    pub const BASQUE: Self = Self(*b"eu ", 2);
    pub const BOKMÅL: Self = Self(*b"nb ", 2);
    pub const BULGARIAN: Self = Self(*b"bg ", 2);
    pub const CATALAN: Self = Self(*b"ca ", 2);
    pub const CHINESE: Self = Self(*b"zh ", 2);
    pub const CROATIAN: Self = Self(*b"hr ", 2);
    pub const CZECH: Self = Self(*b"cs ", 2);
    pub const DANISH: Self = Self(*b"da ", 2);
    pub const DUTCH: Self = Self(*b"nl ", 2);
    pub const ENGLISH: Self = Self(*b"en ", 2);
    pub const ESTONIAN: Self = Self(*b"et ", 2);
    pub const FILIPINO: Self = Self(*b"tl ", 2);
    pub const FINNISH: Self = Self(*b"fi ", 2);
    pub const FRENCH: Self = Self(*b"fr ", 2);
    pub const GALICIAN: Self = Self(*b"gl ", 2);
    pub const GERMAN: Self = Self(*b"de ", 2);
    pub const GREEK: Self = Self(*b"el ", 2);
    pub const HEBREW: Self = Self(*b"he ", 2);
    pub const HUNGARIAN: Self = Self(*b"hu ", 2);
    pub const ICELANDIC: Self = Self(*b"is ", 2);
    pub const INDONESIAN: Self = Self(*b"id ", 2);
    pub const ITALIAN: Self = Self(*b"it ", 2);
    pub const JAPANESE: Self = Self(*b"ja ", 2);
    pub const LATIN: Self = Self(*b"la ", 2);
    pub const LATVIAN: Self = Self(*b"lv ", 2);
    pub const LITHUANIAN: Self = Self(*b"lt ", 2);
    pub const LOWER_SORBIAN: Self = Self(*b"dsb", 3);
    pub const NYNORSK: Self = Self(*b"nn ", 2);
    pub const POLISH: Self = Self(*b"pl ", 2);
    pub const PORTUGUESE: Self = Self(*b"pt ", 2);
    pub const ROMANIAN: Self = Self(*b"ro ", 2);
    pub const RUSSIAN: Self = Self(*b"ru ", 2);
    pub const SERBIAN: Self = Self(*b"sr ", 2);
    pub const SLOVAK: Self = Self(*b"sk ", 2);
    pub const SLOVENIAN: Self = Self(*b"sl ", 2);
    pub const SPANISH: Self = Self(*b"es ", 2);
    pub const SWEDISH: Self = Self(*b"sv ", 2);
    pub const TURKISH: Self = Self(*b"tr ", 2);
    pub const UKRAINIAN: Self = Self(*b"uk ", 2);
    pub const VIETNAMESE: Self = Self(*b"vi ", 2);

    /// Return the language code as an all lowercase string slice.
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0[..usize::from(self.1)]).unwrap_or_default()
    }

    /// The default direction for the language.
    pub fn dir(self) -> Dir {
        match self.as_str() {
            "ar" | "dv" | "fa" | "he" | "ks" | "pa" | "ps" | "sd" | "ug" | "ur"
            | "yi" => Dir::RTL,
            _ => Dir::LTR,
        }
    }
}

impl FromStr for Lang {
    type Err = &'static str;

    /// Construct a language from a two- or three-byte ISO 639-1/2/3 code.
    fn from_str(iso: &str) -> Result<Self, Self::Err> {
        let len = iso.len();
        if matches!(len, 2..=3) && iso.is_ascii() {
            let mut bytes = [b' '; 3];
            bytes[..len].copy_from_slice(iso.as_bytes());
            bytes.make_ascii_lowercase();
            Ok(Self(bytes, len as u8))
        } else {
            Err("expected two or three letter language code (ISO 639-1/2/3)")
        }
    }
}

cast! {
    Lang,
    self => self.as_str().into_value(),
    string: EcoString => {
        let result = Self::from_str(&string);
        if result.is_err()
            && let Some((lang, region)) = string.split_once('-')
                && Lang::from_str(lang).is_ok() && Region::from_str(region).is_ok() {
                    return result
                        .hint(eco_format!(
                            "you should leave only \"{}\" in the `lang` parameter and specify \"{}\" in the `region` parameter",
                            lang, region,
                        ));
                }

        result?
    }
}

/// An identifier for a region somewhere in the world.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Region([u8; 2]);

impl Region {
    /// Return the region code as an all uppercase string slice.
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).unwrap_or_default()
    }
}

impl PartialEq<&str> for Region {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl FromStr for Region {
    type Err = &'static str;

    /// Construct a region from its two-byte ISO 3166-1 alpha-2 code.
    fn from_str(iso: &str) -> Result<Self, Self::Err> {
        if iso.len() == 2 && iso.is_ascii() {
            let mut bytes: [u8; 2] = iso.as_bytes().try_into().unwrap();
            bytes.make_ascii_uppercase();
            Ok(Self(bytes))
        } else {
            Err("expected two letter region code (ISO 3166-1 alpha-2)")
        }
    }
}

cast! {
    Region,
    self => self.as_str().into_value(),
    string: EcoString => Self::from_str(&string)?,
}

/// An ISO 15924-type script identifier.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct WritingScript([u8; 4], u8);

impl WritingScript {
    /// Return the script as an all lowercase string slice.
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0[..usize::from(self.1)]).unwrap_or_default()
    }

    /// Return the description of the script as raw bytes.
    pub fn as_bytes(&self) -> &[u8; 4] {
        &self.0
    }
}

impl FromStr for WritingScript {
    type Err = &'static str;

    /// Construct a region from its ISO 15924 code.
    fn from_str(iso: &str) -> Result<Self, Self::Err> {
        let len = iso.len();
        if matches!(len, 3..=4) && iso.is_ascii() {
            let mut bytes = [b' '; 4];
            bytes[..len].copy_from_slice(iso.as_bytes());
            bytes.make_ascii_lowercase();
            Ok(Self(bytes, len as u8))
        } else {
            Err("expected three or four letter script code (ISO 15924 or 'math')")
        }
    }
}

cast! {
    WritingScript,
    self => self.as_str().into_value(),
    string: EcoString => Self::from_str(&string)?,
}

/// The name with which an element is referenced.
pub trait LocalName {
    /// The key of an element in order to get its localized name.
    const KEY: &'static str;

    /// Get the name in the given language and (optionally) region.
    fn local_name(lang: Lang, region: Option<Region>) -> &'static str {
        localized_str(lang, region, Self::KEY)
    }

    /// Gets the local name from the style chain.
    fn local_name_in(styles: StyleChain) -> &'static str
    where
        Self: Sized,
    {
        Self::local_name(styles.get(TextElem::lang), styles.get(TextElem::region))
    }
}

/// Retrieves the localized string for a given language and region.
/// Silently falls back to English if no fitting string exists for
/// the given language + region. Panics if no fitting string exists
/// in both given language + region and English.
#[comemo::memoize]
pub fn localized_str(lang: Lang, region: Option<Region>, key: &str) -> &'static str {
    let lang_region_bundle = parse_language_bundle(lang, region).unwrap();
    if let Some(str) = lang_region_bundle.get(key) {
        return str;
    }
    let lang_bundle = parse_language_bundle(lang, None).unwrap();
    if let Some(str) = lang_bundle.get(key) {
        return str;
    }
    let english_bundle = parse_language_bundle(Lang::ENGLISH, None).unwrap();
    english_bundle.get(key).unwrap()
}

/// Parses the translation file for a given language and region.
/// Only returns an error if the language file is malformed.
#[comemo::memoize]
fn parse_language_bundle(
    lang: Lang,
    region: Option<Region>,
) -> Result<FxHashMap<&'static str, &'static str>, &'static str> {
    let language_tuple = TRANSLATIONS.iter().find(|it| it.0 == lang_str(lang, region));
    let Some((_lang_name, language_file)) = language_tuple else {
        return Ok(FxHashMap::default());
    };

    let mut bundle = FxHashMap::default();
    let lines = language_file.trim().lines();
    for line in lines {
        if line.trim().starts_with('#') {
            continue;
        }
        let (key, val) = line
            .split_once('=')
            .ok_or("malformed translation file: line without \"=\"")?;
        let (key, val) = (key.trim(), val.trim());
        if val.is_empty() {
            return Err("malformed translation file: empty translation value");
        }
        let duplicate = bundle.insert(key.trim(), val.trim());
        if duplicate.is_some() {
            return Err("malformed translation file: duplicate key");
        }
    }
    Ok(bundle)
}

/// Convert language + region to a string to be able to get a file name.
fn lang_str(lang: Lang, region: Option<Region>) -> EcoString {
    EcoString::from(lang.as_str())
        + region.map_or_else(EcoString::new, |r| EcoString::from("-") + r.as_str())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use rustc_hash::FxHashSet;
    use typst_utils::option_eq;

    use super::*;

    fn translation_files_iter() -> impl Iterator<Item = PathBuf> {
        std::fs::read_dir("translations")
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|e| e.is_file() && e.extension().is_some_and(|e| e == "txt"))
    }

    #[test]
    fn test_region_option_eq() {
        let region = Some(Region([b'U', b'S']));
        assert!(option_eq(region, "US"));
        assert!(!option_eq(region, "AB"));
    }

    #[test]
    fn test_all_translations_included() {
        let defined_keys =
            FxHashSet::<&str>::from_iter(TRANSLATIONS.iter().map(|(lang, _)| *lang));
        let mut checked = 0;
        for file in translation_files_iter() {
            assert!(
                defined_keys.contains(
                    file.file_stem()
                        .expect("translation file should have basename")
                        .to_str()
                        .expect("translation file name should be utf-8 encoded")
                ),
                "translation from {:?} should be registered in TRANSLATIONS in {}",
                file.file_name().unwrap(),
                file!(),
            );
            checked += 1;
        }
        assert_eq!(TRANSLATIONS.len(), checked);
    }

    #[test]
    fn test_all_translation_files_formatted() {
        for file in translation_files_iter() {
            let content = std::fs::read_to_string(&file)
                .expect("translation file should be in utf-8 encoding");
            let filename = file.file_name().unwrap();
            assert!(
                content.ends_with('\n'),
                "translation file {filename:?} should end with linebreak",
            );
            for line in content.lines() {
                assert_eq!(
                    line.trim(),
                    line,
                    "line {line:?} in {filename:?} should not have extra whitespaces"
                );
            }
        }
    }

    #[test]
    fn test_translations_sorted() {
        assert!(
            TRANSLATIONS.is_sorted_by_key(|(lang, _)| lang),
            "TRANSLATIONS should be sorted"
        );
    }
}
