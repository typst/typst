use std::borrow::Cow;
use std::fmt::{self, Debug, Display, Formatter};

use serde::{Deserialize, Serialize};
use unicode_segmentation::UnicodeSegmentation as _;

use crate::diag::bail;
use crate::foundations::{Repr as _, Str, cast};

/// A 4-byte OpenType tag.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Tag([u8; 4]);

impl Tag {
    /// Creates a new tag from exactly four bytes.
    pub const fn from_bytes(bytes: &[u8; 4]) -> Self {
        Self(*bytes)
    }

    /// Creates a new tag from any number of bytes, padding with spaces
    /// or filling
    pub fn from_bytes_lossy(bytes: &[u8]) -> Self {
        let mut array = [b' '; 4];
        let len = bytes.len().min(4);
        array[..len].copy_from_slice(&bytes[..len]);
        Self(array)
    }

    /// Return the four bytes making up the tag.
    pub const fn to_bytes(self) -> [u8; 4] {
        self.0
    }

    /// Turns this into a string.
    pub fn to_str_lossy(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.0)
    }
}

impl Debug for Tag {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Tag({})", self.to_str_lossy())
    }
}

impl Display for Tag {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.to_str_lossy(), f)
    }
}

impl Serialize for Tag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if serializer.is_human_readable() {
            self.to_str_lossy().serialize(serializer)
        } else {
            serializer.serialize_u32(u32::from_be_bytes(self.0))
        }
    }
}

impl<'de> Deserialize<'de> for Tag {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            let s = String::deserialize(deserializer)?;
            let bytes = s.as_bytes().try_into().map_err(serde::de::Error::custom)?;
            Ok(Self::from_bytes(bytes))
        } else {
            let v = u32::deserialize(deserializer)?;
            Ok(Self(v.to_be_bytes()))
        }
    }
}

// Tags must: https://learn.microsoft.com/en-us/typography/opentype/spec/otff#data-types
// - be one to four bytes in length
// - be representable as printable ASCII (0x20..=0x7E)
// - contain at least one character that isn't padding (0x20, space)
// - padding may only appear at the end of a tag
cast! {
    Tag,
    v: Str => {
        if let Some(cluster) = v.graphemes(true).find(|v| {
            !v.as_bytes().iter().all(|v| (0x20..=0x7E).contains(v))
        }) {
            bail!(
                "tag may contain only printable ASCII characters";
                hint: "found invalid cluster `{}`", cluster.repr();
            )
        }

        if !(1..=4).contains(&v.len()) {
            bail!(
                "tag must be one to four characters in length";
                hint: "found {} characters", v.len();
            );
        }

        let mut within_padding = false;
        for (i, &v) in v.as_bytes().iter().enumerate() {
            if (within_padding && v != b' ') || (i == 0 && v == b' ') {
                bail!("spaces may only appear as padding following a tag")
            }
            within_padding |= b' ' == v;
        }

        Self::from_bytes_lossy(v.as_bytes())
    }
}

impl From<Tag> for ttf_parser::Tag {
    fn from(value: Tag) -> Self {
        ttf_parser::Tag::from_bytes(&value.to_bytes())
    }
}
