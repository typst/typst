use std::borrow::Borrow;
use std::convert::TryFrom;
use std::fmt::{self, Debug, Formatter, Write};
use std::ops::{Add, AddAssign, Deref};

use unicode_segmentation::UnicodeSegmentation;

use crate::diag::StrResult;
use crate::util::EcoString;

/// Create a new [`Str`] from a format string.
macro_rules! format_str {
    ($($tts:tt)*) => {{
        use std::fmt::Write;
        let mut s = $crate::eval::Str::new();
        write!(s, $($tts)*).unwrap();
        s
    }};
}

/// A string value with inline storage and clone-on-write semantics.
#[derive(Default, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Str(EcoString);

impl Str {
    /// Create a new, empty string.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the string is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The length of the string in bytes.
    pub fn len(&self) -> i64 {
        self.0.len() as i64
    }

    /// Borrow this as a string slice.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Return an iterator over the grapheme clusters as strings.
    pub fn iter(&self) -> impl Iterator<Item = Str> + '_ {
        self.graphemes(true).map(Into::into)
    }

    /// Repeat this string `n` times.
    pub fn repeat(&self, n: i64) -> StrResult<Self> {
        let n = usize::try_from(n)
            .ok()
            .and_then(|n| self.0.len().checked_mul(n).map(|_| n))
            .ok_or_else(|| format!("cannot repeat this string {} times", n))?;

        Ok(self.0.repeat(n).into())
    }
}

impl Deref for Str {
    type Target = str;

    fn deref(&self) -> &str {
        self.0.deref()
    }
}

impl Debug for Str {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_char('"')?;
        for c in self.chars() {
            match c {
                '\\' => f.write_str(r"\\")?,
                '"' => f.write_str(r#"\""#)?,
                '\n' => f.write_str(r"\n")?,
                '\r' => f.write_str(r"\r")?,
                '\t' => f.write_str(r"\t")?,
                _ => f.write_char(c)?,
            }
        }
        f.write_char('"')
    }
}

impl Add for Str {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self::Output {
        self += rhs;
        self
    }
}

impl AddAssign for Str {
    fn add_assign(&mut self, rhs: Self) {
        self.0.push_str(rhs.as_str());
    }
}

impl Write for Str {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.write_str(s)
    }

    fn write_char(&mut self, c: char) -> fmt::Result {
        self.0.write_char(c)
    }
}

impl AsRef<str> for Str {
    fn as_ref(&self) -> &str {
        self
    }
}

impl Borrow<str> for Str {
    fn borrow(&self) -> &str {
        self
    }
}

impl From<char> for Str {
    fn from(c: char) -> Self {
        Self(c.into())
    }
}

impl From<&str> for Str {
    fn from(s: &str) -> Self {
        Self(s.into())
    }
}

impl From<String> for Str {
    fn from(s: String) -> Self {
        Self(s.into())
    }
}

impl From<EcoString> for Str {
    fn from(s: EcoString) -> Self {
        Self(s)
    }
}

impl From<&EcoString> for Str {
    fn from(s: &EcoString) -> Self {
        Self(s.clone())
    }
}

impl From<Str> for EcoString {
    fn from(s: Str) -> Self {
        s.0
    }
}

impl From<&Str> for EcoString {
    fn from(s: &Str) -> Self {
        s.0.clone()
    }
}
