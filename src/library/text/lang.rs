use crate::eval::Value;
use crate::geom::Dir;

/// A natural language.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Lang([u8; 2]);

impl Lang {
    /// The code for the english language.
    pub const ENGLISH: Self = Self(*b"en");

    /// Construct a language from a two-byte ISO 639-1 code.
    pub fn from_str(iso: &str) -> Option<Self> {
        let mut bytes: [u8; 2] = iso.as_bytes().try_into().ok()?;
        bytes.make_ascii_lowercase();
        Some(Self(bytes))
    }

    /// Return the language code as a string slice.
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).unwrap_or_default()
    }

    /// The default direction for the language.
    pub fn dir(&self) -> Dir {
        match self.as_str() {
            "ar" | "dv" | "fa" | "he" | "ks" | "pa" | "ps" | "sd" | "ug" | "ur"
            | "yi" => Dir::RTL,
            _ => Dir::LTR,
        }
    }
}

castable! {
    Lang,
    Expected: "string",
    Value::Str(string) => Self::from_str(&string)
        .ok_or("expected two letter language code")?,
}
