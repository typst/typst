use typst::syntax::is_newline;

use crate::prelude::*;
use typst::eval::IndexMap;

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
#[elem]
pub struct SmartquoteElem {
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
    /// Does nothing for languages that don't have alternative quotes.
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
    /// - When set to `{auto}`, the appropriate quotes for the
    ///   [text language]($text.lang) will be used. This is the default.
    /// - Custom quotes can be passed as a dictionary
    ///   - `double`: a string or array containing the opening and closing double quotes
    ///   - `single`: a string or array containing the opening and closing single quotes
    ///
    /// ```example
    /// #set text(lang: "de")
    /// "Das sind normale Anführungszeichen."
    ///
    /// #set smartquote(quotes: (single: "()"))
    /// "Das sind eigene Anführungszeichen."
    ///
    /// #set smartquote(quotes: (double: ("[[", "]]")))
    /// "Das sind eigene Anführungszeichen."
    /// ```
    #[default(Smart::Auto)]
    pub quotes: Smart<Overriden>,
}

/// State machine for smart quote substitution.
#[derive(Debug, Clone)]
pub struct Quoter {
    /// How many quotes have been opened.
    quote_depth: usize,
    /// Whether an opening quote might follow.
    expect_opening: bool,
    /// Whether the last character was numeric.
    last_num: bool,
    /// The previous type of quote character, if it was an opening quote.
    prev_quote_type: Option<bool>,
}

impl Quoter {
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
        quotes: &Quotes<'a>,
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

impl Default for Quoter {
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
pub struct Quotes<'s> {
    /// The opening single quote.
    pub single_open: &'s str,
    /// The closing single quote.
    pub single_close: &'s str,
    /// The opening double quote.
    pub double_open: &'s str,
    /// The closing double quote.
    pub double_close: &'s str,
}

impl<'s> Quotes<'s> {
    const DEFAULT_SINGLE_OPEN: &str = "‘";
    const DEFAULT_SINGLE_CLOSE: &str = "’";
    const DEFAULT_DOUBLE_OPEN: &str = "“";
    const DEFAULT_DOUBLE_CLOSE: &str = "”";

    const DEFAULT_VALUES: [&'static str; 4] = [
        Self::DEFAULT_SINGLE_OPEN,
        Self::DEFAULT_SINGLE_CLOSE,
        Self::DEFAULT_DOUBLE_OPEN,
        Self::DEFAULT_DOUBLE_CLOSE,
    ];

    /// Create a new `Quotes` struct with the defaults for a language and
    /// region.
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
    /// For unknown languages, the English quotes are used.
    pub fn from_lang(
        lang: Lang,
        region: Option<Region>,
        alternative: bool,
        overriden: &'s Smart<Overriden>,
    ) -> Self {
        let region = region.as_ref().map(Region::as_str);

        let low_high = ["‚", "‘", "„", "“"];

        let [default_single_open, default_single_close, default_double_open, default_double_close] =
            match lang.as_str() {
                "de" if matches!(region, Some("CH" | "LI")) => match alternative {
                    false => ["‹", "›", "«", "»"],
                    true => low_high,
                },
                "cs" | "da" | "de" | "sk" | "sl" if alternative => ["›", "‹", "»", "«"],
                "cs" | "da" | "de" | "et" | "is" | "lt" | "lv" | "sk" | "sl" => low_high,
                "fr" | "ru" if alternative => Self::DEFAULT_VALUES,
                "fr" => ["‹\u{00A0}", "\u{00A0}›", "«\u{00A0}", "\u{00A0}»"],
                "fi" | "sv" if alternative => ["’", "’", "»", "»"],
                "bs" | "fi" | "sv" => ["’", "’", "”", "”"],
                "es" if matches!(region, Some("ES") | None) => ["“", "”", "«", "»"],
                "hu" | "pl" | "ro" => ["’", "’", "„", "”"],
                "no" | "nb" | "nn" if alternative => low_high,
                "ru" | "no" | "nb" | "nn" | "ua" => ["’", "’", "«", "»"],
                _ if lang.dir() == Dir::RTL => ["’", "‘", "”", "“"],
                _ => Self::DEFAULT_VALUES,
            };

        let [single_open, single_close, double_open, double_close] = match overriden {
            Smart::Auto => [
                default_single_open,
                default_single_close,
                default_double_open,
                default_double_close,
            ],
            Smart::Custom(overriden) => {
                let single = overriden
                    .single
                    .as_ref()
                    .map(|[open, close]| [open.as_str(), close.as_str()])
                    .unwrap_or([default_single_open, default_single_close]);
                let double = overriden
                    .double
                    .as_ref()
                    .map(|[open, close]| [open.as_str(), close.as_str()])
                    .unwrap_or([default_double_open, default_double_close]);

                [single[0], single[1], double[0], double[1]]
            }
        };

        Self {
            single_open,
            single_close,
            double_open,
            double_close,
        }
    }

    /// The opening quote.
    fn open(&self, double: bool) -> &'s str {
        if double {
            self.double_open
        } else {
            self.single_open
        }
    }

    /// The closing quote.
    fn close(&self, double: bool) -> &'s str {
        if double {
            self.double_close
        } else {
            self.single_close
        }
    }

    /// Which character should be used as a prime.
    fn prime(&self, double: bool) -> &'static str {
        if double {
            "″"
        } else {
            "′"
        }
    }

    /// Which character should be used as a fallback quote.
    fn fallback(&self, double: bool) -> &'static str {
        if double {
            "\""
        } else {
            "’"
        }
    }
}

impl Default for Quotes<'_> {
    /// Returns the english quotes as default.
    fn default() -> Self {
        Self {
            single_open: Self::DEFAULT_SINGLE_OPEN,
            single_close: Self::DEFAULT_SINGLE_CLOSE,
            double_open: Self::DEFAULT_DOUBLE_OPEN,
            double_close: Self::DEFAULT_DOUBLE_CLOSE,
        }
    }
}

/// A set of smart quotes.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Overriden {
    single: Option<[EcoString; 2]>,
    double: Option<[EcoString; 2]>,
}

impl Fold for Overriden {
    type Output = Self;

    fn fold(self, outer: Self::Output) -> Self::Output {
        Self {
            single: self.single.or(outer.single),
            double: self.double.or(outer.double),
        }
    }
}

cast! {
    Overriden,
    self => {
        let mut dict = IndexMap::new();
        if let Some(single) = self.single {
            dict.insert(
                Str::from("single"),
                Value::Array(single.into_iter().map(IntoValue::into_value).collect())
            );
        }
        if let Some(double) = self.double {
            dict.insert(
                Str::from("double"),
                Value::Array(double.into_iter().map(IntoValue::into_value).collect())
            );
        }
        Value::Dict(dict.into())
    },
    mut values: Dict => {
        let keys = ["single", "double"];

        fn try_unwrap(value: Value) -> StrResult<[EcoString; 2]> {
            let (open, close) = match value {
                Value::Array(array) => {
                    match array.len() {
                        0 | 1 => bail!("array must contain a opening and closing quote"),
                        2 => {},
                        _ => bail!("unexpected third quote"),
                    }

                    let open = &array.as_slice()[0];
                    let close = &array.as_slice()[1];

                    (open.clone().cast()?, close.clone().cast()?)
                },
                Value::Str(string) => {
                    let mut iter = string.as_str().chars();
                    let open = iter.next();
                    let close = iter.next();

                    if iter.next().is_some() {
                        bail!("unexpected character after closing quote");
                    }

                    (open.map(EcoString::from), close.map(EcoString::from))
                },
                _ => bail!("expected string or array"),
            };

            let open = open.ok_or_else(|| eco_format!("missing opening quote"))?;
            let close = close.ok_or_else(|| eco_format!("missing closing quote"))?;

            Ok([open, close])
        }

        let single = values.take("single").ok().map(try_unwrap).transpose()?;
        let double = values.take("double").ok().map(try_unwrap).transpose()?;
        values.finish(&keys)?;

        Overriden {
            single,
            double,
        }
    },
}
