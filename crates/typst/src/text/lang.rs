use std::str::FromStr;

use ecow::EcoString;

use crate::foundations::{cast, StyleChain};
use crate::layout::Dir;
use crate::text::TextElem;

/// An identifier for a natural language.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Lang([u8; 3], u8);

impl Lang {
    pub const ALBANIAN: Self = Self(*b"sq ", 2);
    pub const ARABIC: Self = Self(*b"ar ", 2);
    pub const BOKMÅL: Self = Self(*b"nb ", 2);
    pub const CATALAN: Self = Self(*b"ca ", 2);
    pub const CHINESE: Self = Self(*b"zh ", 2);
    pub const CZECH: Self = Self(*b"cs ", 2);
    pub const DANISH: Self = Self(*b"da ", 2);
    pub const DUTCH: Self = Self(*b"nl ", 2);
    pub const ENGLISH: Self = Self(*b"en ", 2);
    pub const ESTONIAN: Self = Self(*b"et ", 2);
    pub const FILIPINO: Self = Self(*b"tl ", 2);
    pub const FINNISH: Self = Self(*b"fi ", 2);
    pub const FRENCH: Self = Self(*b"fr ", 2);
    pub const GERMAN: Self = Self(*b"de ", 2);
    pub const GREEK: Self = Self(*b"gr ", 2);
    pub const ITALIAN: Self = Self(*b"it ", 2);
    pub const JAPANESE: Self = Self(*b"ja ", 2);
    pub const NYNORSK: Self = Self(*b"nn ", 2);
    pub const POLISH: Self = Self(*b"pl ", 2);
    pub const PORTUGUESE: Self = Self(*b"pt ", 2);
    pub const RUSSIAN: Self = Self(*b"ru ", 2);
    pub const SERBIAN: Self = Self(*b"sr ", 2);
    pub const SLOVENIAN: Self = Self(*b"sl ", 2);
    pub const SPANISH: Self = Self(*b"es ", 2);
    pub const SWEDISH: Self = Self(*b"sv ", 2);
    pub const TURKISH: Self = Self(*b"tr ", 2);
    pub const UKRAINIAN: Self = Self(*b"ua ", 2);
    pub const VIETNAMESE: Self = Self(*b"vi ", 2);
    pub const HUNGARIAN: Self = Self(*b"hu ", 2);
    pub const ROMANIAN: Self = Self(*b"ro ", 2);

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
    string: EcoString => Self::from_str(&string)?,
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
    /// Get the name in the given language and (optionally) region.
    fn local_name(lang: Lang, region: Option<Region>) -> &'static str;

    /// Gets the local name from the style chain.
    fn local_name_in(styles: StyleChain) -> &'static str
    where
        Self: Sized,
    {
        Self::local_name(TextElem::lang_in(styles), TextElem::region_in(styles))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::option_eq;

    #[test]
    fn test_region_option_eq() {
        let region = Some(Region([b'U', b'S']));
        assert!(option_eq(region, "US"));
        assert!(!option_eq(region, "AB"));
    }
}
