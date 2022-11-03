use typst::syntax::is_newline;

use super::{Lang, Region};

/// State machine for smart quote subtitution.
#[derive(Debug, Clone)]
pub struct Quoter {
    /// How many quotes have been opened.
    quote_depth: usize,
    /// Whether an opening quote might follow.
    expect_opening: bool,
    /// Whether the last character was numeric.
    last_num: bool,
}

impl Quoter {
    /// Start quoting.
    pub fn new() -> Self {
        Self {
            quote_depth: 0,
            expect_opening: true,
            last_num: false,
        }
    }

    /// Process the last seen character.
    pub fn last(&mut self, c: char) {
        self.expect_opening = is_ignorable(c) || is_opening_bracket(c);
        self.last_num = c.is_numeric();
    }

    /// Process and substitute a quote.
    pub fn quote<'a>(
        &mut self,
        quotes: &Quotes<'a>,
        double: bool,
        peeked: Option<char>,
    ) -> &'a str {
        let peeked = peeked.unwrap_or(' ');
        if self.expect_opening {
            self.quote_depth += 1;
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

/// Decides which quotes to subtitute smart quotes with.
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
    /// Create a new `Quotes` struct with the defaults for a language and
    /// region.
    ///
    /// The language should be specified as an all-lowercase ISO 639-1 code, the
    /// region as an all-uppercase ISO 3166-alpha2 code.
    ///
    /// Currently, the supported languages are: English, Czech, Danish, German,
    /// Swiss / Liechtensteinian German, Estonian, Icelandic, Lithuanian,
    /// Latvian, Slovak, Slovenian, Bosnian, Finnish, Swedish, French,
    /// Hungarian, Polish, Romanian, Japanese, Traditional Chinese, Russian, and
    /// Norwegian.
    ///
    /// For unknown languages, the English quotes are used.
    pub fn from_lang(lang: Lang, region: Option<Region>) -> Self {
        let region = region.as_ref().map(Region::as_str);
        let (single_open, single_close, double_open, double_close) = match lang.as_str() {
            "de" if matches!(region, Some("CH" | "LI")) => ("‹", "›", "«", "»"),
            "cs" | "da" | "de" | "et" | "is" | "lt" | "lv" | "sk" | "sl" => {
                ("‚", "‘", "„", "“")
            }
            "fr" => ("‹\u{00A0}", "\u{00A0}›", "«\u{00A0}", "\u{00A0}»"),
            "bs" | "fi" | "sv" => ("’", "’", "”", "”"),
            "hu" | "pl" | "ro" => ("’", "’", "„", "”"),
            "ru" | "no" | "nn" => ("’", "’", "«", "»"),
            _ => return Self::default(),
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
        if double { self.double_open } else { self.single_open }
    }

    /// The closing quote.
    fn close(&self, double: bool) -> &'s str {
        if double { self.double_close } else { self.single_close }
    }

    /// Which character should be used as a prime.
    fn prime(&self, double: bool) -> &'static str {
        if double { "″" } else { "′" }
    }

    /// Which character should be used as a fallback quote.
    fn fallback(&self, double: bool) -> &'static str {
        if double { "\"" } else { "’" }
    }
}

impl Default for Quotes<'_> {
    /// Returns the english quotes as default.
    fn default() -> Self {
        Self {
            single_open: "‘",
            single_close: "’",
            double_open: "“",
            double_close: "”",
        }
    }
}
