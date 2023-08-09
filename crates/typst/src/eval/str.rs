use std::borrow::{Borrow, Cow};
use std::fmt::{self, Debug, Display, Formatter, Write};
use std::hash::{Hash, Hasher};
use std::ops::{Add, AddAssign, Deref, Range};

use ecow::EcoString;
use serde::Serialize;
use unicode_segmentation::UnicodeSegmentation;

use super::{cast, dict, Args, Array, Dict, Func, IntoValue, Value, Vm};
use crate::diag::{bail, At, SourceResult, StrResult};
use crate::geom::GenAlign;

/// Create a new [`Str`] from a format string.
#[macro_export]
#[doc(hidden)]
macro_rules! __format_str {
    ($($tts:tt)*) => {{
        $crate::eval::Str::from($crate::eval::eco_format!($($tts)*))
    }};
}

#[doc(inline)]
pub use crate::__format_str as format_str;
#[doc(hidden)]
pub use ecow::eco_format;

/// An immutable reference counted string.
#[derive(Default, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize)]
pub struct Str(EcoString);

impl Str {
    /// Create a new, empty string.
    pub fn new() -> Self {
        Self(EcoString::new())
    }

    /// Return `true` if the length is 0.
    pub fn is_empty(&self) -> bool {
        self.0.len() == 0
    }

    /// The length of the string in bytes.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// A string slice containing the entire string.
    pub fn as_str(&self) -> &str {
        self
    }

    /// Extract the first grapheme cluster.
    pub fn first(&self) -> StrResult<Self> {
        self.0
            .graphemes(true)
            .next()
            .map(Into::into)
            .ok_or_else(string_is_empty)
    }

    /// Extract the last grapheme cluster.
    pub fn last(&self) -> StrResult<Self> {
        self.0
            .graphemes(true)
            .next_back()
            .map(Into::into)
            .ok_or_else(string_is_empty)
    }

    /// Extract the grapheme cluster at the given index.
    pub fn at(&self, index: i64, default: Option<Value>) -> StrResult<Value> {
        let len = self.len();
        self.locate_opt(index)?
            .and_then(|i| self.0[i..].graphemes(true).next().map(|s| s.into_value()))
            .or(default)
            .ok_or_else(|| no_default_and_out_of_bounds(index, len))
    }

    /// Extract a contiguous substring.
    pub fn slice(&self, start: i64, end: Option<i64>) -> StrResult<Self> {
        let start = self.locate(start)?;
        let end = self.locate(end.unwrap_or(self.len() as i64))?.max(start);
        Ok(self.0[start..end].into())
    }

    /// The grapheme clusters the string consists of.
    pub fn clusters(&self) -> Array {
        self.as_str().graphemes(true).map(|s| Value::Str(s.into())).collect()
    }

    /// The codepoints the string consists of.
    pub fn codepoints(&self) -> Array {
        self.chars().map(|c| Value::Str(c.into())).collect()
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
            StrPattern::Str(pat) => self.0.contains(pat.as_str()).then_some(pat),
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
                let mut range = 0..s.len();

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

                &s[range.start..range.start.max(range.end)]
            }
        };

        trimmed.into()
    }

    /// Replace at most `count` occurrences of the given pattern with a
    /// replacement string or function (beginning from the start). If no count
    /// is given, all occurrences are replaced.
    pub fn replace(
        &self,
        vm: &mut Vm,
        pattern: StrPattern,
        with: Replacement,
        count: Option<usize>,
    ) -> SourceResult<Self> {
        // Heuristic: Assume the new string is about the same length as
        // the current string.
        let mut output = EcoString::with_capacity(self.as_str().len());

        // Replace one match of a pattern with the replacement.
        let mut last_match = 0;
        let mut handle_match = |range: Range<usize>, dict: Dict| -> SourceResult<()> {
            // Push everything until the match.
            output.push_str(&self[last_match..range.start]);
            last_match = range.end;

            // Determine and push the replacement.
            match &with {
                Replacement::Str(s) => output.push_str(s),
                Replacement::Func(func) => {
                    let args = Args::new(func.span(), [dict]);
                    let piece = func.call_vm(vm, args)?.cast::<Str>().at(func.span())?;
                    output.push_str(&piece);
                }
            }

            Ok(())
        };

        // Iterate over the matches of the `pattern`.
        let count = count.unwrap_or(usize::MAX);
        match &pattern {
            StrPattern::Str(pat) => {
                for m in self.match_indices(pat.as_str()).take(count) {
                    let (start, text) = m;
                    handle_match(start..start + text.len(), match_to_dict(m))?;
                }
            }
            StrPattern::Regex(re) => {
                for caps in re.captures_iter(self).take(count) {
                    // Extract the entire match over all capture groups.
                    let m = caps.get(0).unwrap();
                    handle_match(m.start()..m.end(), captures_to_dict(caps))?;
                }
            }
        }

        // Push the remainder.
        output.push_str(&self[last_match..]);
        Ok(output.into())
    }

    /// Repeat the string a number of times.
    pub fn repeat(&self, n: i64) -> StrResult<Self> {
        let n = usize::try_from(n)
            .ok()
            .and_then(|n| self.0.len().checked_mul(n).map(|_| n))
            .ok_or_else(|| format!("cannot repeat this string {} times", n))?;

        Ok(Self(self.0.repeat(n)))
    }

    /// Resolve an index or throw an out of bounds error.
    fn locate(&self, index: i64) -> StrResult<usize> {
        self.locate_opt(index)?
            .ok_or_else(|| out_of_bounds(index, self.len()))
    }

    /// Resolve an index, if it is within bounds and on a valid char boundary.
    ///
    /// `index == len` is considered in bounds.
    fn locate_opt(&self, index: i64) -> StrResult<Option<usize>> {
        let wrapped =
            if index >= 0 { Some(index) } else { (self.len() as i64).checked_add(index) };

        let resolved = wrapped
            .and_then(|v| usize::try_from(v).ok())
            .filter(|&v| v <= self.0.len());

        if resolved.map_or(false, |i| !self.0.is_char_boundary(i)) {
            return Err(not_a_char_boundary(index));
        }

        Ok(resolved)
    }
}

/// The out of bounds access error message.
#[cold]
fn out_of_bounds(index: i64, len: usize) -> EcoString {
    eco_format!("string index out of bounds (index: {}, len: {})", index, len)
}

/// The out of bounds access error message when no default value was given.
#[cold]
fn no_default_and_out_of_bounds(index: i64, len: usize) -> EcoString {
    eco_format!("no default value was specified and string index out of bounds (index: {}, len: {})", index, len)
}

/// The char boundary access error message.
#[cold]
fn not_a_char_boundary(index: i64) -> EcoString {
    eco_format!("string index {} is not a character boundary", index)
}

/// The error message when the string is empty.
#[cold]
fn string_is_empty() -> EcoString {
    "string is empty".into()
}

/// Convert an item of std's `match_indices` to a dictionary.
fn match_to_dict((start, text): (usize, &str)) -> Dict {
    dict! {
        "start" => start,
        "end" => start + text.len(),
        "text" => text,
        "captures" => Array::new(),
    }
}

/// Convert regex captures to a dictionary.
fn captures_to_dict(cap: regex::Captures) -> Dict {
    let m = cap.get(0).expect("missing first match");
    dict! {
        "start" => m.start(),
        "end" => m.end(),
        "text" => m.as_str(),
        "captures" =>  cap.iter()
            .skip(1)
            .map(|opt| opt.map_or(Value::None, |m| m.as_str().into_value()))
            .collect::<Array>(),
    }
}

impl Deref for Str {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl Display for Str {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(self)
    }
}

impl Debug for Str {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_char('"')?;
        for c in self.chars() {
            match c {
                '\0' => f.write_str("\\u{0}")?,
                '\'' => f.write_str("'")?,
                '"' => f.write_str(r#"\""#)?,
                _ => Display::fmt(&c.escape_debug(), f)?,
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

cast! {
    char,
    self => Value::Str(self.into()),
    string: Str => {
        let mut chars = string.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) => c,
            _ => bail!("expected exactly one character"),
        }
    },
}

cast! {
    &str,
    self => Value::Str(self.into()),
}

cast! {
    EcoString,
    self => Value::Str(self.into()),
    v: Str => v.into(),
}

cast! {
    String,
    self => Value::Str(self.into()),
    v: Str => v.into(),
}

/// A regular expression.
#[derive(Clone)]
pub struct Regex(regex::Regex);

impl Regex {
    /// Create a new regular expression.
    pub fn new(re: &str) -> StrResult<Self> {
        regex::Regex::new(re).map(Self).map_err(|err| eco_format!("{err}"))
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

cast! {
    type Regex: "regular expression",
}

/// A pattern which can be searched for in a string.
#[derive(Debug, Clone)]
pub enum StrPattern {
    /// Just a string.
    Str(Str),
    /// A regular expression.
    Regex(Regex),
}

cast! {
    StrPattern,
    text: Str => Self::Str(text),
    regex: Regex => Self::Regex(regex),
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

cast! {
    StrSide,
    align: GenAlign => match align {
        GenAlign::Start => Self::Start,
        GenAlign::End => Self::End,
        _ => bail!("expected either `start` or `end`"),
    },
}

/// A replacement for a matched [`Str`]
pub enum Replacement {
    /// A string a match is replaced with.
    Str(Str),
    /// Function of type Dict -> Str (see `captures_to_dict` or `match_to_dict`)
    /// whose output is inserted for the match.
    Func(Func),
}

cast! {
    Replacement,
    text: Str => Self::Str(text),
    func: Func => Self::Func(func)
}
