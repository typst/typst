use std::convert::TryFrom;
use std::fmt::{self, Debug, Display, Formatter, Write};
use std::ops::{Add, AddAssign, Deref};

use crate::diag::StrResult;
use crate::util::EcoString;

/// A string value with inline storage and clone-on-write semantics.
#[derive(Default, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Str {
    string: EcoString,
}

impl Str {
    /// Create a new, empty string.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the string is empty.
    pub fn is_empty(&self) -> bool {
        self.string.is_empty()
    }

    /// The length of the string in bytes.
    pub fn len(&self) -> i64 {
        self.string.len() as i64
    }

    /// Borrow this as a string slice.
    pub fn as_str(&self) -> &str {
        self.string.as_str()
    }

    /// Return an iterator over the chars as strings.
    pub fn iter(&self) -> impl Iterator<Item = Str> + '_ {
        self.chars().map(Into::into)
    }

    /// Repeat this string `n` times.
    pub fn repeat(&self, n: i64) -> StrResult<Self> {
        let n = usize::try_from(n)
            .ok()
            .and_then(|n| self.string.len().checked_mul(n).map(|_| n))
            .ok_or_else(|| format!("cannot repeat this string {} times", n))?;

        Ok(self.string.repeat(n).into())
    }
}

impl Display for Str {
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

impl Debug for Str {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Debug::fmt(&self.string, f)
    }
}

impl Deref for Str {
    type Target = str;

    fn deref(&self) -> &str {
        self.string.deref()
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
        self.string.push_str(rhs.as_str());
    }
}

impl From<char> for Str {
    fn from(c: char) -> Self {
        Self { string: c.into() }
    }
}

impl From<&str> for Str {
    fn from(string: &str) -> Self {
        Self { string: string.into() }
    }
}

impl From<String> for Str {
    fn from(string: String) -> Self {
        Self { string: string.into() }
    }
}

impl From<EcoString> for Str {
    fn from(string: EcoString) -> Self {
        Self { string }
    }
}

impl From<&EcoString> for Str {
    fn from(string: &EcoString) -> Self {
        Self { string: string.clone() }
    }
}

impl From<Str> for EcoString {
    fn from(string: Str) -> Self {
        string.string
    }
}

impl From<&Str> for EcoString {
    fn from(string: &Str) -> Self {
        string.string.clone()
    }
}
