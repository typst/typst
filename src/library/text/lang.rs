use crate::eval::Value;
use crate::geom::Dir;

/// A code for a natural language.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Lang([u8; 3], u8);

impl Lang {
    /// The code for the english language.
    pub const ENGLISH: Self = Self(*b"en ", 2);

    /// Construct a language from a two- or three-byte ISO 639-1/2/3 code.
    pub fn from_str(iso: &str) -> Option<Self> {
        let len = iso.len();
        if matches!(len, 2 ..= 3) && iso.is_ascii() {
            let mut bytes = [b' '; 3];
            bytes[.. len].copy_from_slice(iso.as_bytes());
            bytes.make_ascii_lowercase();
            Some(Self(bytes, len as u8))
        } else {
            None
        }
    }

    /// Return the language code as an all lowercase string slice.
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0[.. usize::from(self.1)]).unwrap_or_default()
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

castable! {
    Lang,
    Expected: "string",
    Value::Str(string) => Self::from_str(&string)
        .ok_or("expected two or three letter language code (ISO 639-1/2/3)")?,
}

/// A code for a region somewhere in the world.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Region([u8; 2]);

impl Region {
    /// Construct a region from its two-byte ISO 3166-1 alpha-2 code.
    pub fn from_str(iso: &str) -> Option<Self> {
        if iso.is_ascii() {
            let mut bytes: [u8; 2] = iso.as_bytes().try_into().ok()?;
            bytes.make_ascii_uppercase();
            Some(Self(bytes))
        } else {
            None
        }
    }

    /// Return the region code as an all uppercase string slice.
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).unwrap_or_default()
    }
}

castable! {
    Region,
    Expected: "string",
    Value::Str(string) => Self::from_str(&string)
        .ok_or("expected two letter region code (ISO 3166-1 alpha-2)")?,
}
