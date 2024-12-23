use std::str::FromStr;

use chinese_number::{
    from_usize_to_chinese_ten_thousand as usize_to_chinese, ChineseCase, ChineseVariant,
};
use comemo::Tracked;
use ecow::{EcoString, EcoVec};

use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{cast, func, Context, Func, Str, Value};

/// Applies a numbering to a sequence of numbers.
///
/// A numbering defines how a sequence of numbers should be displayed as
/// content. It is defined either through a pattern string or an arbitrary
/// function.
///
/// A numbering pattern consists of counting symbols, for which the actual
/// number is substituted, their prefixes, and one suffix. The prefixes and the
/// suffix are repeated as-is.
///
/// # Example
/// ```example
/// #numbering("1.1)", 1, 2, 3) \
/// #numbering("1.a.i", 1, 2) \
/// #numbering("I – 1", 12, 2) \
/// #numbering(
///   (..nums) => nums
///     .pos()
///     .map(str)
///     .join(".") + ")",
///   1, 2, 3,
/// )
/// ```
///
/// # Numbering patterns and numbering functions
/// There are multiple instances where you can provide a numbering pattern or
/// function in Typst. For example, when defining how to number
/// [headings]($heading) or [figures]($figure). Every time, the expected format
/// is the same as the one described below for the
/// [`numbering`]($numbering.numbering) parameter.
///
/// The following example illustrates that a numbering function is just a
/// regular [function] that accepts numbers and returns [`content`].
/// ```example
/// #let unary(.., last) = "|" * last
/// #set heading(numbering: unary)
/// = First heading
/// = Second heading
/// = Third heading
/// ```
#[func]
pub fn numbering(
    /// The engine.
    engine: &mut Engine,
    /// The callsite context.
    context: Tracked<Context>,
    /// Defines how the numbering works.
    ///
    /// **Counting symbols** are `1`, `a`, `A`, `i`, `I`, `α`, `Α`, `一`, `壹`,
    /// `あ`, `い`, `ア`, `イ`, `א`, `가`, `ㄱ`, `*`, `١`, `۱`, `१`, `১`, `ক`,
    /// `①`, and `⓵`. They are replaced by the number in the sequence,
    /// preserving the original case.
    ///
    /// The `*` character means that symbols should be used to count, in the
    /// order of `*`, `†`, `‡`, `§`, `¶`, `‖`. If there are more than six
    /// items, the number is represented using repeated symbols.
    ///
    /// **Suffixes** are all characters after the last counting symbol. They are
    /// repeated as-is at the end of any rendered number.
    ///
    /// **Prefixes** are all characters that are neither counting symbols nor
    /// suffixes. They are repeated as-is at in front of their rendered
    /// equivalent of their counting symbol.
    ///
    /// This parameter can also be an arbitrary function that gets each number
    /// as an individual argument. When given a function, the `numbering`
    /// function just forwards the arguments to that function. While this is not
    /// particularly useful in itself, it means that you can just give arbitrary
    /// numberings to the `numbering` function without caring whether they are
    /// defined as a pattern or function.
    numbering: Numbering,
    /// The numbers to apply the numbering to. Must be positive.
    ///
    /// If `numbering` is a pattern and more numbers than counting symbols are
    /// given, the last counting symbol with its prefix is repeated.
    #[variadic]
    numbers: Vec<usize>,
) -> SourceResult<Value> {
    numbering.apply(engine, context, &numbers)
}

/// How to number a sequence of things.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Numbering {
    /// A pattern with prefix, numbering, lower / upper case and suffix.
    Pattern(NumberingPattern),
    /// A closure mapping from an item's number to content.
    Func(Func),
}

impl Numbering {
    /// Apply the pattern to the given numbers.
    pub fn apply(
        &self,
        engine: &mut Engine,
        context: Tracked<Context>,
        numbers: &[usize],
    ) -> SourceResult<Value> {
        Ok(match self {
            Self::Pattern(pattern) => Value::Str(pattern.apply(numbers).into()),
            Self::Func(func) => func.call(engine, context, numbers.iter().copied())?,
        })
    }

    /// Trim the prefix suffix if this is a pattern.
    pub fn trimmed(mut self) -> Self {
        if let Self::Pattern(pattern) = &mut self {
            pattern.trimmed = true;
        }
        self
    }
}

impl From<NumberingPattern> for Numbering {
    fn from(pattern: NumberingPattern) -> Self {
        Self::Pattern(pattern)
    }
}

cast! {
    Numbering,
    self => match self {
        Self::Pattern(pattern) => pattern.into_value(),
        Self::Func(func) => func.into_value(),
    },
    v: NumberingPattern => Self::Pattern(v),
    v: Func => Self::Func(v),
}

/// How to turn a number into text.
///
/// A pattern consists of a prefix, followed by one of the counter symbols (see
/// [`numbering()`] docs), and then a suffix.
///
/// Examples of valid patterns:
/// - `1)`
/// - `a.`
/// - `(I)`
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct NumberingPattern {
    pub pieces: EcoVec<(EcoString, NumberingKind)>,
    pub suffix: EcoString,
    trimmed: bool,
}

impl NumberingPattern {
    /// Apply the pattern to the given number.
    pub fn apply(&self, numbers: &[usize]) -> EcoString {
        let mut fmt = EcoString::new();
        let mut numbers = numbers.iter();

        for (i, ((prefix, kind), &n)) in self.pieces.iter().zip(&mut numbers).enumerate()
        {
            if i > 0 || !self.trimmed {
                fmt.push_str(prefix);
            }
            fmt.push_str(&kind.apply(n));
        }

        for ((prefix, kind), &n) in self.pieces.last().into_iter().cycle().zip(numbers) {
            if prefix.is_empty() {
                fmt.push_str(&self.suffix);
            } else {
                fmt.push_str(prefix);
            }
            fmt.push_str(&kind.apply(n));
        }

        if !self.trimmed {
            fmt.push_str(&self.suffix);
        }

        fmt
    }

    /// Apply only the k-th segment of the pattern to a number.
    pub fn apply_kth(&self, k: usize, number: usize) -> EcoString {
        let mut fmt = EcoString::new();
        if let Some((prefix, _)) = self.pieces.first() {
            fmt.push_str(prefix);
        }
        if let Some((_, kind)) = self
            .pieces
            .iter()
            .chain(self.pieces.last().into_iter().cycle())
            .nth(k)
        {
            fmt.push_str(&kind.apply(number));
        }
        fmt.push_str(&self.suffix);
        fmt
    }

    /// How many counting symbols this pattern has.
    pub fn pieces(&self) -> usize {
        self.pieces.len()
    }
}

impl FromStr for NumberingPattern {
    type Err = &'static str;

    fn from_str(pattern: &str) -> Result<Self, Self::Err> {
        let mut chars = pattern.char_indices();
        let mut handled = 0;
        let mut start_name = 0;
        let mut pieces = EcoVec::new();
        let mut verbose = false;

        while let Some((i, c)) = chars.next() {
            match c {
                '{' if !verbose => {
                    pieces.clear();
                    handled = 0;
                    chars = pattern.char_indices();
                    verbose = true;
                }
                '{' => {
                    start_name = i;
                }
                '}' => {
                    let name: EcoString = pattern[start_name + 1..i].into();
                    let Some(kind) = NumberingKind::from_name(&name) else {
                        continue;
                    };
                    let prefix = pattern[handled..start_name].into();
                    pieces.push((prefix, kind));
                    handled = i + 1;
                }
                _ if !verbose => {
                    let Some(kind) = NumberingKind::from_char(c) else {
                        continue;
                    };

                    let prefix = pattern[handled..i].into();
                    pieces.push((prefix, kind));
                    handled = c.len_utf8() + i;
                }
                _ => continue,
            }
        }

        let suffix = pattern[handled..].into();
        if pieces.is_empty() {
            return Err("invalid numbering pattern");
        }

        Ok(Self { pieces, suffix, trimmed: false })
    }
}

cast! {
    NumberingPattern,
    self => {
        let mut pat = EcoString::new();
        for (prefix, kind) in &self.pieces {
            pat.push_str(prefix);
            pat.push_str(kind.to_name());
        }
        pat.push_str(&self.suffix);
        pat.into_value()
    },
    v: Str => v.parse()?,
}

/// Different kinds of numberings.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum NumberingKind {
    /// Adlam
    Adlam,

    /// Arabic
    ArabicIndic,
    ArabicAbjad,
    Kashmiri,
    MaghrebiAbjad,
    Persian,

    LowerLatin,
    UpperLatin,
    LowerRoman,
    UpperRoman,

    /// Greek
    GreekLowerModern,
    GreekUpperModern,
    GreekLowerAncient,
    GreekUpperAncient,
    LowerGreek,
    UpperGreek,

    /// Cyrillic
    LowerBelorussian,
    UpperBelorussian,
    LowerBulgarian,
    UpperBulgarian,
    LowerMacedonian,
    UpperMacedonian,
    LowerRussian,
    UpperRussian,
    LowerRussianFull,
    UpperRussianFull,
    LowerSerbian,
    UpperSerbian,
    LowerUkrainian,
    UpperUkrainian,
    LowerUkrainianFull,
    UpperUkrainianFull,

    Symbol,
    /// Hebrew numerals, including Geresh/Gershayim.
    Hebrew,
    /// Simplified Chinese standard numerals. This corresponds to the
    LowerSimplifiedChinese,
    /// Simplified Chinese "banknote" numerals. This corresponds to the
    UpperSimplifiedChinese,
    /// Traditional Chinese standard numerals. This corresponds to the
    LowerTraditionalChinese,
    /// Traditional Chinese "banknote" numerals. This corresponds to the
    UpperTraditionalChinese,
    /// Hiragana in the gojūon order. Includes n but excludes wi and we.
    HiraganaAiueo,
    /// Hiragana in the iroha order. Includes wi and we but excludes n.
    HiraganaIroha,
    /// Katakana in the gojūon order. Includes n but excludes wi and we.
    KatakanaAiueo,
    /// Katakana in the iroha order. Includes wi and we but excludes n.
    KatakanaIroha,
    /// Korean jamo (ㄱ, ㄴ, ㄷ, etc.).
    KoreanJamo,
    /// Korean syllables (가, 나, 다, etc.).
    KoreanSyllable,
    /// Bengali letters (ক, খ, গ, ...কক, কখ etc.).
    BengaliLetter,

    /// European Digits, Roman, etc.
    CircledDecimal,
    Decimal,
    DoubleCircledDecimal,
    FilledCircledDecimal,
    TallyMark,
}

impl NumberingKind {
    /// Create a numbering kind from a representative character.
    pub fn from_char(c: char) -> Option<Self> {
        Some(match c {
            '1' => NumberingKind::Decimal,
            'a' => NumberingKind::LowerLatin,
            'A' => NumberingKind::UpperLatin,
            'i' => NumberingKind::LowerRoman,
            'I' => NumberingKind::UpperRoman,
            'α' => NumberingKind::LowerGreek,
            'Α' => NumberingKind::UpperGreek,
            '*' => NumberingKind::Symbol,
            'א' => NumberingKind::Hebrew,
            '一' => NumberingKind::LowerSimplifiedChinese,
            '壹' => NumberingKind::UpperSimplifiedChinese,
            'あ' => NumberingKind::HiraganaAiueo,
            'い' => NumberingKind::HiraganaIroha,
            'ア' => NumberingKind::KatakanaAiueo,
            'イ' => NumberingKind::KatakanaIroha,
            'ㄱ' => NumberingKind::KoreanJamo,
            '가' => NumberingKind::KoreanSyllable,
            '\u{0995}' => NumberingKind::BengaliLetter,
            '①' => NumberingKind::CircledDecimal,
            '⓵' => NumberingKind::DoubleCircledDecimal,
            _ => return None,
        })
    }

    /// Create a numbering kind from a name.
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "adlam" => NumberingKind::Adlam,
            "arabic-indic" => NumberingKind::ArabicIndic,
            "arabic-abjad" => NumberingKind::ArabicAbjad,
            "kashmiri" => NumberingKind::Kashmiri,
            "maghrebi-abjad" => NumberingKind::MaghrebiAbjad,
            "persian" => NumberingKind::Persian,

            "latin" => NumberingKind::LowerLatin,
            "Latin" => NumberingKind::UpperLatin,
            "roman" => NumberingKind::LowerRoman,
            "Roman" => NumberingKind::UpperRoman,

            "greek-lower-modern" => NumberingKind::GreekLowerModern,
            "greek-upper-modern" => NumberingKind::GreekUpperModern,
            "greek-lower-ancient" => NumberingKind::GreekLowerAncient,
            "greek-upper-ancient" => NumberingKind::GreekUpperAncient,
            "lower-greek" => NumberingKind::LowerGreek,
            "upper-greek" => NumberingKind::UpperGreek,

            "lower-belorussian" => NumberingKind::LowerBelorussian,
            "upper-belorussian" => NumberingKind::UpperBelorussian,
            "lower-bulgarian" => NumberingKind::LowerBulgarian,
            "upper-bulgarian" => NumberingKind::UpperBulgarian,
            "lower-macedonian" => NumberingKind::LowerMacedonian,
            "upper-macedonian" => NumberingKind::UpperMacedonian,
            "lower-russian" => NumberingKind::LowerRussian,
            "upper-russian" => NumberingKind::UpperRussian,
            "lower-russian-full" => NumberingKind::LowerRussianFull,
            "upper-russian-full" => NumberingKind::UpperRussianFull,
            "lower-serbian" => NumberingKind::LowerSerbian,
            "upper-serbian" => NumberingKind::UpperSerbian,
            "lower-ukrainian" => NumberingKind::LowerUkrainian,
            "upper-ukrainian" => NumberingKind::UpperUkrainian,
            "lower-ukrainian-full" => NumberingKind::LowerUkrainianFull,
            "upper-ukrainian-full" => NumberingKind::UpperUkrainianFull,

            "symbol" => NumberingKind::Symbol,
            "hebrew" => NumberingKind::Hebrew,
            "chinese-simplified" => NumberingKind::LowerSimplifiedChinese,
            "Chinese-simplified" => NumberingKind::UpperSimplifiedChinese,
            "chinese-traditional" => NumberingKind::LowerTraditionalChinese,
            "Chinese-traditional" => NumberingKind::UpperTraditionalChinese,
            "hiragana" => NumberingKind::HiraganaAiueo,
            "hiragana-iroha" => NumberingKind::HiraganaIroha,
            "katakana" => NumberingKind::KatakanaAiueo,
            "katakana-iroha" => NumberingKind::KatakanaIroha,
            "korean" => NumberingKind::KoreanJamo,
            "korean-syllable" => NumberingKind::KoreanSyllable,
            "bengali-letter" => NumberingKind::BengaliLetter,

            "circled-decimal" => NumberingKind::CircledDecimal,
            "decimal" => NumberingKind::Decimal,
            "doubled-circled-decimal" => NumberingKind::DoubleCircledDecimal,
            "filled-circled-decimal" => NumberingKind::FilledCircledDecimal,
            "tally-mark" => NumberingKind::TallyMark,
            _ => return None,
        })
    }

    /// The name for this numbering kind.
    pub fn to_name(self) -> &'static str {
        match self {
            Self::Adlam => "adlam",
            Self::ArabicIndic => "arabic-indic",
            Self::ArabicAbjad => "arabic-abjad",
            Self::Kashmiri => "kashmiri",
            Self::MaghrebiAbjad => "maghrebi-abjad",
            Self::Persian => "persian",

            Self::LowerLatin => "latin",
            Self::UpperLatin => "Latin",
            Self::LowerRoman => "roman",
            Self::UpperRoman => "Roman",

            Self::GreekLowerModern => "greek-lower-modern",
            Self::GreekUpperModern => "greek-upper-modern",
            Self::GreekLowerAncient => "greek-lower-ancient",
            Self::GreekUpperAncient => "greek-upper-ancient",
            Self::LowerGreek => "lower-greek",
            Self::UpperGreek => "upper-greek",

            Self::LowerBelorussian => "lower-belorussian",
            Self::UpperBelorussian => "upper-belorussian",
            Self::LowerBulgarian => "lower-bulgarian",
            Self::UpperBulgarian => "upper-bulgarian",
            Self::LowerMacedonian => "lower-macedonian",
            Self::UpperMacedonian => "upper-macedonian",
            Self::LowerRussian => "lower-russian",
            Self::UpperRussian => "upper-russian",
            Self::LowerRussianFull => "lower-russian-full",
            Self::UpperRussianFull => "upper-russian-full",
            Self::LowerSerbian => "lower-serbian",
            Self::UpperSerbian => "upper-serbian",
            Self::LowerUkrainian => "lower-ukrainian",
            Self::UpperUkrainian => "upper-ukrainian",
            Self::LowerUkrainianFull => "lower-ukrainian-full",
            Self::UpperUkrainianFull => "upper-ukrainian-full",

            Self::Symbol => "symbol",
            Self::Hebrew => "hebrew",
            Self::LowerSimplifiedChinese => "chinese-simplified",
            Self::UpperSimplifiedChinese => "Chinese-simplified",
            Self::LowerTraditionalChinese => "chinese-traditional",
            Self::UpperTraditionalChinese => "Chinese-traditional",
            Self::HiraganaAiueo => "hiragana",
            Self::HiraganaIroha => "hiragana-iroha",
            Self::KatakanaAiueo => "katakana",
            Self::KatakanaIroha => "katakana-iroha",
            Self::KoreanJamo => "korean",
            Self::KoreanSyllable => "korean-syllable",
            Self::BengaliLetter => "bengali-letter",

            Self::CircledDecimal => "circled-decimal",
            Self::Decimal => "decimal",
            Self::DoubleCircledDecimal => "doubled-circled-decimal",
            Self::FilledCircledDecimal => "filled-circled-decimal",
            Self::TallyMark => "tally-mark",
        }
    }

    /// Apply the numbering to the given number.
    pub fn apply(self, n: usize) -> EcoString {
        match self {
            Self::Adlam => numeric(['𞥐', '𞥑', '𞥒', '𞥓', '𞥔', '𞥕', '𞥖', '𞥗', '𞥘', '𞥙'], n),
            Self::ArabicIndic => {
                numeric(['٠', '١', '٢', '٣', '٤', '٥', '٦', '٧', '٨', '٩'], n)
            }
            Self::ArabicAbjad => fixed(
                [
                    'ا', 'ب', 'ج', 'د', 'ه', 'و', 'ز', 'ح', 'ط', 'ي', 'ك', 'ل', 'م', 'ن',
                    'س', 'ع', 'ف', 'ص', 'ق', 'ر', 'ش', 'ت', 'ث', 'خ', 'ذ', 'ض', 'ظ', 'غ',
                ],
                n,
            ),
            Self::Kashmiri => alphabetic(
                [
                    'ا', 'آ', 'ب', 'پ', 'ت', 'ٹ', 'ث', 'ج', 'چ', 'ح', 'خ', 'د', 'ڈ', 'ذ',
                    'ر', 'ڑ', 'ز', 'ژ', 'س', 'ش', 'ص', 'ض', 'ط', 'ظ', 'ع', 'غ', 'ف', 'ق',
                    'ک', 'گ', 'ل', 'م', 'ن', 'ں', 'و', 'ہ', 'ھ', 'ء', 'ی', 'ے', 'ۄ', 'ؠ',
                ],
                n,
            ),
            Self::MaghrebiAbjad => fixed(
                [
                    'ا', 'ب', 'ج', 'د', 'ه', 'و', 'ز', 'ح', 'ط', 'ي', 'ك', 'ل', 'م', 'ن',
                    'ص', 'ع', 'ف', 'ض', 'ق', 'ر', 'س', 'ت', 'ث', 'خ', 'ذ', 'ظ', 'غ', 'ش',
                ],
                n,
            ),
            Self::Persian => {
                numeric(['۰', '۱', '۲', '۳', '۴', '۵', '۶', '۷', '۸', '۹'], n)
            }
            Self::LowerRoman => additive(
                [
                    (&"m", 1000),
                    (&"cm", 900),
                    (&"d", 500),
                    (&"cd", 400),
                    (&"c", 100),
                    (&"xc", 90),
                    (&"l", 50),
                    (&"xl", 40),
                    (&"x", 10),
                    (&"ix", 9),
                    (&"v", 5),
                    (&"iv", 4),
                    (&"i", 1),
                ],
                n,
            ),
            Self::UpperRoman => additive(
                [
                    (&"M", 1000),
                    (&"CM", 900),
                    (&"D", 500),
                    (&"CD", 400),
                    (&"C", 100),
                    (&"XC", 90),
                    (&"L", 50),
                    (&"XL", 40),
                    (&"X", 10),
                    (&"IX", 9),
                    (&"V", 5),
                    (&"IV", 4),
                    (&"I", 1),
                ],
                n,
            ),
            Self::GreekLowerModern => additive(
                [
                    (&"ϡ", 900),
                    (&"ω", 800),
                    (&"ψ", 700),
                    (&"χ", 600),
                    (&"φ", 500),
                    (&"υ", 400),
                    (&"τ", 300),
                    (&"σ", 200),
                    (&"ρ", 100),
                    (&"ϟ", 90),
                    (&"π", 80),
                    (&"ο", 70),
                    (&"ξ", 60),
                    (&"ν", 50),
                    (&"μ", 40),
                    (&"λ", 30),
                    (&"κ", 20),
                    (&"ι", 10),
                    (&"θ", 9),
                    (&"η", 8),
                    (&"ζ", 7),
                    (&"στ", 6),
                    (&"ε", 5),
                    (&"δ", 4),
                    (&"γ", 3),
                    (&"β", 2),
                    (&"α", 1),
                    (&"𐆊", 0),
                ],
                n,
            ),
            Self::GreekUpperModern => additive(
                [
                    (&"Ϡ", 900),
                    (&"Ω", 800),
                    (&"Ψ", 700),
                    (&"Χ", 600),
                    (&"Φ", 500),
                    (&"Υ", 400),
                    (&"Τ", 300),
                    (&"Σ", 200),
                    (&"Ρ", 100),
                    (&"Ϟ", 90),
                    (&"Π", 80),
                    (&"Ο", 70),
                    (&"Ξ", 60),
                    (&"Ν", 50),
                    (&"Μ", 40),
                    (&"Λ", 30),
                    (&"Κ", 20),
                    (&"Ι", 10),
                    (&"Θ", 9),
                    (&"Η", 8),
                    (&"Ζ", 7),
                    (&"ΣΤ", 6),
                    (&"Ε", 5),
                    (&"Δ", 4),
                    (&"Γ", 3),
                    (&"Β", 2),
                    (&"Α", 1),
                    (&"𐆊", 0),
                ],
                n,
            ),
            Self::GreekLowerAncient => additive(
                [
                    (&"ϡ", 900),
                    (&"ω", 800),
                    (&"ψ", 700),
                    (&"χ", 600),
                    (&"φ", 500),
                    (&"υ", 400),
                    (&"τ", 300),
                    (&"σ", 200),
                    (&"ρ", 100),
                    (&"ϟ", 90),
                    (&"π", 80),
                    (&"ο", 70),
                    (&"ξ", 60),
                    (&"ν", 50),
                    (&"μ", 40),
                    (&"λ", 30),
                    (&"κ", 20),
                    (&"ι", 10),
                    (&"θ", 9),
                    (&"η", 8),
                    (&"ζ", 7),
                    (&"ϛ", 6),
                    (&"ε", 5),
                    (&"δ", 4),
                    (&"γ", 3),
                    (&"β", 2),
                    (&"α", 1),
                    (&"𐆊", 0),
                ],
                n,
            ),
            Self::GreekUpperAncient => additive(
                [
                    (&"Ϡ", 900),
                    (&"Ω", 800),
                    (&"Ψ", 700),
                    (&"Χ", 600),
                    (&"Φ", 500),
                    (&"Υ", 400),
                    (&"Τ", 300),
                    (&"Σ", 200),
                    (&"Ρ", 100),
                    (&"Ϟ", 90),
                    (&"Π", 80),
                    (&"Ο", 70),
                    (&"Ξ", 60),
                    (&"Ν", 50),
                    (&"Μ", 40),
                    (&"Λ", 30),
                    (&"Κ", 20),
                    (&"Ι", 10),
                    (&"Θ", 9),
                    (&"Η", 8),
                    (&"Ζ", 7),
                    (&"Ϛ", 6),
                    (&"Ε", 5),
                    (&"Δ", 4),
                    (&"Γ", 3),
                    (&"Β", 2),
                    (&"Α", 1),
                    (&"𐆊", 0),
                ],
                n,
            ),
            Self::LowerGreek => alphabetic(
                [
                    'α', 'β', 'γ', 'δ', 'ε', 'ζ', 'η', 'θ', 'ι', 'κ', 'λ', 'μ', 'ν', 'ξ',
                    'ο', 'π', 'ρ', 'σ', 'τ', 'υ', 'φ', 'χ', 'ψ', 'ω',
                ],
                n,
            ),
            Self::UpperGreek => alphabetic(
                [
                    'Α', 'Β', 'Γ', 'Δ', 'Ε', 'Ζ', 'Η', 'Θ', 'Ι', 'Κ', 'Λ', 'Μ', 'Ν', 'Ξ',
                    'Ο', 'Π', 'Ρ', 'Σ', 'Τ', 'Υ', 'Φ', 'Χ', 'Ψ', 'Ω',
                ],
                n,
            ),

            Self::LowerBelorussian => alphabetic(
                [
                    'а', 'б', 'в', 'г', 'д', 'е', 'ё', 'ж', 'з', 'і', 'й', 'к', 'л', 'м',
                    'н', 'о', 'п', 'р', 'с', 'т', 'у', 'ў', 'ф', 'х', 'ц', 'ч', 'ш', 'ы',
                    'ь', 'э', 'ю', 'я',
                ],
                n,
            ),
            Self::UpperBelorussian => alphabetic(
                [
                    'А', 'Б', 'В', 'Г', 'Д', 'Е', 'Ё', 'Ж', 'З', 'І', 'Й', 'К', 'Л', 'М',
                    'Н', 'О', 'П', 'Р', 'С', 'Т', 'У', 'Ў', 'Ф', 'Х', 'Ц', 'Ч', 'Ш', 'Ы',
                    'Ь', 'Э', 'Ю', 'Я',
                ],
                n,
            ),
            Self::LowerBulgarian => alphabetic(
                [
                    'а', 'б', 'в', 'г', 'д', 'е', 'ж', 'з', 'и', 'й', 'к', 'л', 'м', 'н',
                    'о', 'п', 'р', 'с', 'т', 'у', 'ф', 'х', 'ц', 'ч', 'ш', 'щ', 'ъ', 'ь',
                    'ю', 'я',
                ],
                n,
            ),
            Self::UpperBulgarian => alphabetic(
                [
                    'А', 'Б', 'В', 'Г', 'Д', 'Е', 'Ж', 'З', 'И', 'Й', 'К', 'Л', 'М', 'Н',
                    'О', 'П', 'Р', 'С', 'Т', 'У', 'Ф', 'Х', 'Ц', 'Ч', 'Ш', 'Щ', 'Ъ', 'Ь',
                    'Ю', 'Я',
                ],
                n,
            ),
            Self::LowerMacedonian => alphabetic(
                [
                    'а', 'б', 'в', 'г', 'д', 'ѓ', 'е', 'ж', 'з', 'ѕ', 'и', 'ј', 'к', 'л',
                    'љ', 'м', 'н', 'њ', 'о', 'п', 'р', 'с', 'т', 'ќ', 'у', 'ф', 'х', 'ц',
                    'ч', 'џ', 'ш',
                ],
                n,
            ),
            Self::UpperMacedonian => alphabetic(
                [
                    'А', 'Б', 'В', 'Г', 'Д', 'Ѓ', 'Е', 'Ж', 'З', 'Ѕ', 'И', 'Ј', 'К', 'Л',
                    'Љ', 'М', 'Н', 'Њ', 'О', 'П', 'Р', 'С', 'Т', 'Ќ', 'У', 'Ф', 'Х', 'Ц',
                    'Ч', 'Џ', 'Ш',
                ],
                n,
            ),
            Self::LowerRussian => alphabetic(
                [
                    'а', 'б', 'в', 'г', 'д', 'е', 'ж', 'з', 'и', 'к', 'л', 'м', 'н', 'о',
                    'п', 'р', 'с', 'т', 'у', 'ф', 'х', 'ц', 'ч', 'ш', 'щ', 'э', 'ю', 'я',
                ],
                n,
            ),
            Self::UpperRussian => alphabetic(
                [
                    'А', 'Б', 'В', 'Г', 'Д', 'Е', 'Ж', 'З', 'И', 'К', 'Л', 'М', 'Н', 'О',
                    'П', 'Р', 'С', 'Т', 'У', 'Ф', 'Х', 'Ц', 'Ч', 'Ш', 'Щ', 'Э', 'Ю', 'Я',
                ],
                n,
            ),
            Self::LowerRussianFull => alphabetic(
                [
                    'а', 'б', 'в', 'г', 'д', 'е', 'ё', 'ж', 'з', 'и', 'й', 'к', 'л', 'м',
                    'н', 'о', 'п', 'р', 'с', 'т', 'у', 'ф', 'х', 'ц', 'ч', 'ш', 'щ', 'ъ',
                    'ы', 'ь', 'э', 'ю', 'я',
                ],
                n,
            ),
            Self::UpperRussianFull => alphabetic(
                [
                    'А', 'Б', 'В', 'Г', 'Д', 'Е', 'Ё', 'Ж', 'З', 'И', 'Й', 'К', 'Л', 'М',
                    'Н', 'О', 'П', 'Р', 'С', 'Т', 'У', 'Ф', 'Х', 'Ц', 'Ч', 'Ш', 'Щ', 'Ъ',
                    'Ы', 'Ь', 'Э', 'Ю', 'Я',
                ],
                n,
            ),
            Self::LowerSerbian => alphabetic(
                [
                    'а', 'б', 'в', 'г', 'д', 'ђ', 'е', 'ж', 'з', 'и', 'ј', 'к', 'л', 'љ',
                    'м', 'н', 'њ', 'о', 'п', 'р', 'с', 'т', 'ћ', 'у', 'ф', 'х', 'ц', 'ч',
                    'џ', 'ш',
                ],
                n,
            ),
            Self::UpperSerbian => alphabetic(
                [
                    'А', 'Б', 'В', 'Г', 'Д', 'Ђ', 'Е', 'Ж', 'З', 'И', 'Ј', 'К', 'Л', 'Љ',
                    'М', 'Н', 'Њ', 'О', 'П', 'Р', 'С', 'Т', 'Ћ', 'У', 'Ф', 'Х', 'Ц', 'Ч',
                    'Џ', 'Ш',
                ],
                n,
            ),
            Self::LowerUkrainian => alphabetic(
                [
                    'а', 'б', 'в', 'г', 'д', 'е', 'є', 'ж', 'з', 'и', 'і', 'к', 'л', 'м',
                    'н', 'о', 'п', 'р', 'с', 'т', 'у', 'ф', 'х', 'ц', 'ч', 'ш', 'ю', 'я',
                ],
                n,
            ),
            Self::UpperUkrainian => alphabetic(
                [
                    'А', 'Б', 'В', 'Г', 'Д', 'Е', 'Є', 'Ж', 'З', 'И', 'І', 'К', 'Л', 'М',
                    'Н', 'О', 'П', 'Р', 'С', 'Т', 'У', 'Ф', 'Х', 'Ц', 'Ч', 'Ш', 'Ю', 'Я',
                ],
                n,
            ),
            Self::LowerUkrainianFull => alphabetic(
                [
                    'а', 'б', 'в', 'г', 'ґ', 'д', 'е', 'є', 'ж', 'з', 'и', 'і', 'ї', 'й',
                    'к', 'л', 'м', 'н', 'о', 'п', 'р', 'с', 'т', 'у', 'ф', 'х', 'ц', 'ч',
                    'ш', 'щ', 'ь', 'ю', 'я',
                ],
                n,
            ),
            Self::UpperUkrainianFull => alphabetic(
                [
                    'А', 'Б', 'В', 'Г', 'Ґ', 'Д', 'Е', 'Є', 'Ж', 'З', 'И', 'І', 'Ї', 'Й',
                    'К', 'Л', 'М', 'Н', 'О', 'П', 'Р', 'С', 'Т', 'У', 'Ф', 'Х', 'Ц', 'Ч',
                    'Ш', 'Щ', 'Ь', 'Ю', 'Я',
                ],
                n,
            ),

            Self::Symbol => symbolic(['*', '†', '‡', '§', '¶', '‖'], n),

            Self::Hebrew => additive(
                [
                    (&"א׳", 1000),
                    (&"ת", 400),
                    (&"ש", 300),
                    (&"ר", 200),
                    (&"ק", 100),
                    (&"צ", 90),
                    (&"פ", 80),
                    (&"ע", 70),
                    (&"ס", 60),
                    (&"נ", 50),
                    (&"מ", 40),
                    (&"ל", 30),
                    (&"כ", 20),
                    (&"יט", 19),
                    (&"יח", 18),
                    (&"יז", 17),
                    (&"טז", 16),
                    (&"טו", 15),
                    (&"י", 10),
                    (&"ט", 9),
                    (&"ח", 8),
                    (&"ז", 7),
                    (&"ו", 6),
                    (&"ה", 5),
                    (&"ד", 4),
                    (&"ג", 3),
                    (&"ב", 2),
                    (&"א", 1),
                ],
                n,
            ),

            Self::LowerLatin => alphabetic(
                [
                    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n',
                    'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
                ],
                n,
            ),
            Self::UpperLatin => alphabetic(
                [
                    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N',
                    'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
                ],
                n,
            ),
            Self::HiraganaAiueo => alphabetic(
                [
                    'あ', 'い', 'う', 'え', 'お', 'か', 'き', 'く', 'け', 'こ', 'さ',
                    'し', 'す', 'せ', 'そ', 'た', 'ち', 'つ', 'て', 'と', 'な', 'に',
                    'ぬ', 'ね', 'の', 'は', 'ひ', 'ふ', 'へ', 'ほ', 'ま', 'み', 'む',
                    'め', 'も', 'や', 'ゆ', 'よ', 'ら', 'り', 'る', 'れ', 'ろ', 'わ',
                    'を', 'ん',
                ],
                n,
            ),
            Self::HiraganaIroha => alphabetic(
                [
                    'い', 'ろ', 'は', 'に', 'ほ', 'へ', 'と', 'ち', 'り', 'ぬ', 'る',
                    'を', 'わ', 'か', 'よ', 'た', 'れ', 'そ', 'つ', 'ね', 'な', 'ら',
                    'む', 'う', 'ゐ', 'の', 'お', 'く', 'や', 'ま', 'け', 'ふ', 'こ',
                    'え', 'て', 'あ', 'さ', 'き', 'ゆ', 'め', 'み', 'し', 'ゑ', 'ひ',
                    'も', 'せ', 'す',
                ],
                n,
            ),
            Self::KatakanaAiueo => alphabetic(
                [
                    'ア', 'イ', 'ウ', 'エ', 'オ', 'カ', 'キ', 'ク', 'ケ', 'コ', 'サ',
                    'シ', 'ス', 'セ', 'ソ', 'タ', 'チ', 'ツ', 'テ', 'ト', 'ナ', 'ニ',
                    'ヌ', 'ネ', 'ノ', 'ハ', 'ヒ', 'フ', 'ヘ', 'ホ', 'マ', 'ミ', 'ム',
                    'メ', 'モ', 'ヤ', 'ユ', 'ヨ', 'ラ', 'リ', 'ル', 'レ', 'ロ', 'ワ',
                    'ヲ', 'ン',
                ],
                n,
            ),
            Self::KatakanaIroha => alphabetic(
                [
                    'イ', 'ロ', 'ハ', 'ニ', 'ホ', 'ヘ', 'ト', 'チ', 'リ', 'ヌ', 'ル',
                    'ヲ', 'ワ', 'カ', 'ヨ', 'タ', 'レ', 'ソ', 'ツ', 'ネ', 'ナ', 'ラ',
                    'ム', 'ウ', 'ヰ', 'ノ', 'オ', 'ク', 'ヤ', 'マ', 'ケ', 'フ', 'コ',
                    'エ', 'テ', 'ア', 'サ', 'キ', 'ユ', 'メ', 'ミ', 'シ', 'ヱ', 'ヒ',
                    'モ', 'セ', 'ス',
                ],
                n,
            ),
            Self::KoreanJamo => alphabetic(
                [
                    'ㄱ', 'ㄴ', 'ㄷ', 'ㄹ', 'ㅁ', 'ㅂ', 'ㅅ', 'ㅇ', 'ㅈ', 'ㅊ', 'ㅋ',
                    'ㅌ', 'ㅍ', 'ㅎ',
                ],
                n,
            ),
            Self::KoreanSyllable => alphabetic(
                [
                    '가', '나', '다', '라', '마', '바', '사', '아', '자', '차', '카',
                    '타', '파', '하',
                ],
                n,
            ),
            Self::BengaliLetter => alphabetic(
                [
                    'ক', 'খ', 'গ', 'ঘ', 'ঙ', 'চ', 'ছ', 'জ', 'ঝ', 'ঞ', 'ট', 'ঠ', 'ড', 'ঢ',
                    'ণ', 'ত', 'থ', 'দ', 'ধ', 'ন', 'প', 'ফ', 'ব', 'ভ', 'ম', 'য', 'র', 'ল',
                    'শ', 'ষ', 'স', 'হ',
                ],
                n,
            ),

            Self::LowerSimplifiedChinese => {
                usize_to_chinese(ChineseVariant::Simple, ChineseCase::Lower, n).into()
            }
            Self::UpperSimplifiedChinese => {
                usize_to_chinese(ChineseVariant::Simple, ChineseCase::Upper, n).into()
            }
            Self::LowerTraditionalChinese => {
                usize_to_chinese(ChineseVariant::Traditional, ChineseCase::Lower, n)
                    .into()
            }
            Self::UpperTraditionalChinese => {
                usize_to_chinese(ChineseVariant::Traditional, ChineseCase::Upper, n)
                    .into()
            }

            Self::CircledDecimal => fixed(
                [
                    '①', '②', '③', '④', '⑤', '⑥', '⑦', '⑧', '⑨', '⑩', '⑪', '⑫', '⑬', '⑭',
                    '⑮', '⑯', '⑰', '⑱', '⑲', '⑳', '㉑', '㉒', '㉓', '㉔', '㉕', '㉖',
                    '㉗', '㉘', '㉙', '㉚', '㉛', '㉜', '㉝', '㉞', '㉟', '㊱', '㊲',
                    '㊳', '㊴', '㊵', '㊶', '㊷', '㊸', '㊹', '㊺', '㊻', '㊼', '㊽',
                    '㊾', '㊿',
                ],
                n,
            ),
            Self::Decimal => {
                numeric(['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'], n)
            }
            Self::DoubleCircledDecimal => {
                fixed(['⓵', '⓶', '⓷', '⓸', '⓹', '⓺', '⓻', '⓼', '⓽', '⓾'], n)
            }
            Self::FilledCircledDecimal => fixed(
                [
                    '❶', '❷', '❸', '❹', '❺', '❻', '❼', '❽', '❾', '❿', '⓫', '⓬', '⓭', '⓮',
                    '⓯', '⓰', '⓱', '⓲', '⓳', '⓴',
                ],
                n,
            ),
            Self::TallyMark => additive([(&"𝍸", 5), (&"𝍷", 1)], n),
        }
    }
}

fn additive<const N_DIGITS: usize>(
    symbols: [(&str, usize); N_DIGITS],
    mut n: usize,
) -> EcoString {
    if n == 0 {
        for (symbol, weight) in symbols {
            if weight == 0 {
                return (*symbol).into();
            }
        }
        return '0'.into();
    }

    let mut s = EcoString::new();
    for (symbol, weight) in symbols {
        if weight == 0 || weight > n {
            continue;
        }
        let reps = n / weight;
        for _ in 0..reps {
            s.push_str(symbol);
        }

        n -= weight * reps;
        if n == 0 {
            return s;
        }
    }
    s
}

fn alphabetic<const N_DIGITS: usize>(
    symbols: [char; N_DIGITS],
    mut n: usize,
) -> EcoString {
    let mut s = EcoString::new();
    while n != 0 {
        n -= 1;
        s.push(symbols[n % N_DIGITS]);
        n /= N_DIGITS;
    }
    s.chars().rev().collect()
}

fn fixed<const N_DIGITS: usize>(symbols: [char; N_DIGITS], n: usize) -> EcoString {
    if n - 1 > N_DIGITS {
        return "{n}".into();
    }
    symbols[n - 1].into()
}

fn numeric<const N_DIGITS: usize>(symbols: [char; N_DIGITS], mut n: usize) -> EcoString {
    if n == 0 {
        return symbols[0].into();
    }
    let mut s = EcoString::new();
    while n != 0 {
        s.push(symbols[n % N_DIGITS]);
        n /= N_DIGITS;
    }
    s.chars().rev().collect()
}

fn symbolic<const N_DIGITS: usize>(symbols: [char; N_DIGITS], n: usize) -> EcoString {
    EcoString::from(symbols[(n - 1) % N_DIGITS]).repeat(n.div_ceil(N_DIGITS))
}
