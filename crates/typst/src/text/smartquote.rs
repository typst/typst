use ecow::EcoString;
use unicode_segmentation::UnicodeSegmentation;

use crate::diag::{bail, StrResult};
use crate::foundations::{
    array, cast, dict, elem, Array, Dict, FromValue, Packed, PlainText, Smart, Str,
};
use crate::layout::Dir;
use crate::syntax::is_newline;
use crate::text::{Lang, Region};

/// A language-aware quote that reacts to its context.
///
/// Automatically turns into an appropriate opening or closing quote based on
/// the active [text language]($text.lang).
///
/// # Example
/// ```example
/// "This is in quotes."
///
/// #set text(lang: "de")
/// "Das ist in Anführungszeichen."
///
/// #set text(lang: "fr")
/// "C'est entre guillemets."
/// ```
///
/// # Syntax
/// This function also has dedicated syntax: The normal quote characters
/// (`'` and `"`). Typst automatically makes your quotes smart.
#[elem(name = "smartquote", PlainText)]
pub struct SmartQuoteElem {
    /// Whether this should be a double quote.
    #[default(true)]
    pub double: bool,

    /// Whether smart quotes are enabled.
    ///
    /// To disable smartness for a single quote, you can also escape it with a
    /// backslash.
    ///
    /// ```example
    /// #set smartquote(enabled: false)
    ///
    /// These are "dumb" quotes.
    /// ```
    #[default(true)]
    pub enabled: bool,

    /// Whether to use alternative quotes.
    ///
    /// Does nothing for languages that don't have alternative quotes, or if
    /// explicit quotes were set.
    ///
    /// ```example
    /// #set text(lang: "de")
    /// #set smartquote(alternative: true)
    ///
    /// "Das ist in anderen Anführungszeichen."
    /// ```
    #[default(false)]
    pub alternative: bool,

    /// The quotes to use.
    ///
    /// - When set to `{auto}`, the appropriate single quotes for the
    ///   [text language]($text.lang) will be used. This is the default.
    /// - Custom quotes can be passed as a string, array, or dictionary of either
    ///   - [string]($str): a string consisting of two characters containing the
    ///     opening and closing double quotes (characters here refer to Unicode
    ///     grapheme clusters)
    ///   - [array]: an array containing the opening and closing double quotes
    ///   - [dictionary]: an array containing the double and single quotes, each
    ///     specified as either `{auto}`, string, or array
    ///
    /// ```example
    /// #set text(lang: "de")
    /// 'Das sind normale Anführungszeichen.'
    ///
    /// #set smartquote(quotes: "()")
    /// "Das sind eigene Anführungszeichen."
    ///
    /// #set smartquote(quotes: (single: ("[[", "]]"),  double: auto))
    /// 'Das sind eigene Anführungszeichen.'
    /// ```
    #[borrowed]
    pub quotes: Smart<SmartQuoteDict>,
}

impl PlainText for Packed<SmartQuoteElem> {
    fn plain_text(&self, text: &mut EcoString) {
        if self.double.unwrap_or(true) {
            text.push_str("\"");
        } else {
            text.push_str("'");
        }
    }
}

/// State machine for smart quote substitution.
#[derive(Debug, Clone)]
pub struct SmartQuoter {
    /// How many quotes have been opened.
    quote_depth: usize,
    /// Whether an opening quote might follow.
    expect_opening: bool,
    /// Whether the last character was numeric.
    last_num: bool,
    /// The previous type of quote character, if it was an opening quote.
    prev_quote_type: Option<bool>,
}

impl SmartQuoter {
    /// Start quoting.
    pub fn new() -> Self {
        Self {
            quote_depth: 0,
            expect_opening: true,
            last_num: false,
            prev_quote_type: None,
        }
    }

    /// Process the last seen character.
    pub fn last(&mut self, c: char, is_quote: bool) {
        self.expect_opening = is_ignorable(c) || is_opening_bracket(c);
        self.last_num = c.is_numeric();
        if !is_quote {
            self.prev_quote_type = None;
        }
    }

    /// Process and substitute a quote.
    pub fn quote<'a>(
        &mut self,
        quotes: &SmartQuotes<'a>,
        double: bool,
        peeked: Option<char>,
    ) -> &'a str {
        let peeked = peeked.unwrap_or(' ');
        let mut expect_opening = self.expect_opening;
        if let Some(prev_double) = self.prev_quote_type.take() {
            if double != prev_double {
                expect_opening = true;
            }
        }

        if expect_opening {
            self.quote_depth += 1;
            self.prev_quote_type = Some(double);
            quotes.open(double)
        } else if self.quote_depth > 0
            && (peeked.is_ascii_punctuation() || is_ignorable(peeked))
        {
            self.quote_depth -= 1;
            quotes.close(double)
        } else if self.last_num {
            quotes.prime(double)
        } else {
            quotes.fallback(double)
        }
    }
}

impl Default for SmartQuoter {
    fn default() -> Self {
        Self::new()
    }
}

fn is_ignorable(c: char) -> bool {
    c.is_whitespace() || is_newline(c)
}

fn is_opening_bracket(c: char) -> bool {
    matches!(c, '(' | '{' | '[')
}

/// Decides which quotes to substitute smart quotes with.
pub struct SmartQuotes<'s> {
    /// The opening single quote.
    pub single_open: &'s str,
    /// The closing single quote.
    pub single_close: &'s str,
    /// The opening double quote.
    pub double_open: &'s str,
    /// The closing double quote.
    pub double_close: &'s str,
}

impl<'s> SmartQuotes<'s> {
    /// Create a new `Quotes` struct with the given quotes, optionally falling
    /// back to the defaults for a language and region.
    ///
    /// The language should be specified as an all-lowercase ISO 639-1 code, the
    /// region as an all-uppercase ISO 3166-alpha2 code.
    ///
    /// Currently, the supported languages are: English, Czech, Danish, German,
    /// Swiss / Liechtensteinian German, Estonian, Icelandic, Lithuanian,
    /// Latvian, Slovak, Slovenian, Spanish, Bosnian, Finnish, Swedish, French,
    /// Hungarian, Polish, Romanian, Japanese, Traditional Chinese, Russian, and
    /// Norwegian.
    ///
    /// For unknown languages, the English quotes are used as fallback.
    pub fn new(
        quotes: &'s Smart<SmartQuoteDict>,
        lang: Lang,
        region: Option<Region>,
        alternative: bool,
    ) -> Self {
        let region = region.as_ref().map(Region::as_str);

        let default = ("‘", "’", "“", "”");
        let low_high = ("‚", "‘", "„", "“");

        let (single_open, single_close, double_open, double_close) = match lang.as_str() {
            "de" if matches!(region, Some("CH" | "LI")) => match alternative {
                false => ("‹", "›", "«", "»"),
                true => low_high,
            },
            "cs" | "da" | "de" | "sk" | "sl" if alternative => ("›", "‹", "»", "«"),
            "cs" | "de" | "et" | "is" | "lt" | "lv" | "sk" | "sl" => low_high,
            "da" => ("‘", "’", "“", "”"),
            "fr" | "ru" if alternative => default,
            "fr" => ("‹\u{00A0}", "\u{00A0}›", "«\u{00A0}", "\u{00A0}»"),
            "fi" | "sv" if alternative => ("’", "’", "»", "»"),
            "bs" | "fi" | "sv" => ("’", "’", "”", "”"),
            "es" if matches!(region, Some("ES") | None) => ("“", "”", "«", "»"),
            "hu" | "pl" | "ro" => ("’", "’", "„", "”"),
            "no" | "nb" | "nn" if alternative => low_high,
            "ru" | "no" | "nb" | "nn" | "ua" => ("’", "’", "«", "»"),
            "gr" => ("‘", "’", "«", "»"),
            _ if lang.dir() == Dir::RTL => ("’", "‘", "”", "“"),
            _ => default,
        };

        fn inner_or_default<'s>(
            quotes: Smart<&'s SmartQuoteDict>,
            f: impl FnOnce(&'s SmartQuoteDict) -> Smart<&'s SmartQuoteSet>,
            default: [&'s str; 2],
        ) -> [&'s str; 2] {
            match quotes.and_then(f) {
                Smart::Auto => default,
                Smart::Custom(SmartQuoteSet { open, close }) => {
                    [open, close].map(|s| s.as_str())
                }
            }
        }

        let quotes = quotes.as_ref();
        let [single_open, single_close] =
            inner_or_default(quotes, |q| q.single.as_ref(), [single_open, single_close]);
        let [double_open, double_close] =
            inner_or_default(quotes, |q| q.double.as_ref(), [double_open, double_close]);

        Self {
            single_open,
            single_close,
            double_open,
            double_close,
        }
    }

    /// The opening quote.
    pub fn open(&self, double: bool) -> &'s str {
        if double {
            self.double_open
        } else {
            self.single_open
        }
    }

    /// The closing quote.
    pub fn close(&self, double: bool) -> &'s str {
        if double {
            self.double_close
        } else {
            self.single_close
        }
    }

    /// Which character should be used as a prime.
    pub fn prime(&self, double: bool) -> &'static str {
        if double {
            "″"
        } else {
            "′"
        }
    }

    /// Which character should be used as a fallback quote.
    pub fn fallback(&self, double: bool) -> &'static str {
        if double {
            "\""
        } else {
            "’"
        }
    }
}

/// An opening and closing quote.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct SmartQuoteSet {
    open: EcoString,
    close: EcoString,
}

cast! {
    SmartQuoteSet,
    self => array![self.open, self.close].into_value(),
    value: Array => {
        let [open, close] = array_to_set(value)?;
        Self { open, close }
    },
    value: Str => {
        let [open, close] = str_to_set(value.as_str())?;
        Self { open, close }
    },
}

fn str_to_set(value: &str) -> StrResult<[EcoString; 2]> {
    let mut iter = value.graphemes(true);
    match (iter.next(), iter.next(), iter.next()) {
        (Some(open), Some(close), None) => Ok([open.into(), close.into()]),
        _ => {
            let count = value.graphemes(true).count();
            bail!(
                "expected 2 characters, found {count} character{}",
                if count > 1 { "s" } else { "" }
            );
        }
    }
}

fn array_to_set(value: Array) -> StrResult<[EcoString; 2]> {
    let value = value.as_slice();
    if value.len() != 2 {
        bail!(
            "expected 2 quotes, found {} quote{}",
            value.len(),
            if value.len() > 1 { "s" } else { "" }
        );
    }

    let open: EcoString = value[0].clone().cast()?;
    let close: EcoString = value[1].clone().cast()?;

    Ok([open, close])
}

/// A dict of single and double quotes.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct SmartQuoteDict {
    double: Smart<SmartQuoteSet>,
    single: Smart<SmartQuoteSet>,
}

cast! {
    SmartQuoteDict,
    self => dict! { "double" => self.double, "single" => self.single }.into_value(),
    mut value: Dict => {
        let keys = ["double", "single"];

        let double = value
            .take("double")
            .ok()
            .map(FromValue::from_value)
            .transpose()?
            .unwrap_or(Smart::Auto);
        let single = value
            .take("single")
            .ok()
            .map(FromValue::from_value)
            .transpose()?
            .unwrap_or(Smart::Auto);

        value.finish(&keys)?;

        Self { single, double }
    },
    value: SmartQuoteSet => Self {
        double: Smart::Custom(value),
        single: Smart::Auto,
    },
}
