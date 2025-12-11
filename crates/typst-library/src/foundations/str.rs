use std::borrow::{Borrow, Cow};
use std::fmt::{self, Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::{Add, AddAssign, Deref, Range};

use comemo::Tracked;
use ecow::EcoString;
use serde::{Deserialize, Serialize};
use typst_syntax::{Span, Spanned};
use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

use crate::diag::{At, SourceResult, StrResult, bail};
use crate::engine::Engine;
use crate::foundations::{
    Array, Bytes, Cast, Context, Decimal, Dict, Func, IntoValue, Label, Repr, Type,
    Value, Version, cast, dict, func, repr, scope, ty,
};
use crate::layout::Alignment;

/// Create a new [`Str`] from a format string.
#[macro_export]
#[doc(hidden)]
macro_rules! __format_str {
    ($($tts:tt)*) => {{
        $crate::foundations::Str::from($crate::foundations::eco_format!($($tts)*))
    }};
}

#[doc(hidden)]
pub use ecow::eco_format;

#[doc(inline)]
pub use crate::__format_str as format_str;

/// A sequence of Unicode codepoints.
///
/// You can iterate over the grapheme clusters of the string using a [for
/// loop]($scripting/#loops). Grapheme clusters are basically characters but
/// keep together things that belong together, e.g. multiple codepoints that
/// together form a flag emoji. Strings can be added with the `+` operator,
/// [joined together]($scripting/#blocks) and multiplied with integers.
///
/// Typst provides utility methods for string manipulation. Many of these
/// methods (e.g., [`split`]($str.split), [`trim`]($str.trim) and
/// [`replace`]($str.replace)) operate on _patterns:_ A pattern can be either a
/// string or a [regular expression]($regex). This makes the methods quite
/// versatile.
///
/// All lengths and indices are expressed in terms of UTF-8 bytes. Indices are
/// zero-based and negative indices wrap around to the end of the string.
///
/// You can convert a value to a string with the `str` constructor.
///
/// # Example
/// ```example
/// #"hello world!" \
/// #"\"hello\n  world\"!" \
/// #"1 2 3".split() \
/// #"1,2;3".split(regex("[,;]")) \
/// #(regex("\\d+") in "ten euros") \
/// #(regex("\\d+") in "10 euros")
/// ```
///
/// # Escape sequences { #escapes }
/// Just like in markup, you can escape a few symbols in strings:
/// - `[\\]` for a backslash
/// - `[\"]` for a quote
/// - `[\n]` for a newline
/// - `[\r]` for a carriage return
/// - `[\t]` for a tab
/// - `[\u{1f600}]` for a hexadecimal Unicode escape sequence
#[ty(scope, cast, title = "String")]
#[derive(Default, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct Str(EcoString);

impl Str {
    /// Create a new, empty string.
    pub fn new() -> Self {
        Self(EcoString::new())
    }

    /// Return `true` if the length is 0.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Repeat the string a number of times.
    pub fn repeat(&self, n: usize) -> StrResult<Self> {
        if self.0.len().checked_mul(n).is_none() {
            return Err(eco_format!("cannot repeat this string {n} times"));
        }
        Ok(Self(self.0.repeat(n)))
    }

    /// A string slice containing the entire string.
    pub fn as_str(&self) -> &str {
        self
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

        if resolved.is_some_and(|i| !self.0.is_char_boundary(i)) {
            return Err(not_a_char_boundary(index));
        }

        Ok(resolved)
    }
}

#[scope]
impl Str {
    /// Converts a value to a string.
    ///
    /// - Integers are formatted in base 10. This can be overridden with the
    ///   optional `base` parameter.
    /// - Floats are formatted in base 10 and never in exponential notation.
    /// - Negative integers and floats are formatted with the Unicode minus sign
    ///   ("−" U+2212) instead of the ASCII minus sign ("-" U+002D).
    /// - From labels the name is extracted.
    /// - Bytes are decoded as UTF-8.
    ///
    /// If you wish to convert from and to Unicode code points, see the
    /// [`to-unicode`]($str.to-unicode) and [`from-unicode`]($str.from-unicode)
    /// functions.
    ///
    /// ```example
    /// #str(10) \
    /// #str(4000, base: 16) \
    /// #str(2.7) \
    /// #str(1e8) \
    /// #str(<intro>)
    /// ```
    #[func(constructor)]
    pub fn construct(
        /// The value that should be converted to a string.
        value: ToStr,
        /// The base (radix) to display integers in, between 2 and 36.
        #[named]
        #[default(Spanned::new(Base::Default, Span::detached()))]
        base: Spanned<Base>,
    ) -> SourceResult<Str> {
        Ok(match value {
            ToStr::Str(s) => {
                if matches!(base.v, Base::User(_)) {
                    bail!(base.span, "base is only supported for integers");
                }
                s
            }
            ToStr::Int(n) => {
                let b = base.v.value();
                if b == 1 && n > 0 {
                    bail!(
                        base.span, "base must be between 2 and 36";
                        hint: "generate a unary representation with `\"1\" * {n}`";
                    );
                }
                if b < 2 || b > 36 {
                    bail!(base.span, "base must be between 2 and 36");
                }
                repr::format_int_with_base(n, b).into()
            }
        })
    }

    /// The length of the string in UTF-8 encoded bytes.
    #[func(title = "Length")]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Extracts the first grapheme cluster of the string.
    ///
    /// Returns the provided default value if the string is empty or fails with
    /// an error if no default value was specified.
    #[func]
    pub fn first(
        &self,
        /// A default value to return if the string is empty.
        #[named]
        default: Option<Str>,
    ) -> StrResult<Str> {
        self.0
            .graphemes(true)
            .next()
            .map(Into::into)
            .or(default)
            .ok_or_else(string_is_empty)
    }

    /// Extracts the last grapheme cluster of the string.
    ///
    /// Returns the provided default value if the string is empty or fails with
    /// an error if no default value was specified.
    #[func]
    pub fn last(
        &self,
        /// A default value to return if the string is empty.
        #[named]
        default: Option<Str>,
    ) -> StrResult<Str> {
        self.0
            .graphemes(true)
            .next_back()
            .map(Into::into)
            .or(default)
            .ok_or_else(string_is_empty)
    }

    /// Extracts the first grapheme cluster after the specified index. Returns
    /// the default value if the index is out of bounds or fails with an error
    /// if no default value was specified.
    #[func]
    pub fn at(
        &self,
        /// The byte index. If negative, indexes from the back.
        index: i64,
        /// A default value to return if the index is out of bounds.
        #[named]
        default: Option<Value>,
    ) -> StrResult<Value> {
        let len = self.len();
        self.locate_opt(index)?
            .and_then(|i| self.0[i..].graphemes(true).next().map(|s| s.into_value()))
            .or(default)
            .ok_or_else(|| no_default_and_out_of_bounds(index, len))
    }

    /// Extracts a substring of the string.
    /// Fails with an error if the start or end index is out of bounds.
    #[func]
    pub fn slice(
        &self,
        /// The start byte index (inclusive). If negative, indexes from the
        /// back.
        start: i64,
        /// The end byte index (exclusive). If omitted, the whole slice until
        /// the end of the string is extracted. If negative, indexes from the
        /// back.
        #[default]
        end: Option<i64>,
        /// The number of bytes to extract. This is equivalent to passing
        /// `start + count` as the `end` position. Mutually exclusive with `end`.
        #[named]
        count: Option<i64>,
    ) -> StrResult<Str> {
        if end.is_some() && count.is_some() {
            bail!("`end` and `count` are mutually exclusive");
        }
        let start = self.locate(start)?;
        let end = end.or(count.map(|c| start as i64 + c));
        let end = self.locate(end.unwrap_or(self.len() as i64))?.max(start);
        Ok(self.0[start..end].into())
    }

    /// Returns the grapheme clusters of the string as an array of substrings.
    #[func]
    pub fn clusters(&self) -> Array {
        self.as_str().graphemes(true).map(|s| Value::Str(s.into())).collect()
    }

    /// Returns the Unicode codepoints of the string as an array of substrings.
    #[func]
    pub fn codepoints(&self) -> Array {
        self.chars().map(|c| Value::Str(c.into())).collect()
    }

    /// Converts a character into its corresponding code point.
    ///
    /// ```example
    /// #"a".to-unicode() \
    /// #("a\u{0300}"
    ///    .codepoints()
    ///    .map(str.to-unicode))
    /// ```
    #[func]
    pub fn to_unicode(
        /// The character that should be converted.
        character: char,
    ) -> u32 {
        character as u32
    }

    /// Converts a unicode code point into its corresponding string.
    ///
    /// ```example
    /// #str.from-unicode(97)
    /// ```
    #[func]
    pub fn from_unicode(
        /// The code point that should be converted.
        value: u32,
    ) -> StrResult<Str> {
        let c: char = value
            .try_into()
            .map_err(|_| eco_format!("{value:#x} is not a valid codepoint"))?;
        Ok(c.into())
    }

    /// Normalizes the string to the given Unicode normal form.
    ///
    /// This is useful when manipulating strings containing Unicode combining
    /// characters.
    ///
    /// ```typ
    /// #assert.eq("é".normalize(form: "nfd"), "e\u{0301}")
    /// #assert.eq("ſ́".normalize(form: "nfkc"), "ś")
    /// ```
    #[func]
    pub fn normalize(
        &self,
        #[named]
        #[default(UnicodeNormalForm::Nfc)]
        form: UnicodeNormalForm,
    ) -> Str {
        match form {
            UnicodeNormalForm::Nfc => self.nfc().collect(),
            UnicodeNormalForm::Nfd => self.nfd().collect(),
            UnicodeNormalForm::Nfkc => self.nfkc().collect(),
            UnicodeNormalForm::Nfkd => self.nfkd().collect(),
        }
    }

    /// Whether the string contains the specified pattern.
    ///
    /// This method also has dedicated syntax: You can write `{"bc" in "abcd"}`
    /// instead of `{"abcd".contains("bc")}`.
    #[func]
    pub fn contains(
        &self,
        /// The pattern to search for.
        pattern: StrPattern,
    ) -> bool {
        match pattern {
            StrPattern::Str(pat) => self.0.contains(pat.as_str()),
            StrPattern::Regex(re) => re.is_match(self),
        }
    }

    /// Whether the string starts with the specified pattern.
    #[func]
    pub fn starts_with(
        &self,
        /// The pattern the string might start with.
        pattern: StrPattern,
    ) -> bool {
        match pattern {
            StrPattern::Str(pat) => self.0.starts_with(pat.as_str()),
            StrPattern::Regex(re) => re.find(self).is_some_and(|m| m.start() == 0),
        }
    }

    /// Whether the string ends with the specified pattern.
    #[func]
    pub fn ends_with(
        &self,
        /// The pattern the string might end with.
        pattern: StrPattern,
    ) -> bool {
        match pattern {
            StrPattern::Str(pat) => self.0.ends_with(pat.as_str()),
            StrPattern::Regex(re) => {
                let mut start_byte = 0;
                while let Some(mat) = re.find_at(self, start_byte) {
                    if mat.end() == self.0.len() {
                        return true;
                    }

                    // There might still be a match overlapping this one, so
                    // restart at the next code point.
                    let Some(c) = self[mat.start()..].chars().next() else { break };
                    start_byte = mat.start() + c.len_utf8();
                }
                false
            }
        }
    }

    /// Searches for the specified pattern in the string and returns the first
    /// match as a string or `{none}` if there is no match.
    #[func]
    pub fn find(
        &self,
        /// The pattern to search for.
        pattern: StrPattern,
    ) -> Option<Str> {
        match pattern {
            StrPattern::Str(pat) => self.0.contains(pat.as_str()).then_some(pat),
            StrPattern::Regex(re) => re.find(self).map(|m| m.as_str().into()),
        }
    }

    /// Searches for the specified pattern in the string and returns the index
    /// of the first match as an integer or `{none}` if there is no match.
    #[func]
    pub fn position(
        &self,
        /// The pattern to search for.
        pattern: StrPattern,
    ) -> Option<usize> {
        match pattern {
            StrPattern::Str(pat) => self.0.find(pat.as_str()),
            StrPattern::Regex(re) => re.find(self).map(|m| m.start()),
        }
    }

    /// Searches for the specified pattern in the string and returns a
    /// dictionary with details about the first match or `{none}` if there is no
    /// match.
    ///
    /// The returned dictionary has the following keys:
    /// - `start`: The start offset of the match
    /// - `end`: The end offset of the match
    /// - `text`: The text that matched.
    /// - `captures`: An array containing a string for each matched capturing
    ///   group. The first item of the array contains the first matched
    ///   capturing, not the whole match! This is empty unless the `pattern` was
    ///   a regex with capturing groups.
    ///
    /// ```example:"Shape of the returned dictionary"
    /// #let pat = regex("not (a|an) (apple|cat)")
    /// #"I'm a doctor, not an apple.".match(pat) \
    /// #"I am not a cat!".match(pat)
    /// ```
    ///
    /// ```example:"Different kinds of patterns"
    /// #assert.eq("Is there a".match("for this?"), none)
    /// #"The time of my life.".match(regex("[mit]+e"))
    /// ```
    #[func]
    pub fn match_(
        &self,
        /// The pattern to search for.
        pattern: StrPattern,
    ) -> Option<Dict> {
        match pattern {
            StrPattern::Str(pat) => {
                self.0.match_indices(pat.as_str()).next().map(match_to_dict)
            }
            StrPattern::Regex(re) => re.captures(self).map(captures_to_dict),
        }
    }

    /// Searches for the specified pattern in the string and returns an array of
    /// dictionaries with details about all matches. For details about the
    /// returned dictionaries, see [above]($str.match).
    ///
    /// ```example
    /// #"Day by Day.".matches("Day")
    /// ```
    #[func]
    pub fn matches(
        &self,
        /// The pattern to search for.
        pattern: StrPattern,
    ) -> Array {
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

    /// Replace at most `count` occurrences of the given pattern with a
    /// replacement string or function (beginning from the start). If no count
    /// is given, all occurrences are replaced.
    #[func]
    pub fn replace(
        &self,
        engine: &mut Engine,
        context: Tracked<Context>,
        /// The pattern to search for.
        pattern: StrPattern,
        /// The string to replace the matches with or a function that gets a
        /// dictionary for each match and can return individual replacement
        /// strings.
        ///
        /// The dictionary passed to the function has the same shape as the
        /// dictionary returned by [`match`]($str.match).
        replacement: Replacement,
        ///  If given, only the first `count` matches of the pattern are placed.
        #[named]
        count: Option<usize>,
    ) -> SourceResult<Str> {
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
            match &replacement {
                Replacement::Str(s) => output.push_str(s),
                Replacement::Func(func) => {
                    let piece = func
                        .call(engine, context, [dict])?
                        .cast::<Str>()
                        .at(func.span())?;
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

    /// Removes matches of a pattern from one or both sides of the string, once or
    /// repeatedly and returns the resulting string.
    #[func]
    pub fn trim(
        &self,
        /// The pattern to search for. If `{none}`, trims white spaces.
        #[default]
        pattern: Option<StrPattern>,
        /// Can be `{start}` or `{end}` to only trim the start or end of the
        /// string. If omitted, both sides are trimmed.
        #[named]
        at: Option<StrSide>,
        /// Whether to repeatedly removes matches of the pattern or just once.
        /// Defaults to `{true}`.
        #[named]
        #[default(true)]
        repeat: bool,
    ) -> Str {
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
                let mut last = None;
                let mut range = 0..s.len();

                for m in re.find_iter(s) {
                    // Does this match follow directly after the last one?
                    let consecutive = last == Some(m.start());

                    // As long as we're at the beginning or in a consecutive run
                    // of matches, and we're still trimming at the start, trim.
                    start &= m.start() == 0 || consecutive;
                    if start {
                        range.start = m.end();
                        start &= repeat;
                    }

                    // Reset end trim if we aren't consecutive anymore or aren't
                    // repeating.
                    if end && (!consecutive || !repeat) {
                        range.end = m.start();
                    }

                    last = Some(m.end());
                }

                // Is the last match directly at the end?
                if last.is_some_and(|last| last < s.len()) {
                    range.end = s.len();
                }

                &s[range.start..range.start.max(range.end)]
            }
        };

        trimmed.into()
    }

    /// Splits a string at matches of a specified pattern and returns an array
    /// of the resulting parts.
    ///
    /// When the empty string is used as a separator, it separates every
    /// character (i.e., Unicode code point) in the string, along with the
    /// beginning and end of the string. In practice, this means that the
    /// resulting list of parts will contain the empty string at the start
    /// and end of the list.
    #[func]
    pub fn split(
        &self,
        /// The pattern to split at. Defaults to whitespace.
        #[default]
        pattern: Option<StrPattern>,
    ) -> Array {
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

    /// Reverse the string.
    #[func(title = "Reverse")]
    pub fn rev(&self) -> Str {
        let mut s = EcoString::with_capacity(self.0.len());
        for grapheme in self.as_str().graphemes(true).rev() {
            s.push_str(grapheme);
        }
        s.into()
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
        Debug::fmt(self.as_str(), f)
    }
}

impl Display for Str {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(self.as_str(), f)
    }
}

impl Repr for Str {
    fn repr(&self) -> EcoString {
        self.as_ref().repr()
    }
}

impl Repr for EcoString {
    fn repr(&self) -> EcoString {
        self.as_ref().repr()
    }
}

impl Repr for str {
    fn repr(&self) -> EcoString {
        let mut r = EcoString::with_capacity(self.len() + 2);
        r.push('"');
        for c in self.chars() {
            match c {
                '\0' => r.push_str(r"\u{0}"),
                '\'' => r.push('\''),
                '"' => r.push_str(r#"\""#),
                _ => r.extend(c.escape_debug()),
            }
        }
        r.push('"');
        r
    }
}

impl Repr for char {
    fn repr(&self) -> EcoString {
        EcoString::from(*self).repr()
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

/// A value that can be cast to a string.
pub enum ToStr {
    /// A string value ready to be used as-is.
    Str(Str),
    /// An integer about to be formatted in a given base.
    Int(i64),
}

cast! {
    ToStr,
    v: i64 => Self::Int(v),
    v: f64 => Self::Str(repr::display_float(v).into()),
    v: Decimal => Self::Str(format_str!("{}", v)),
    v: Version => Self::Str(format_str!("{}", v)),
    v: Bytes => Self::Str(v.to_str().map_err(|_| "bytes are not valid UTF-8")?),
    v: Label => Self::Str(v.resolve().as_str().into()),
    v: Type => Self::Str(v.long_name().into()),
    v: Str => Self::Str(v),
}

/// Similar to `Option<i64>`, but the default value casts to `10` rather than
/// `none`, so that the right default value is documented.
#[derive(Debug, Copy, Clone)]
pub enum Base {
    Default,
    User(i64),
}

impl Base {
    pub fn value(self) -> i64 {
        match self {
            Self::Default => 10,
            Self::User(b) => b,
        }
    }
}

cast! {
    Base,
    self => self.value().into_value(),
    v: i64 => Self::User(v),
}

/// A Unicode normalization form.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum UnicodeNormalForm {
    /// Canonical composition where e.g. accented letters are turned into a
    /// single Unicode codepoint.
    #[string("nfc")]
    Nfc,
    /// Canonical decomposition where e.g. accented letters are split into a
    /// separate base and diacritic.
    #[string("nfd")]
    Nfd,
    /// Like NFC, but using the Unicode compatibility decompositions.
    #[string("nfkc")]
    Nfkc,
    /// Like NFD, but using the Unicode compatibility decompositions.
    #[string("nfkd")]
    Nfkd,
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

/// The out of bounds access error message.
#[cold]
fn out_of_bounds(index: i64, len: usize) -> EcoString {
    eco_format!("string index out of bounds (index: {}, len: {})", index, len)
}

/// The out of bounds access error message when no default value was given.
#[cold]
fn no_default_and_out_of_bounds(index: i64, len: usize) -> EcoString {
    eco_format!(
        "no default value was specified and string index out of bounds (index: {}, len: {})",
        index,
        len
    )
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

/// A regular expression.
///
/// Can be used as a [show rule selector]($styling/#show-rules) and with
/// [string methods]($str) like `find`, `split`, and `replace`.
///
/// [See here](https://docs.rs/regex/latest/regex/#syntax) for a specification
/// of the supported syntax.
///
/// # Example
/// ```example
/// // Works with string methods.
/// #"a,b;c".split(regex("[,;]"))
///
/// // Works with show rules.
/// #show regex("\\d+"): set text(red)
///
/// The numbers 1 to 10.
/// ```
#[ty(scope)]
#[derive(Debug, Clone)]
pub struct Regex(regex::Regex);

impl Regex {
    /// Create a new regular expression.
    pub fn new(re: &str) -> StrResult<Self> {
        regex::Regex::new(re).map(Self).map_err(|err| eco_format!("{err}"))
    }
}

#[scope]
impl Regex {
    /// Create a regular expression from a string.
    #[func(constructor)]
    pub fn construct(
        /// The regular expression as a string.
        ///
        /// Both Typst strings and regular expressions use backslashes for
        /// escaping. To produce a regex escape sequence that is also valid in
        /// Typst, you need to escape the backslash itself (e.g., writing
        /// `{regex("\\\\")}` for the regex `\\`). Regex escape sequences that
        /// are not valid Typst escape sequences (e.g., `\d` and `\b`) can be
        /// entered into strings directly, but it's good practice to still
        /// escape them to avoid ambiguity (i.e., `{regex("\\b\\d")}`). See the
        /// [list of valid string escape sequences]($str/#escapes).
        ///
        /// If you need many escape sequences, you can also create a raw element
        /// and extract its text to use it for your regular expressions:
        /// ``{regex(`\d+\.\d+\.\d+`.text)}``.
        regex: Spanned<Str>,
    ) -> SourceResult<Regex> {
        Self::new(&regex.v).at(regex.span)
    }
}

impl Deref for Regex {
    type Target = regex::Regex;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Repr for Regex {
    fn repr(&self) -> EcoString {
        eco_format!("regex({})", self.0.as_str().repr())
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

cast! {
    StrPattern,
    self => match self {
        Self::Str(v) => v.into_value(),
        Self::Regex(v) => v.into_value(),
    },
    v: Str => Self::Str(v),
    v: Regex => Self::Regex(v),
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
    v: Alignment => match v {
        Alignment::START => Self::Start,
        Alignment::END => Self::End,
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
    self => match self {
        Self::Str(v) => v.into_value(),
        Self::Func(v) => v.into_value(),
    },
    v: Str => Self::Str(v),
    v: Func => Self::Func(v)
}
