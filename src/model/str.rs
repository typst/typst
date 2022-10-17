use std::borrow::{Borrow, Cow};
use std::fmt::{self, Debug, Formatter, Write};
use std::hash::{Hash, Hasher};
use std::ops::{Add, AddAssign, Deref};

use unicode_segmentation::UnicodeSegmentation;

use super::{Array, Dict, RawAlign, Value};
use crate::diag::StrResult;
use crate::util::EcoString;

/// Create a new [`Str`] from a format string.
#[allow(unused_macros)]
macro_rules! format_str {
    ($($tts:tt)*) => {{
        $crate::model::Str::from(format_eco!($($tts)*))
    }};
}

/// An immutable reference counted string.
#[derive(Default, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Str(EcoString);

impl Str {
    /// Create a new, empty string.
    pub fn new() -> Self {
        Self(EcoString::new())
    }

    /// The length of the string in bytes.
    pub fn len(&self) -> i64 {
        self.0.len() as i64
    }

    /// A string slice containing the entire string.
    pub fn as_str(&self) -> &str {
        self
    }

    /// The codepoints the string consists of.
    pub fn codepoints(&self) -> Array {
        self.as_str().chars().map(|c| Value::Str(c.into())).collect()
    }

    /// The grapheme clusters the string consists of.
    pub fn graphemes(&self) -> Array {
        self.as_str().graphemes(true).map(|s| Value::Str(s.into())).collect()
    }

    /// Extract a contigous substring.
    pub fn slice(&self, start: i64, end: Option<i64>) -> StrResult<Self> {
        let len = self.len();
        let start = self
            .locate(start)
            .filter(|&start| start <= self.0.len())
            .ok_or_else(|| out_of_bounds(start, len))?;

        let end = end.unwrap_or(self.len());
        let end = self
            .locate(end)
            .filter(|&end| end <= self.0.len())
            .ok_or_else(|| out_of_bounds(end, len))?
            .max(start);

        Ok(self.0[start .. end].into())
    }

    /// Resolve an index.
    fn locate(&self, index: i64) -> Option<usize> {
        usize::try_from(if index >= 0 {
            index
        } else {
            self.len().checked_add(index)?
        })
        .ok()
    }

    /// Whether the given pattern exists in this string.
    pub fn contains(&self, pattern: StrPattern) -> bool {
        match pattern {
            StrPattern::Str(pat) => self.0.contains(pat.as_str()),
            StrPattern::Regex(re) => re.is_match(self),
        }
    }

    /// Whether this string begins with the given pattern.
    pub fn starts_with(&self, pattern: StrPattern) -> bool {
        match pattern {
            StrPattern::Str(pat) => self.0.starts_with(pat.as_str()),
            StrPattern::Regex(re) => re.find(self).map_or(false, |m| m.start() == 0),
        }
    }

    /// Whether this string ends with the given pattern.
    pub fn ends_with(&self, pattern: StrPattern) -> bool {
        match pattern {
            StrPattern::Str(pat) => self.0.ends_with(pat.as_str()),
            StrPattern::Regex(re) => {
                re.find_iter(self).last().map_or(false, |m| m.end() == self.0.len())
            }
        }
    }

    /// The text of the pattern's first match in this string.
    pub fn find(&self, pattern: StrPattern) -> Option<Self> {
        match pattern {
            StrPattern::Str(pat) => self.0.contains(pat.as_str()).then(|| pat),
            StrPattern::Regex(re) => re.find(self).map(|m| m.as_str().into()),
        }
    }

    /// The position of the pattern's first match in this string.
    pub fn position(&self, pattern: StrPattern) -> Option<i64> {
        match pattern {
            StrPattern::Str(pat) => self.0.find(pat.as_str()).map(|i| i as i64),
            StrPattern::Regex(re) => re.find(self).map(|m| m.start() as i64),
        }
    }

    /// The start and, text and capture groups (if any) of the first match of
    /// the pattern in this string.
    pub fn match_(&self, pattern: StrPattern) -> Option<Dict> {
        match pattern {
            StrPattern::Str(pat) => {
                self.0.match_indices(pat.as_str()).next().map(match_to_dict)
            }
            StrPattern::Regex(re) => re.captures(self).map(captures_to_dict),
        }
    }

    /// The start, end, text and capture groups (if any) of all matches of the
    /// pattern in this string.
    pub fn matches(&self, pattern: StrPattern) -> Array {
        match pattern {
            StrPattern::Str(pat) => self
                .0
                .match_indices(pat.as_str())
                .map(match_to_dict)
                .map(Value::Dict)
                .collect(),
            StrPattern::Regex(re) => re
                .captures_iter(self)
                .map(captures_to_dict)
                .map(Value::Dict)
                .collect(),
        }
    }

    /// Split this string at whitespace or a specific pattern.
    pub fn split(&self, pattern: Option<StrPattern>) -> Array {
        let s = self.as_str();
        match pattern {
            None => s.split_whitespace().map(|v| Value::Str(v.into())).collect(),
            Some(StrPattern::Str(pat)) => {
                s.split(pat.as_str()).map(|v| Value::Str(v.into())).collect()
            }
            Some(StrPattern::Regex(re)) => {
                re.split(s).map(|v| Value::Str(v.into())).collect()
            }
        }
    }

    /// Trim either whitespace or the given pattern at both or just one side of
    /// the string. If `repeat` is true, the pattern is trimmed repeatedly
    /// instead of just once. Repeat must only be given in combination with a
    /// pattern.
    pub fn trim(
        &self,
        pattern: Option<StrPattern>,
        at: Option<StrSide>,
        repeat: bool,
    ) -> Self {
        let mut start = matches!(at, Some(StrSide::Start) | None);
        let end = matches!(at, Some(StrSide::End) | None);

        let trimmed = match pattern {
            None => match at {
                None => self.0.trim(),
                Some(StrSide::Start) => self.0.trim_start(),
                Some(StrSide::End) => self.0.trim_end(),
            },
            Some(StrPattern::Str(pat)) => {
                let pat = pat.as_str();
                let mut s = self.as_str();
                if repeat {
                    if start {
                        s = s.trim_start_matches(pat);
                    }
                    if end {
                        s = s.trim_end_matches(pat);
                    }
                } else {
                    if start {
                        s = s.strip_prefix(pat).unwrap_or(s);
                    }
                    if end {
                        s = s.strip_suffix(pat).unwrap_or(s);
                    }
                }
                s
            }
            Some(StrPattern::Regex(re)) => {
                let s = self.as_str();
                let mut last = 0;
                let mut range = 0 .. s.len();

                for m in re.find_iter(s) {
                    // Does this match follow directly after the last one?
                    let consecutive = last == m.start();

                    // As long as we're consecutive and still trimming at the
                    // start, trim.
                    start &= consecutive;
                    if start {
                        range.start = m.end();
                        start &= repeat;
                    }

                    // Reset end trim if we aren't consecutive anymore or aren't
                    // repeating.
                    if end && (!consecutive || !repeat) {
                        range.end = m.start();
                    }

                    last = m.end();
                }

                // Is the last match directly at the end?
                if last < s.len() {
                    range.end = s.len();
                }

                &s[range.start .. range.start.max(range.end)]
            }
        };

        trimmed.into()
    }

    /// Replace at most `count` occurances of the given pattern with a
    /// replacement string (beginning from the start).
    pub fn replace(&self, pattern: StrPattern, with: Self, count: Option<usize>) -> Self {
        match pattern {
            StrPattern::Str(pat) => match count {
                Some(n) => self.0.replacen(pat.as_str(), &with, n).into(),
                None => self.0.replace(pat.as_str(), &with).into(),
            },
            StrPattern::Regex(re) => match count {
                Some(n) => re.replacen(self, n, with.as_str()).into(),
                None => re.replace(self, with.as_str()).into(),
            },
        }
    }

    /// Repeat the string a number of times.
    pub fn repeat(&self, n: i64) -> StrResult<Self> {
        let n = usize::try_from(n)
            .ok()
            .and_then(|n| self.0.len().checked_mul(n).map(|_| n))
            .ok_or_else(|| format!("cannot repeat this string {} times", n))?;

        Ok(Self(self.0.repeat(n)))
    }
}

/// The out of bounds access error message.
#[cold]
fn out_of_bounds(index: i64, len: i64) -> String {
    format!(
        "string index out of bounds (index: {}, len: {})",
        index, len
    )
}

/// Convert an item of std's `match_indices` to a dictionary.
fn match_to_dict((start, text): (usize, &str)) -> Dict {
    dict! {
        "start" => Value::Int(start as i64),
        "end" => Value::Int((start + text.len()) as i64),
        "text" => Value::Str(text.into()),
        "captures" => Value::Array(Array::new()),
    }
}

/// Convert regex captures to a dictionary.
fn captures_to_dict(cap: regex::Captures) -> Dict {
    let m = cap.get(0).expect("missing first match");
    dict! {
        "start" => Value::Int(m.start() as i64),
        "end" => Value::Int(m.end() as i64),
        "text" => Value::Str(m.as_str().into()),
        "captures" => Value::Array(
            cap.iter()
                .skip(1)
                .map(|opt| opt.map_or(Value::None, |m| m.as_str().into()))
                .collect(),
        ),
    }
}

impl Deref for Str {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
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

impl From<EcoString> for Str {
    fn from(s: EcoString) -> Self {
        Self(s)
    }
}

impl From<String> for Str {
    fn from(s: String) -> Self {
        Self(s.into())
    }
}
impl From<Cow<'_, str>> for Str {
    fn from(s: Cow<str>) -> Self {
        Self(s.into())
    }
}
impl FromIterator<char> for Str {
    fn from_iter<T: IntoIterator<Item = char>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl From<Str> for EcoString {
    fn from(str: Str) -> Self {
        str.0
    }
}

impl From<Str> for String {
    fn from(s: Str) -> Self {
        s.0.into()
    }
}

/// A regular expression.
#[derive(Clone)]
pub struct Regex(regex::Regex);

impl Regex {
    /// Create a new regular expression.
    pub fn new(re: &str) -> StrResult<Self> {
        regex::Regex::new(re).map(Self).map_err(|err| err.to_string())
    }
}

impl Deref for Regex {
    type Target = regex::Regex;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Debug for Regex {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "regex({:?})", self.0.as_str())
    }
}

impl PartialEq for Regex {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_str() == other.0.as_str()
    }
}

impl Hash for Regex {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.as_str().hash(state);
    }
}

/// A pattern which can be searched for in a string.
#[derive(Debug, Clone)]
pub enum StrPattern {
    /// Just a string.
    Str(Str),
    /// A regular expression.
    Regex(Regex),
}

castable! {
    StrPattern,
    Expected: "string or regular expression",
    Value::Str(text) => Self::Str(text),
    @regex: Regex => Self::Regex(regex.clone()),
}

/// A side of a string.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum StrSide {
    /// The logical start of the string, may be left or right depending on the
    /// language.
    Start,
    /// The logical end of the string.
    End,
}

castable! {
    StrSide,
    Expected: "start or end",
    @align: RawAlign => match align {
        RawAlign::Start => Self::Start,
        RawAlign::End => Self::End,
        _ => Err("expected either `start` or `end`")?,
    },
}
