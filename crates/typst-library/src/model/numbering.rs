use std::str::FromStr;

use chinese_number::{
    from_usize_to_chinese_ten_thousand as usize_to_chinese, ChineseCase, ChineseVariant,
};
use comemo::Tracked;
use ecow::{eco_format, EcoString, EcoVec};

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
    Afar,
    Agaw,
    AncientTamil,
    ArabicAbjad,
    ArabicIndic,
    Ari,
    Armenian,
    Balinese,
    Bamum,
    Bangla,
    Bengali,
    Binary,
    Blin,
    Bodo,
    Cambodian,
    CircledDecimal,
    CircledIdeograph,
    CircledKatakana,
    CircledKoreanConsonant,
    CircledKoreanSyllable,
    CircledLowerLatin,
    CircledUpperLatin,
    CjkDecimal,
    CjkEarthlyBranch,
    CjkHeavenlyStem,
    CjkTallyMark,
    Decimal,
    Devanagari,
    Dizi,
    Dogri,
    DottedDecimal,
    DoubleCircledDecimal,
    EthiopicHalehame,
    EthiopicHalehameAm,
    EthiopicHalehameTiEr,
    EthiopicHalehameTiEt,
    FilledCircledDecimal,
    FullwidthDecimal,
    FullwidthLowerAlpha,
    FullwidthLowerRoman,
    FullwidthUpperAlpha,
    FullwidthUpperRoman,
    Gedeo,
    Georgian,
    GreekLowerAncient,
    GreekLowerModern,
    GreekUpperAncient,
    GreekUpperModern,
    Gujarati,
    GujaratiAlpha,
    Gumuz,
    Gurmukhi,
    Hadiyya,
    Hangul,
    HangulConsonant,
    HanifiRohingya,
    Harari,
    Hebrew,
    Hindi,
    Hiragana,
    HiraganaIroha,
    JapaneseFormal,
    JapaneseInformal,
    Javanese,
    Kaffa,
    Kannada,
    KannadaAlpha,
    Kashmiri,
    Katakana,
    KatakanaIroha,
    KayahLi,
    Kebena,
    Kembata,
    Khmer,
    KhmerConsonant,
    Konkani,
    Konso,
    KoreanConsonant,
    KoreanHangulFormal,
    KoreanHanjaFormal,
    KoreanHanjaInformal,
    KoreanSyllable,
    Kunama,
    LannaHora,
    LannaTham,
    Lao,
    Lepcha,
    Limbu,
    LowerAlpha,
    LowerAlphaSymbolic,
    LowerArmenian,
    LowerBelorussian,
    LowerBulgarian,
    LowerGreek,
    LowerHexadecimal,
    LowerMacedonian,
    LowerRoman,
    LowerRussian,
    LowerRussianFull,
    LowerSerbian,
    LowerUkrainian,
    LowerUkrainianFull,
    MaghrebiAbjad,
    Maithili,
    Malayalam,
    MalayalamAlpha,
    Manipuri,
    Marathi,
    Meen,
    Meetei,
    Mongolian,
    Mro,
    Myanmar,
    NagMundari,
    NewBase60,
    Newa,
    NkoCardinal,
    Octal,
    OlChiki,
    Oriya,
    Oromo,
    ParenthesizedDecimal,
    ParenthesizedHangulConsonant,
    ParenthesizedHangulSyllable,
    ParenthesizedIdeograph,
    ParenthesizedLowerLatin,
    Persian,
    PersianAbjad,
    PersianAlphabetic,
    Punjabi,
    Saho,
    Sanskrit,
    Santali,
    Shan,
    Sidama,
    Silti,
    SimpChineseFormal,
    SimpChineseInformal,
    SimpleLowerRoman,
    SimpleUpperRoman,
    Sundanese,
    SuperDecimal,
    Symbol,
    TaiLue,
    TallyMark,
    Tamil,
    Telugu,
    TeluguAlpha,
    Thai,
    ThaiAlpha,
    Tibetan,
    Tigre,
    TradChineseFormal,
    TradChineseInformal,
    UpperAlpha,
    UpperAlphaSymbolic,
    UpperArmenian,
    UpperBelorussian,
    UpperBulgarian,
    UpperHexadecimal,
    UpperMacedonian,
    UpperRoman,
    UpperRussian,
    UpperRussianFull,
    UpperSerbian,
    UpperUkrainian,
    UpperUkrainianFull,
    Urdu,
    UrduAbjad,
    UrduAlphabetic,
    WarangCiti,
    Wolaita,
    Yemsa,
    Zhuyin,
}

impl NumberingKind {
    /// Create a numbering kind from a representative character.
    pub fn from_char(c: char) -> Option<Self> {
        Some(match c {
            '١' => NumberingKind::ArabicIndic,
            'ক' => NumberingKind::Bangla,
            '১' => NumberingKind::Bengali,
            '①' => NumberingKind::CircledDecimal,
            '1' => NumberingKind::Decimal,
            '⓵' => NumberingKind::DoubleCircledDecimal,
            '१' => NumberingKind::Devanagari,
            'א' => NumberingKind::Hebrew,
            'あ' => NumberingKind::Hiragana,
            'い' => NumberingKind::HiraganaIroha,
            'ア' => NumberingKind::Katakana,
            'イ' => NumberingKind::KatakanaIroha,
            'ㄱ' => NumberingKind::KoreanConsonant,
            '가' => NumberingKind::KoreanSyllable,
            'a' => NumberingKind::LowerAlpha,
            'α' => NumberingKind::LowerGreek,
            'i' => NumberingKind::LowerRoman,
            '۱' => NumberingKind::Persian,
            '壹' => NumberingKind::SimpChineseFormal,
            '一' => NumberingKind::SimpChineseInformal,
            '*' => NumberingKind::Symbol,
            'A' => NumberingKind::UpperAlpha,
            'Α' => NumberingKind::GreekUpperModern,
            'I' => NumberingKind::UpperRoman,
            _ => return None,
        })
    }

    /// Create a numbering kind from a name.
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "afar" => NumberingKind::Afar,
            "agaw" => NumberingKind::Agaw,
            "ancient-tamil" => NumberingKind::AncientTamil,
            "arabic-abjad" => NumberingKind::ArabicAbjad,
            "١" | "arabic-indic" => NumberingKind::ArabicIndic,
            "ari" => NumberingKind::Ari,
            "armenian" => NumberingKind::Armenian,
            "balinese" => NumberingKind::Balinese,
            "bamum" => NumberingKind::Bamum,
            "ক" | "bangla" => NumberingKind::Bangla,
            "১" | "bengali" => NumberingKind::Bengali,
            "binary" => NumberingKind::Binary,
            "blin" => NumberingKind::Blin,
            "bodo" => NumberingKind::Bodo,
            "cambodian" => NumberingKind::Cambodian,
            "①" | "circled-decimal" => NumberingKind::CircledDecimal,
            "circled-ideograph" => NumberingKind::CircledIdeograph,
            "ア" | "circled-katakana" => NumberingKind::CircledKatakana,
            "circled-korean-consonant" => NumberingKind::CircledKoreanConsonant,
            "circled-korean-syllable" => NumberingKind::CircledKoreanSyllable,
            "circled-lower-latin" => NumberingKind::CircledLowerLatin,
            "circled-upper-latin" => NumberingKind::CircledUpperLatin,
            "cjk-decimal" => NumberingKind::CjkDecimal,
            "cjk-earthly-branch" => NumberingKind::CjkEarthlyBranch,
            "cjk-heavenly-stem" => NumberingKind::CjkHeavenlyStem,
            "cjk-tally-mark" => NumberingKind::CjkTallyMark,
            "1" | "decimal" => NumberingKind::Decimal,
            "१" | "devanagari" => NumberingKind::Devanagari,
            "dizi" => NumberingKind::Dizi,
            "dogri" => NumberingKind::Dogri,
            "dotted-decimal" => NumberingKind::DottedDecimal,
            "⓵" | "double-circled-decimal" => NumberingKind::DoubleCircledDecimal,
            "ethiopic-halehame" => NumberingKind::EthiopicHalehame,
            "ethiopic-halehame-am" => NumberingKind::EthiopicHalehameAm,
            "ethiopic-halehame-ti-er" => NumberingKind::EthiopicHalehameTiEr,
            "ethiopic-halehame-ti-et" => NumberingKind::EthiopicHalehameTiEt,
            "filled-circled-decimal" => NumberingKind::FilledCircledDecimal,
            "fullwidth-decimal" => NumberingKind::FullwidthDecimal,
            "fullwidth-lower-alpha" => NumberingKind::FullwidthLowerAlpha,
            "fullwidth-lower-roman" => NumberingKind::FullwidthLowerRoman,
            "fullwidth-upper-alpha" => NumberingKind::FullwidthUpperAlpha,
            "fullwidth-upper-roman" => NumberingKind::FullwidthUpperRoman,
            "gedeo" => NumberingKind::Gedeo,
            "georgian" => NumberingKind::Georgian,
            "greek-lower-ancient" => NumberingKind::GreekLowerAncient,
            "greek-lower-modern" => NumberingKind::GreekLowerModern,
            "greek-upper-ancient" => NumberingKind::GreekUpperAncient,
            "Α" | "greek-upper-modern" => NumberingKind::GreekUpperModern,
            "gujarati" => NumberingKind::Gujarati,
            "gujarati-alpha" => NumberingKind::GujaratiAlpha,
            "gumuz" => NumberingKind::Gumuz,
            "gurmukhi" => NumberingKind::Gurmukhi,
            "hadiyya" => NumberingKind::Hadiyya,
            "hangul" => NumberingKind::Hangul,
            "hangul-consonant" => NumberingKind::HangulConsonant,
            "hanifi-rohingya" => NumberingKind::HanifiRohingya,
            "harari" => NumberingKind::Harari,
            "א" | "hebrew" => NumberingKind::Hebrew,
            "hindi" => NumberingKind::Hindi,
            "あ" | "hiragana" => NumberingKind::Hiragana,
            "い" | "hiragana-iroha" => NumberingKind::HiraganaIroha,
            "japanese-formal" => NumberingKind::JapaneseFormal,
            "japanese-informal" => NumberingKind::JapaneseInformal,
            "javanese" => NumberingKind::Javanese,
            "kaffa" => NumberingKind::Kaffa,
            "kannada" => NumberingKind::Kannada,
            "kannada-alpha" => NumberingKind::KannadaAlpha,
            "kashmiri" => NumberingKind::Kashmiri,
            "katakana" => NumberingKind::Katakana,
            "イ" | "katakana-iroha" => NumberingKind::KatakanaIroha,
            "kayah-li" => NumberingKind::KayahLi,
            "kebena" => NumberingKind::Kebena,
            "kembata" => NumberingKind::Kembata,
            "khmer" => NumberingKind::Khmer,
            "khmer-consonant" => NumberingKind::KhmerConsonant,
            "konkani" => NumberingKind::Konkani,
            "konso" => NumberingKind::Konso,
            "ㄱ" | "korean-consonant" => NumberingKind::KoreanConsonant,
            "korean-hangul-formal" => NumberingKind::KoreanHangulFormal,
            "korean-hanja-formal" => NumberingKind::KoreanHanjaFormal,
            "korean-hanja-informal" => NumberingKind::KoreanHanjaInformal,
            "가" | "korean-syllable" => NumberingKind::KoreanSyllable,
            "kunama" => NumberingKind::Kunama,
            "lanna-hora" => NumberingKind::LannaHora,
            "lanna-tham" => NumberingKind::LannaTham,
            "lao" => NumberingKind::Lao,
            "lepcha" => NumberingKind::Lepcha,
            "limbu" => NumberingKind::Limbu,
            "a" | "lower-alpha" => NumberingKind::LowerAlpha,
            "lower-alpha-symbolic" => NumberingKind::LowerAlphaSymbolic,
            "lower-armenian" => NumberingKind::LowerArmenian,
            "lower-belorussian" => NumberingKind::LowerBelorussian,
            "lower-bulgarian" => NumberingKind::LowerBulgarian,
            "α" | "lower-greek" => NumberingKind::LowerGreek,
            "lower-hexadecimal" => NumberingKind::LowerHexadecimal,
            "lower-macedonian" => NumberingKind::LowerMacedonian,
            "i" | "lower-roman" => NumberingKind::LowerRoman,
            "lower-russian" => NumberingKind::LowerRussian,
            "lower-russian-full" => NumberingKind::LowerRussianFull,
            "lower-serbian" => NumberingKind::LowerSerbian,
            "lower-ukrainian" => NumberingKind::LowerUkrainian,
            "lower-ukrainian-full" => NumberingKind::LowerUkrainianFull,
            "maghrebi-abjad" => NumberingKind::MaghrebiAbjad,
            "maithili" => NumberingKind::Maithili,
            "malayalam" => NumberingKind::Malayalam,
            "malayalam-alpha" => NumberingKind::MalayalamAlpha,
            "manipuri" => NumberingKind::Manipuri,
            "marathi" => NumberingKind::Marathi,
            "meen" => NumberingKind::Meen,
            "meetei" => NumberingKind::Meetei,
            "mongolian" => NumberingKind::Mongolian,
            "mro" => NumberingKind::Mro,
            "myanmar" => NumberingKind::Myanmar,
            "nag-mundari" => NumberingKind::NagMundari,
            "new-base-60" => NumberingKind::NewBase60,
            "newa" => NumberingKind::Newa,
            "nko-cardinal" => NumberingKind::NkoCardinal,
            "octal" => NumberingKind::Octal,
            "ol-chiki" => NumberingKind::OlChiki,
            "oriya" => NumberingKind::Oriya,
            "oromo" => NumberingKind::Oromo,
            "parenthesized-decimal" => NumberingKind::ParenthesizedDecimal,
            "parenthesized-hangul-consonant" => {
                NumberingKind::ParenthesizedHangulConsonant
            }
            "parenthesized-hangul-syllable" => NumberingKind::ParenthesizedHangulSyllable,
            "parenthesized-ideograph" => NumberingKind::ParenthesizedIdeograph,
            "parenthesized-lower-latin" => NumberingKind::ParenthesizedLowerLatin,
            "۱" | "persian" => NumberingKind::Persian,
            "persian-abjad" => NumberingKind::PersianAbjad,
            "persian-alphabetic" => NumberingKind::PersianAlphabetic,
            "punjabi" => NumberingKind::Punjabi,
            "saho" => NumberingKind::Saho,
            "sanskrit" => NumberingKind::Sanskrit,
            "santali" => NumberingKind::Santali,
            "shan" => NumberingKind::Shan,
            "sidama" => NumberingKind::Sidama,
            "silti" => NumberingKind::Silti,
            "壹" | "simp-chinese-formal" => NumberingKind::SimpChineseFormal,
            "一" | "simp-chinese-informal" => NumberingKind::SimpChineseInformal,
            "simple-lower-roman" => NumberingKind::SimpleLowerRoman,
            "simple-upper-roman" => NumberingKind::SimpleUpperRoman,
            "sundanese" => NumberingKind::Sundanese,
            "super-decimal" => NumberingKind::SuperDecimal,
            "*" | "symbol" => NumberingKind::Symbol,
            "tai-lue" => NumberingKind::TaiLue,
            "tally-mark" => NumberingKind::TallyMark,
            "tamil" => NumberingKind::Tamil,
            "telugu" => NumberingKind::Telugu,
            "telugu-alpha" => NumberingKind::TeluguAlpha,
            "thai" => NumberingKind::Thai,
            "thai-alpha" => NumberingKind::ThaiAlpha,
            "tibetan" => NumberingKind::Tibetan,
            "tigre" => NumberingKind::Tigre,
            "trad-chinese-formal" => NumberingKind::TradChineseFormal,
            "trad-chinese-informal" => NumberingKind::TradChineseInformal,
            "A" | "upper-alpha" => NumberingKind::UpperAlpha,
            "upper-alpha-symbolic" => NumberingKind::UpperAlphaSymbolic,
            "upper-armenian" => NumberingKind::UpperArmenian,
            "upper-belorussian" => NumberingKind::UpperBelorussian,
            "upper-bulgarian" => NumberingKind::UpperBulgarian,
            "upper-hexadecimal" => NumberingKind::UpperHexadecimal,
            "upper-macedonian" => NumberingKind::UpperMacedonian,
            "I" | "upper-roman" => NumberingKind::UpperRoman,
            "upper-russian" => NumberingKind::UpperRussian,
            "upper-russian-full" => NumberingKind::UpperRussianFull,
            "upper-serbian" => NumberingKind::UpperSerbian,
            "upper-ukrainian" => NumberingKind::UpperUkrainian,
            "upper-ukrainian-full" => NumberingKind::UpperUkrainianFull,
            "urdu" => NumberingKind::Urdu,
            "urdu-abjad" => NumberingKind::UrduAbjad,
            "urdu-alphabetic" => NumberingKind::UrduAlphabetic,
            "warang-citi" => NumberingKind::WarangCiti,
            "wolaita" => NumberingKind::Wolaita,
            "yemsa" => NumberingKind::Yemsa,
            "zhuyin" => NumberingKind::Zhuyin,
            _ => return None,
        })
    }

    /// The name for this numbering kind.
    pub fn to_name(self) -> &'static str {
        match self {
            Self::Afar => "afar",
            Self::Agaw => "agaw",
            Self::AncientTamil => "ancient-tamil",
            Self::ArabicAbjad => "arabic-abjad",
            Self::ArabicIndic => "arabic-indic",
            Self::Ari => "ari",
            Self::Armenian => "armenian",
            Self::Balinese => "balinese",
            Self::Bamum => "bamum",
            Self::Bangla => "bangla",
            Self::Bengali => "bengali",
            Self::Binary => "binary",
            Self::Blin => "blin",
            Self::Bodo => "bodo",
            Self::Cambodian => "cambodian",
            Self::CircledDecimal => "circled-decimal",
            Self::CircledIdeograph => "circled-ideograph",
            Self::CircledKatakana => "circled-katakana",
            Self::CircledKoreanConsonant => "circled-korean-consonant",
            Self::CircledKoreanSyllable => "circled-korean-syllable",
            Self::CircledLowerLatin => "circled-lower-latin",
            Self::CircledUpperLatin => "circled-upper-latin",
            Self::CjkDecimal => "cjk-decimal",
            Self::CjkEarthlyBranch => "cjk-earthly-branch",
            Self::CjkHeavenlyStem => "cjk-heavenly-stem",
            Self::CjkTallyMark => "cjk-tally-mark",
            Self::Decimal => "decimal",
            Self::Devanagari => "devanagari",
            Self::Dizi => "dizi",
            Self::Dogri => "dogri",
            Self::DottedDecimal => "dotted-decimal",
            Self::DoubleCircledDecimal => "double-circled-decimal",
            Self::EthiopicHalehame => "ethiopic-halehame",
            Self::EthiopicHalehameAm => "ethiopic-halehame-am",
            Self::EthiopicHalehameTiEr => "ethiopic-halehame-ti-er",
            Self::EthiopicHalehameTiEt => "ethiopic-halehame-ti-et",
            Self::FilledCircledDecimal => "filled-circled-decimal",
            Self::FullwidthDecimal => "fullwidth-decimal",
            Self::FullwidthLowerAlpha => "fullwidth-lower-alpha",
            Self::FullwidthLowerRoman => "fullwidth-lower-roman",
            Self::FullwidthUpperAlpha => "fullwidth-upper-alpha",
            Self::FullwidthUpperRoman => "fullwidth-upper-roman",
            Self::Gedeo => "gedeo",
            Self::Georgian => "georgian",
            Self::GreekLowerAncient => "greek-lower-ancient",
            Self::GreekLowerModern => "greek-lower-modern",
            Self::GreekUpperAncient => "greek-upper-ancient",
            Self::GreekUpperModern => "greek-upper-modern",
            Self::Gujarati => "gujarati",
            Self::GujaratiAlpha => "gujarati-alpha",
            Self::Gumuz => "gumuz",
            Self::Gurmukhi => "gurmukhi",
            Self::Hadiyya => "hadiyya",
            Self::Hangul => "hangul",
            Self::HangulConsonant => "hangul-consonant",
            Self::HanifiRohingya => "hanifi-rohingya",
            Self::Harari => "harari",
            Self::Hebrew => "hebrew",
            Self::Hindi => "hindi",
            Self::Hiragana => "hiragana",
            Self::HiraganaIroha => "hiragana-iroha",
            Self::JapaneseFormal => "japanese-formal",
            Self::JapaneseInformal => "japanese-informal",
            Self::Javanese => "javanese",
            Self::Kaffa => "kaffa",
            Self::Kannada => "kannada",
            Self::KannadaAlpha => "kannada-alpha",
            Self::Kashmiri => "kashmiri",
            Self::Katakana => "katakana",
            Self::KatakanaIroha => "katakana-iroha",
            Self::KayahLi => "kayah-li",
            Self::Kebena => "kebena",
            Self::Kembata => "kembata",
            Self::Khmer => "khmer",
            Self::KhmerConsonant => "khmer-consonant",
            Self::Konkani => "konkani",
            Self::Konso => "konso",
            Self::KoreanConsonant => "korean-consonant",
            Self::KoreanHangulFormal => "korean-hangul-formal",
            Self::KoreanHanjaFormal => "korean-hanja-formal",
            Self::KoreanHanjaInformal => "korean-hanja-informal",
            Self::KoreanSyllable => "korean-syllable",
            Self::Kunama => "kunama",
            Self::LannaHora => "lanna-hora",
            Self::LannaTham => "lanna-tham",
            Self::Lao => "lao",
            Self::Lepcha => "lepcha",
            Self::Limbu => "limbu",
            Self::LowerAlpha => "lower-alpha",
            Self::LowerAlphaSymbolic => "lower-alpha-symbolic",
            Self::LowerArmenian => "lower-armenian",
            Self::LowerBelorussian => "lower-belorussian",
            Self::LowerBulgarian => "lower-bulgarian",
            Self::LowerGreek => "lower-greek",
            Self::LowerHexadecimal => "lower-hexadecimal",
            Self::LowerMacedonian => "lower-macedonian",
            Self::LowerRoman => "lower-roman",
            Self::LowerRussian => "lower-russian",
            Self::LowerRussianFull => "lower-russian-full",
            Self::LowerSerbian => "lower-serbian",
            Self::LowerUkrainian => "lower-ukrainian",
            Self::LowerUkrainianFull => "lower-ukrainian-full",
            Self::MaghrebiAbjad => "maghrebi-abjad",
            Self::Maithili => "maithili",
            Self::Malayalam => "malayalam",
            Self::MalayalamAlpha => "malayalam-alpha",
            Self::Manipuri => "manipuri",
            Self::Marathi => "marathi",
            Self::Meen => "meen",
            Self::Meetei => "meetei",
            Self::Mongolian => "mongolian",
            Self::Mro => "mro",
            Self::Myanmar => "myanmar",
            Self::NagMundari => "nag-mundari",
            Self::NewBase60 => "new-base-60",
            Self::Newa => "newa",
            Self::NkoCardinal => "nko-cardinal",
            Self::Octal => "octal",
            Self::OlChiki => "ol-chiki",
            Self::Oriya => "oriya",
            Self::Oromo => "oromo",
            Self::ParenthesizedDecimal => "parenthesized-decimal",
            Self::ParenthesizedHangulConsonant => "parenthesized-hangul-consonant",
            Self::ParenthesizedHangulSyllable => "parenthesized-hangul-syllable",
            Self::ParenthesizedIdeograph => "parenthesized-ideograph",
            Self::ParenthesizedLowerLatin => "parenthesized-lower-latin",
            Self::Persian => "persian",
            Self::PersianAbjad => "persian-abjad",
            Self::PersianAlphabetic => "persian-alphabetic",
            Self::Punjabi => "punjabi",
            Self::Saho => "saho",
            Self::Sanskrit => "sanskrit",
            Self::Santali => "santali",
            Self::Shan => "shan",
            Self::Sidama => "sidama",
            Self::Silti => "silti",
            Self::SimpChineseFormal => "simp-chinese-formal",
            Self::SimpChineseInformal => "simp-chinese-informal",
            Self::SimpleLowerRoman => "simple-lower-roman",
            Self::SimpleUpperRoman => "simple-upper-roman",
            Self::Sundanese => "sundanese",
            Self::SuperDecimal => "super-decimal",
            Self::Symbol => "symbol",
            Self::TaiLue => "tai-lue",
            Self::TallyMark => "tally-mark",
            Self::Tamil => "tamil",
            Self::Telugu => "telugu",
            Self::TeluguAlpha => "telugu-alpha",
            Self::Thai => "thai",
            Self::ThaiAlpha => "thai-alpha",
            Self::Tibetan => "tibetan",
            Self::Tigre => "tigre",
            Self::TradChineseFormal => "trad-chinese-formal",
            Self::TradChineseInformal => "trad-chinese-informal",
            Self::UpperAlpha => "upper-alpha",
            Self::UpperAlphaSymbolic => "upper-alpha-symbolic",
            Self::UpperArmenian => "upper-armenian",
            Self::UpperBelorussian => "upper-belorussian",
            Self::UpperBulgarian => "upper-bulgarian",
            Self::UpperHexadecimal => "upper-hexadecimal",
            Self::UpperMacedonian => "upper-macedonian",
            Self::UpperRoman => "upper-roman",
            Self::UpperRussian => "upper-russian",
            Self::UpperRussianFull => "upper-russian-full",
            Self::UpperSerbian => "upper-serbian",
            Self::UpperUkrainian => "upper-ukrainian",
            Self::UpperUkrainianFull => "upper-ukrainian-full",
            Self::Urdu => "urdu",
            Self::UrduAbjad => "urdu-abjad",
            Self::UrduAlphabetic => "urdu-alphabetic",
            Self::WarangCiti => "warang-citi",
            Self::Wolaita => "wolaita",
            Self::Yemsa => "yemsa",
            Self::Zhuyin => "zhuyin",
        }
    }

    /// Apply the numbering to the given number.
    pub fn apply(self, n: usize) -> EcoString {
        match self {
            Self::Afar => alphabetic(
                [
                    '\u{1200}', '\u{1208}', '\u{1210}', '\u{1218}', '\u{1228}',
                    '\u{1230}', '\u{1260}', '\u{1270}', '\u{1290}', '\u{12A0}',
                    '\u{12A8}', '\u{12C8}', '\u{12D0}', '\u{12E8}', '\u{12F0}',
                    '\u{12F8}', '\u{1308}', '\u{1338}', '\u{1348}',
                ],
                n,
            ),
            Self::Agaw => alphabetic(
                [
                    '\u{1200}', '\u{1208}', '\u{1210}', '\u{1218}', '\u{1228}',
                    '\u{1230}', '\u{1238}', '\u{1240}', '\u{1250}', '\u{1260}',
                    '\u{1268}', '\u{1270}', '\u{1278}', '\u{1290}', '\u{1298}',
                    '\u{1300}', '\u{1308}', '\u{1318}', '\u{1320}', '\u{1328}',
                    '\u{1330}', '\u{1338}', '\u{1348}', '\u{1350}',
                ],
                n,
            ),
            Self::AncientTamil => additive(
                [
                    ("\u{BEF}\u{BF2}", 9000),
                    ("\u{BEE}\u{BF2}", 8000),
                    ("\u{BED}\u{BF2}", 7000),
                    ("\u{BEC}\u{BF2}", 6000),
                    ("\u{BEB}\u{BF2}", 5000),
                    ("\u{BEA}\u{BF2}", 4000),
                    ("\u{BE9}\u{BF2}", 3000),
                    ("\u{BE8}\u{BF2}", 2000),
                    ("\u{BF2}", 1000),
                    ("\u{BEF}\u{BF1}", 900),
                    ("\u{BEE}\u{BF1}", 800),
                    ("\u{BED}\u{BF1}", 700),
                    ("\u{BEC}\u{BF1}", 600),
                    ("\u{BEB}\u{BF1}", 500),
                    ("\u{BEA}\u{BF1}", 400),
                    ("\u{BE9}\u{BF1}", 300),
                    ("\u{BE8}\u{BF1}", 200),
                    ("\u{BF1}", 100),
                    ("\u{BEF}\u{BF0}", 90),
                    ("\u{BEE}\u{BF0}", 80),
                    ("\u{BED}\u{BF0}", 70),
                    ("\u{BEC}\u{BF0}", 60),
                    ("\u{BEB}\u{BF0}", 50),
                    ("\u{BEA}\u{BF0}", 40),
                    ("\u{BE9}\u{BF0}", 30),
                    ("\u{BE8}\u{BF0}", 20),
                    ("\u{BF0}", 10),
                    ("\u{BEF}", 9),
                    ("\u{BEE}", 8),
                    ("\u{BED}", 7),
                    ("\u{BEC}", 6),
                    ("\u{BEB}", 5),
                    ("\u{BEA}", 4),
                    ("\u{BE9}", 3),
                    ("\u{BE8}", 2),
                    ("\u{BE7}", 1),
                ],
                n,
            ),
            Self::ArabicAbjad => fixed(
                [
                    '\u{627}', '\u{628}', '\u{62C}', '\u{62F}', '\u{647}', '\u{648}',
                    '\u{632}', '\u{62D}', '\u{637}', '\u{64A}', '\u{643}', '\u{644}',
                    '\u{645}', '\u{646}', '\u{633}', '\u{639}', '\u{641}', '\u{635}',
                    '\u{642}', '\u{631}', '\u{634}', '\u{62A}', '\u{62B}', '\u{62E}',
                    '\u{630}', '\u{636}', '\u{638}', '\u{63A}',
                ],
                n,
            ),
            Self::ArabicIndic => numeric(
                [
                    '\u{660}', '\u{661}', '\u{662}', '\u{663}', '\u{664}', '\u{665}',
                    '\u{666}', '\u{667}', '\u{668}', '\u{669}',
                ],
                n,
            ),
            Self::Ari => alphabetic(
                [
                    '\u{1200}', '\u{1208}', '\u{1218}', '\u{1228}', '\u{1230}',
                    '\u{1238}', '\u{1260}', '\u{1268}', '\u{1270}', '\u{1278}',
                    '\u{1290}', '\u{1300}', '\u{1308}', '\u{1328}', '\u{1340}',
                    '\u{1350}',
                ],
                n,
            ),
            Self::Armenian => additive(
                [
                    ("\u{554}", 9000),
                    ("\u{553}", 8000),
                    ("\u{552}", 7000),
                    ("\u{551}", 6000),
                    ("\u{550}", 5000),
                    ("\u{54F}", 4000),
                    ("\u{54E}", 3000),
                    ("\u{54D}", 2000),
                    ("\u{54C}", 1000),
                    ("\u{54B}", 900),
                    ("\u{54A}", 800),
                    ("\u{549}", 700),
                    ("\u{548}", 600),
                    ("\u{547}", 500),
                    ("\u{546}", 400),
                    ("\u{545}", 300),
                    ("\u{544}", 200),
                    ("\u{543}", 100),
                    ("\u{542}", 90),
                    ("\u{541}", 80),
                    ("\u{540}", 70),
                    ("\u{53F}", 60),
                    ("\u{53E}", 50),
                    ("\u{53D}", 40),
                    ("\u{53C}", 30),
                    ("\u{53B}", 20),
                    ("\u{53A}", 10),
                    ("\u{539}", 9),
                    ("\u{538}", 8),
                    ("\u{537}", 7),
                    ("\u{536}", 6),
                    ("\u{535}", 5),
                    ("\u{534}", 4),
                    ("\u{533}", 3),
                    ("\u{532}", 2),
                    ("\u{531}", 1),
                ],
                n,
            ),
            Self::Balinese => numeric(
                [
                    '\u{1B50}', '\u{1B51}', '\u{1B52}', '\u{1B53}', '\u{1B54}',
                    '\u{1B55}', '\u{1B56}', '\u{1B57}', '\u{1B58}', '\u{1B59}',
                ],
                n,
            ),
            Self::Bamum => numeric(
                [
                    '\u{A6EF}', '\u{A6E6}', '\u{A6E7}', '\u{A6E8}', '\u{A6E9}',
                    '\u{A6EA}', '\u{A6EB}', '\u{A6EC}', '\u{A6ED}', '\u{A6EE}',
                ],
                n,
            ),
            Self::Bangla => alphabetic(
                [
                    '\u{0995}', '\u{0996}', '\u{0997}', '\u{0998}', '\u{0999}',
                    '\u{099A}', '\u{099B}', '\u{099C}', '\u{099D}', '\u{099E}',
                    '\u{099F}', '\u{09A0}', '\u{09A1}', '\u{09A1}', '\u{09A2}',
                    '\u{09A2}', '\u{09A3}', '\u{09A4}', '\u{09CE}', '\u{09A5}',
                    '\u{09A6}', '\u{09A7}', '\u{09A8}', '\u{09AA}', '\u{09AB}',
                    '\u{09AC}', '\u{09AD}', '\u{09AE}', '\u{09AF}', '\u{09AF}',
                    '\u{09B0}', '\u{09B2}', '\u{09B6}', '\u{09B7}', '\u{09B8}',
                    '\u{09B9}',
                ],
                n,
            ),
            Self::Bengali => numeric(
                [
                    '\u{9E6}', '\u{9E7}', '\u{9E8}', '\u{9E9}', '\u{9EA}', '\u{9EB}',
                    '\u{9EC}', '\u{9ED}', '\u{9EE}', '\u{9EF}',
                ],
                n,
            ),
            Self::Binary => numeric(['\u{30}', '\u{31}'], n),
            Self::Blin => alphabetic(
                [
                    '\u{1200}', '\u{1208}', '\u{1210}', '\u{1218}', '\u{1230}',
                    '\u{1238}', '\u{1228}', '\u{1240}', '\u{1250}', '\u{1260}',
                    '\u{1270}', '\u{1290}', '\u{1300}', '\u{1308}', '\u{1318}',
                    '\u{1320}', '\u{1328}', '\u{1348}', '\u{1278}', '\u{1298}',
                    '\u{1338}', '\u{1330}', '\u{1350}',
                ],
                n,
            ),
            Self::Bodo => alphabetic(
                [
                    '\u{915}', '\u{916}', '\u{917}', '\u{918}', '\u{919}', '\u{91A}',
                    '\u{91B}', '\u{91C}', '\u{91D}', '\u{91E}', '\u{91F}', '\u{920}',
                    '\u{921}', '\u{922}', '\u{923}', '\u{924}', '\u{925}', '\u{926}',
                    '\u{927}', '\u{928}', '\u{92A}', '\u{92B}', '\u{92C}', '\u{92D}',
                    '\u{92E}', '\u{92F}', '\u{930}', '\u{932}', '\u{935}', '\u{936}',
                    '\u{937}', '\u{938}', '\u{939}',
                ],
                n,
            ),
            Self::Cambodian => numeric(
                [
                    '\u{17E0}', '\u{17E1}', '\u{17E2}', '\u{17E3}', '\u{17E4}',
                    '\u{17E5}', '\u{17E6}', '\u{17E7}', '\u{17E8}', '\u{17E9}',
                ],
                n,
            ),
            Self::CircledDecimal => fixed(
                [
                    '\u{2460}', '\u{2461}', '\u{2462}', '\u{2463}', '\u{2464}',
                    '\u{2465}', '\u{2466}', '\u{2467}', '\u{2468}', '\u{2469}',
                    '\u{246A}', '\u{246B}', '\u{246C}', '\u{246D}', '\u{246E}',
                    '\u{246F}', '\u{2470}', '\u{2471}', '\u{2472}', '\u{2473}',
                    '\u{3251}', '\u{3252}', '\u{3253}', '\u{3254}', '\u{3255}',
                    '\u{3256}', '\u{3257}', '\u{3258}', '\u{3259}', '\u{325a}',
                    '\u{325b}', '\u{325c}', '\u{325d}', '\u{325e}', '\u{325f}',
                    '\u{32b1}', '\u{32b2}', '\u{32b3}', '\u{32b4}', '\u{32b5}',
                    '\u{32b6}', '\u{32b7}', '\u{32b8}', '\u{32b9}', '\u{32ba}',
                    '\u{32bb}', '\u{32bc}', '\u{32bd}', '\u{32be}', '\u{32bf}',
                ],
                n,
            ),
            Self::CircledIdeograph => fixed(
                [
                    '\u{3280}', '\u{3281}', '\u{3282}', '\u{3283}', '\u{3284}',
                    '\u{3285}', '\u{3286}', '\u{3287}', '\u{3288}', '\u{3289}',
                ],
                n,
            ),
            Self::CircledKatakana => fixed(
                [
                    '\u{32D0}', '\u{32D1}', '\u{32D2}', '\u{32D3}', '\u{32D4}',
                    '\u{32D5}', '\u{32D6}', '\u{32D7}', '\u{32D8}', '\u{32D9}',
                    '\u{32DA}', '\u{32DB}', '\u{32DC}', '\u{32DD}', '\u{32DE}',
                    '\u{32DF}', '\u{32E0}', '\u{32E1}', '\u{32E2}', '\u{32E3}',
                    '\u{32E4}', '\u{32E5}', '\u{32E6}', '\u{32E7}', '\u{32E8}',
                    '\u{32E9}', '\u{32EA}', '\u{32EB}', '\u{32EC}', '\u{32ED}',
                    '\u{32EE}', '\u{32EF}', '\u{32F0}', '\u{32F1}', '\u{32F2}',
                    '\u{32F3}', '\u{32F4}', '\u{32F5}', '\u{32F6}', '\u{32F7}',
                    '\u{32F8}', '\u{32F9}', '\u{32FA}', '\u{32FB}', '\u{32FC}',
                    '\u{32FD}', '\u{32FE}',
                ],
                n,
            ),
            Self::CircledKoreanConsonant => fixed(
                [
                    '\u{3260}', '\u{3261}', '\u{3262}', '\u{3263}', '\u{3264}',
                    '\u{3265}', '\u{3266}', '\u{3267}', '\u{3268}', '\u{3269}',
                    '\u{326A}', '\u{326B}', '\u{326C}', '\u{326D}',
                ],
                n,
            ),
            Self::CircledKoreanSyllable => fixed(
                [
                    '\u{326E}', '\u{326F}', '\u{3270}', '\u{3271}', '\u{3272}',
                    '\u{3273}', '\u{3274}', '\u{3275}', '\u{3276}', '\u{3277}',
                    '\u{3278}', '\u{3279}', '\u{327A}', '\u{327B}',
                ],
                n,
            ),
            Self::CircledLowerLatin => fixed(
                [
                    '\u{24D0}', '\u{24D1}', '\u{24D2}', '\u{24D3}', '\u{24D4}',
                    '\u{24D5}', '\u{24D6}', '\u{24D7}', '\u{24D8}', '\u{24D9}',
                    '\u{24DA}', '\u{24DB}', '\u{24DC}', '\u{24DD}', '\u{24DE}',
                    '\u{24DF}', '\u{24E0}', '\u{24E1}', '\u{24E2}', '\u{24E3}',
                    '\u{24E4}', '\u{24E5}', '\u{24E6}', '\u{24E7}', '\u{24E8}',
                    '\u{24E9}',
                ],
                n,
            ),
            Self::CircledUpperLatin => fixed(
                [
                    '\u{24B6}', '\u{24B7}', '\u{24B8}', '\u{24B9}', '\u{24BA}',
                    '\u{24BB}', '\u{24BC}', '\u{24BD}', '\u{24BE}', '\u{24BF}',
                    '\u{24C0}', '\u{24C1}', '\u{24C2}', '\u{24C3}', '\u{24C4}',
                    '\u{24C5}', '\u{24C6}', '\u{24C7}', '\u{24C8}', '\u{24C9}',
                    '\u{24CA}', '\u{24CB}', '\u{24CC}', '\u{24CD}', '\u{24CE}',
                    '\u{24CF}',
                ],
                n,
            ),
            Self::CjkDecimal => numeric(
                [
                    '\u{3007}', '\u{4E00}', '\u{4E8C}', '\u{4E09}', '\u{56DB}',
                    '\u{4E94}', '\u{516D}', '\u{4E03}', '\u{516B}', '\u{4E5D}',
                ],
                n,
            ),
            Self::CjkEarthlyBranch => fixed(
                [
                    '\u{5B50}', '\u{4E11}', '\u{5BC5}', '\u{536F}', '\u{8FB0}',
                    '\u{5DF3}', '\u{5348}', '\u{672A}', '\u{7533}', '\u{9149}',
                    '\u{620C}', '\u{4EA5}',
                ],
                n,
            ),
            Self::CjkHeavenlyStem => fixed(
                [
                    '\u{7532}', '\u{4E59}', '\u{4E19}', '\u{4E01}', '\u{620A}',
                    '\u{5DF1}', '\u{5E9A}', '\u{8F9B}', '\u{58EC}', '\u{7678}',
                ],
                n,
            ),
            Self::CjkTallyMark => additive(
                [
                    ("\u{1D376}", 5),
                    ("\u{1D375}", 4),
                    ("\u{1D374}", 3),
                    ("\u{1D373}", 2),
                    ("\u{1D372}", 1),
                ],
                n,
            ),
            Self::Decimal => numeric(
                [
                    '\u{30}', '\u{31}', '\u{32}', '\u{33}', '\u{34}', '\u{35}', '\u{36}',
                    '\u{37}', '\u{38}', '\u{39}',
                ],
                n,
            ),
            Self::Devanagari => numeric(
                [
                    '\u{966}', '\u{967}', '\u{968}', '\u{969}', '\u{96A}', '\u{96B}',
                    '\u{96C}', '\u{96D}', '\u{96E}', '\u{96F}',
                ],
                n,
            ),
            Self::Dizi => alphabetic(
                [
                    '\u{1200}', '\u{1208}', '\u{1218}', '\u{1228}', '\u{1230}',
                    '\u{1238}', '\u{1240}', '\u{1260}', '\u{1270}', '\u{1278}',
                    '\u{1290}', '\u{1298}', '\u{1300}', '\u{1308}', '\u{1320}',
                    '\u{1328}', '\u{1338}', '\u{1340}', '\u{1348}',
                ],
                n,
            ),
            Self::Dogri => alphabetic(
                [
                    '\u{915}', '\u{916}', '\u{917}', '\u{918}', '\u{919}', '\u{91A}',
                    '\u{91B}', '\u{91C}', '\u{91D}', '\u{91E}', '\u{91F}', '\u{920}',
                    '\u{921}', '\u{922}', '\u{923}', '\u{924}', '\u{925}', '\u{926}',
                    '\u{927}', '\u{928}', '\u{92A}', '\u{92B}', '\u{92C}', '\u{92D}',
                    '\u{92E}', '\u{92F}', '\u{930}', '\u{932}', '\u{935}', '\u{936}',
                    '\u{937}', '\u{938}', '\u{939}',
                ],
                n,
            ),
            Self::DottedDecimal => fixed(
                [
                    '\u{2488}', '\u{2489}', '\u{248A}', '\u{248B}', '\u{248C}',
                    '\u{248D}', '\u{248E}', '\u{248F}', '\u{2490}', '\u{2491}',
                    '\u{2492}', '\u{2493}', '\u{2494}', '\u{2495}', '\u{2496}',
                    '\u{2497}', '\u{2498}', '\u{2499}', '\u{249A}', '\u{249B}',
                ],
                n,
            ),
            Self::DoubleCircledDecimal => fixed(
                [
                    '\u{24F5}', '\u{24F6}', '\u{24F7}', '\u{24F8}', '\u{24F9}',
                    '\u{24FA}', '\u{24FB}', '\u{24FC}', '\u{24FD}', '\u{24FE}',
                ],
                n,
            ),
            Self::EthiopicHalehame => alphabetic(
                [
                    '\u{1200}', '\u{1208}', '\u{1210}', '\u{1218}', '\u{1220}',
                    '\u{1228}', '\u{1230}', '\u{1240}', '\u{1260}', '\u{1270}',
                    '\u{1280}', '\u{1290}', '\u{12A0}', '\u{12A8}', '\u{12C8}',
                    '\u{12D0}', '\u{12D8}', '\u{12E8}', '\u{12F0}', '\u{1308}',
                    '\u{1320}', '\u{1330}', '\u{1338}', '\u{1340}', '\u{1348}',
                    '\u{1350}',
                ],
                n,
            ),
            Self::EthiopicHalehameAm => alphabetic(
                [
                    '\u{1200}', '\u{1208}', '\u{1210}', '\u{1218}', '\u{1220}',
                    '\u{1228}', '\u{1230}', '\u{1238}', '\u{1240}', '\u{1260}',
                    '\u{1270}', '\u{1278}', '\u{1280}', '\u{1290}', '\u{1298}',
                    '\u{12A0}', '\u{12A8}', '\u{12B8}', '\u{12C8}', '\u{12D0}',
                    '\u{12D8}', '\u{12E0}', '\u{12E8}', '\u{12F0}', '\u{1300}',
                    '\u{1308}', '\u{1320}', '\u{1328}', '\u{1330}', '\u{1338}',
                    '\u{1340}', '\u{1348}', '\u{1350}',
                ],
                n,
            ),
            Self::EthiopicHalehameTiEr => alphabetic(
                [
                    '\u{1200}', '\u{1208}', '\u{1210}', '\u{1218}', '\u{1228}',
                    '\u{1230}', '\u{1238}', '\u{1240}', '\u{1250}', '\u{1260}',
                    '\u{1270}', '\u{1278}', '\u{1290}', '\u{1298}', '\u{12A0}',
                    '\u{12A8}', '\u{12B8}', '\u{12C8}', '\u{12D0}', '\u{12D8}',
                    '\u{12E0}', '\u{12E8}', '\u{12F0}', '\u{1300}', '\u{1308}',
                    '\u{1320}', '\u{1328}', '\u{1330}', '\u{1338}', '\u{1348}',
                    '\u{1350}',
                ],
                n,
            ),
            Self::EthiopicHalehameTiEt => alphabetic(
                [
                    '\u{1200}', '\u{1208}', '\u{1210}', '\u{1218}', '\u{1220}',
                    '\u{1228}', '\u{1230}', '\u{1238}', '\u{1240}', '\u{1250}',
                    '\u{1260}', '\u{1270}', '\u{1278}', '\u{1280}', '\u{1290}',
                    '\u{1298}', '\u{12A0}', '\u{12A8}', '\u{12B8}', '\u{12C8}',
                    '\u{12D0}', '\u{12D8}', '\u{12E0}', '\u{12E8}', '\u{12F0}',
                    '\u{1300}', '\u{1308}', '\u{1320}', '\u{1328}', '\u{1330}',
                    '\u{1338}', '\u{1340}', '\u{1348}', '\u{1350}',
                ],
                n,
            ),
            Self::FilledCircledDecimal => fixed(
                [
                    '\u{2776}', '\u{2777}', '\u{2778}', '\u{2779}', '\u{277a}',
                    '\u{277b}', '\u{277c}', '\u{277d}', '\u{277e}', '\u{277f}',
                    '\u{24EB}', '\u{24EC}', '\u{24ED}', '\u{24EE}', '\u{24EF}',
                    '\u{24F0}', '\u{24F1}', '\u{24F2}', '\u{24F3}', '\u{24F4}',
                ],
                n,
            ),
            Self::FullwidthDecimal => numeric(
                [
                    '\u{FF10}', '\u{FF11}', '\u{FF12}', '\u{FF13}', '\u{FF14}',
                    '\u{FF15}', '\u{FF16}', '\u{FF17}', '\u{FF18}', '\u{FF19}',
                ],
                n,
            ),
            Self::FullwidthLowerAlpha => alphabetic(
                [
                    '\u{FF41}', '\u{FF42}', '\u{FF43}', '\u{FF44}', '\u{FF45}',
                    '\u{FF46}', '\u{FF47}', '\u{FF48}', '\u{FF49}', '\u{FF4A}',
                    '\u{FF4B}', '\u{FF4C}', '\u{FF4D}', '\u{FF4E}', '\u{FF4F}',
                    '\u{FF50}', '\u{FF51}', '\u{FF52}', '\u{FF53}', '\u{FF54}',
                    '\u{FF55}', '\u{FF56}', '\u{FF57}', '\u{FF58}', '\u{FF59}',
                    '\u{FF5A}',
                ],
                n,
            ),
            Self::FullwidthLowerRoman => fixed(
                [
                    '\u{2170}', '\u{2171}', '\u{2172}', '\u{2173}', '\u{2174}',
                    '\u{2175}', '\u{2176}', '\u{2177}', '\u{2178}', '\u{2179}',
                    '\u{217A}', '\u{217B}',
                ],
                n,
            ),
            Self::FullwidthUpperAlpha => alphabetic(
                [
                    '\u{FF21}', '\u{FF22}', '\u{FF23}', '\u{FF24}', '\u{FF25}',
                    '\u{FF26}', '\u{FF27}', '\u{FF28}', '\u{FF29}', '\u{FF2A}',
                    '\u{FF2B}', '\u{FF2C}', '\u{FF2D}', '\u{FF2E}', '\u{FF2F}',
                    '\u{FF30}', '\u{FF31}', '\u{FF32}', '\u{FF33}', '\u{FF34}',
                    '\u{FF35}', '\u{FF36}', '\u{FF37}', '\u{FF38}', '\u{FF39}',
                    '\u{FF3A}',
                ],
                n,
            ),
            Self::FullwidthUpperRoman => fixed(
                [
                    '\u{2160}', '\u{2161}', '\u{2162}', '\u{2163}', '\u{2164}',
                    '\u{2165}', '\u{2166}', '\u{2167}', '\u{2168}', '\u{2169}',
                    '\u{216A}', '\u{216B}',
                ],
                n,
            ),
            Self::Gedeo => alphabetic(
                [
                    '\u{1200}', '\u{1208}', '\u{1218}', '\u{1228}', '\u{1230}',
                    '\u{1238}', '\u{1240}', '\u{1260}', '\u{1270}', '\u{1278}',
                    '\u{1290}', '\u{1300}', '\u{1308}', '\u{1320}', '\u{1328}',
                    '\u{1330}', '\u{1338}', '\u{1348}', '\u{1350}',
                ],
                n,
            ),
            Self::Georgian => additive(
                [
                    ("\u{10F5}", 10000),
                    ("\u{10F0}", 9000),
                    ("\u{10EF}", 8000),
                    ("\u{10F4}", 7000),
                    ("\u{10EE}", 6000),
                    ("\u{10ED}", 5000),
                    ("\u{10EC}", 4000),
                    ("\u{10EB}", 3000),
                    ("\u{10EA}", 2000),
                    ("\u{10E9}", 1000),
                    ("\u{10E8}", 900),
                    ("\u{10E7}", 800),
                    ("\u{10E6}", 700),
                    ("\u{10E5}", 600),
                    ("\u{10E4}", 500),
                    ("\u{10F3}", 400),
                    ("\u{10E2}", 300),
                    ("\u{10E1}", 200),
                    ("\u{10E0}", 100),
                    ("\u{10DF}", 90),
                    ("\u{10DE}", 80),
                    ("\u{10DD}", 70),
                    ("\u{10F2}", 60),
                    ("\u{10DC}", 50),
                    ("\u{10DB}", 40),
                    ("\u{10DA}", 30),
                    ("\u{10D9}", 20),
                    ("\u{10D8}", 10),
                    ("\u{10D7}", 9),
                    ("\u{10F1}", 8),
                    ("\u{10D6}", 7),
                    ("\u{10D5}", 6),
                    ("\u{10D4}", 5),
                    ("\u{10D3}", 4),
                    ("\u{10D2}", 3),
                    ("\u{10D1}", 2),
                    ("\u{10D0}", 1),
                ],
                n,
            ),
            Self::GreekLowerAncient => additive(
                [
                    ("\u{3E1}", 900),
                    ("\u{3C9}", 800),
                    ("\u{3C8}", 700),
                    ("\u{3C7}", 600),
                    ("\u{3C6}", 500),
                    ("\u{3C5}", 400),
                    ("\u{3C4}", 300),
                    ("\u{3C3}", 200),
                    ("\u{3C1}", 100),
                    ("\u{3DF}", 90),
                    ("\u{3C0}", 80),
                    ("\u{3BF}", 70),
                    ("\u{3BE}", 60),
                    ("\u{3BD}", 50),
                    ("\u{3BC}", 40),
                    ("\u{3BB}", 30),
                    ("\u{3BA}", 20),
                    ("\u{3B9}", 10),
                    ("\u{3B8}", 9),
                    ("\u{3B7}", 8),
                    ("\u{3B6}", 7),
                    ("\u{3DB}", 6),
                    ("\u{3B5}", 5),
                    ("\u{3B4}", 4),
                    ("\u{3B3}", 3),
                    ("\u{3B2}", 2),
                    ("\u{3B1}", 1),
                    ("\u{1018a}", 0),
                ],
                n,
            ),
            Self::GreekLowerModern => additive(
                [
                    ("\u{3E1}", 900),
                    ("\u{3C9}", 800),
                    ("\u{3C8}", 700),
                    ("\u{3C7}", 600),
                    ("\u{3C6}", 500),
                    ("\u{3C5}", 400),
                    ("\u{3C4}", 300),
                    ("\u{3C3}", 200),
                    ("\u{3C1}", 100),
                    ("\u{3DF}", 90),
                    ("\u{3C0}", 80),
                    ("\u{3BF}", 70),
                    ("\u{3BE}", 60),
                    ("\u{3BD}", 50),
                    ("\u{3BC}", 40),
                    ("\u{3BB}", 30),
                    ("\u{3BA}", 20),
                    ("\u{3B9}", 10),
                    ("\u{3B8}", 9),
                    ("\u{3B7}", 8),
                    ("\u{3B6}", 7),
                    ("\u{3C3}\u{3C4}", 6),
                    ("\u{3B5}", 5),
                    ("\u{3B4}", 4),
                    ("\u{3B3}", 3),
                    ("\u{3B2}", 2),
                    ("\u{3B1}", 1),
                    ("\u{1018a}", 0),
                ],
                n,
            ),
            Self::GreekUpperAncient => additive(
                [
                    ("\u{3E0}", 900),
                    ("\u{3A9}", 800),
                    ("\u{3A8}", 700),
                    ("\u{3A7}", 600),
                    ("\u{3A6}", 500),
                    ("\u{3A5}", 400),
                    ("\u{3A4}", 300),
                    ("\u{3A3}", 200),
                    ("\u{3A1}", 100),
                    ("\u{3DE}", 90),
                    ("\u{3A0}", 80),
                    ("\u{39F}", 70),
                    ("\u{39E}", 60),
                    ("\u{39D}", 50),
                    ("\u{39C}", 40),
                    ("\u{39B}", 30),
                    ("\u{39A}", 20),
                    ("\u{399}", 10),
                    ("\u{398}", 9),
                    ("\u{397}", 8),
                    ("\u{396}", 7),
                    ("\u{3DA}", 6),
                    ("\u{395}", 5),
                    ("\u{394}", 4),
                    ("\u{393}", 3),
                    ("\u{392}", 2),
                    ("\u{391}", 1),
                    ("\u{1018a}", 0),
                ],
                n,
            ),
            Self::GreekUpperModern => additive(
                [
                    ("\u{3E0}", 900),
                    ("\u{3A9}", 800),
                    ("\u{3A8}", 700),
                    ("\u{3A7}", 600),
                    ("\u{3A6}", 500),
                    ("\u{3A5}", 400),
                    ("\u{3A4}", 300),
                    ("\u{3A3}", 200),
                    ("\u{3A1}", 100),
                    ("\u{3DE}", 90),
                    ("\u{3A0}", 80),
                    ("\u{39F}", 70),
                    ("\u{39E}", 60),
                    ("\u{39D}", 50),
                    ("\u{39C}", 40),
                    ("\u{39B}", 30),
                    ("\u{39A}", 20),
                    ("\u{399}", 10),
                    ("\u{398}", 9),
                    ("\u{397}", 8),
                    ("\u{396}", 7),
                    ("\u{3A3}\u{3A4}", 6),
                    ("\u{395}", 5),
                    ("\u{394}", 4),
                    ("\u{393}", 3),
                    ("\u{392}", 2),
                    ("\u{391}", 1),
                    ("\u{1018a}", 0),
                ],
                n,
            ),
            Self::Gujarati => numeric(
                [
                    '\u{AE6}', '\u{AE7}', '\u{AE8}', '\u{AE9}', '\u{AEA}', '\u{AEB}',
                    '\u{AEC}', '\u{AED}', '\u{AEE}', '\u{AEF}',
                ],
                n,
            ),
            Self::GujaratiAlpha => alphabetic(
                [
                    '\u{0A95}', '\u{0A96}', '\u{0A97}', '\u{0A98}', '\u{0A99}',
                    '\u{0A9A}', '\u{0A9B}', '\u{0A9C}', '\u{0A9D}', '\u{0A9E}',
                    '\u{0A9F}', '\u{0AA0}', '\u{0AA1}', '\u{0AA2}', '\u{0AA3}',
                    '\u{0AA4}', '\u{0AA5}', '\u{0AA6}', '\u{0AA7}', '\u{0AA8}',
                    '\u{0AAA}', '\u{0AAB}', '\u{0AAC}', '\u{0AAD}', '\u{0AAE}',
                    '\u{0AAF}', '\u{0AB0}', '\u{0AB2}', '\u{0AB5}', '\u{0AB6}',
                    '\u{0AB7}', '\u{0AB8}', '\u{0AB9}', '\u{0AB3}',
                ],
                n,
            ),
            Self::Gumuz => alphabetic(
                [
                    '\u{1200}', '\u{1210}', '\u{1208}', '\u{1210}', '\u{1218}',
                    '\u{1228}', '\u{1230}', '\u{1238}', '\u{1240}', '\u{1260}',
                    '\u{1268}', '\u{1270}', '\u{1278}', '\u{1290}', '\u{1298}',
                    '\u{1308}', '\u{1328}', '\u{1330}', '\u{1340}', '\u{1350}',
                ],
                n,
            ),
            Self::Gurmukhi => numeric(
                [
                    '\u{A66}', '\u{A67}', '\u{A68}', '\u{A69}', '\u{A6A}', '\u{A6B}',
                    '\u{A6C}', '\u{A6D}', '\u{A6E}', '\u{A6F}',
                ],
                n,
            ),
            Self::Hadiyya => alphabetic(
                [
                    '\u{1200}', '\u{1208}', '\u{1218}', '\u{1228}', '\u{1230}',
                    '\u{1238}', '\u{1240}', '\u{1260}', '\u{1270}', '\u{1278}',
                    '\u{1290}', '\u{1300}', '\u{1308}', '\u{1320}', '\u{1328}',
                    '\u{1330}', '\u{1348}', '\u{1350}',
                ],
                n,
            ),
            Self::Hangul => alphabetic(
                [
                    '\u{AC00}', '\u{B098}', '\u{B2E4}', '\u{B77C}', '\u{B9C8}',
                    '\u{BC14}', '\u{C0AC}', '\u{C544}', '\u{C790}', '\u{CC28}',
                    '\u{CE74}', '\u{D0C0}', '\u{D30C}', '\u{D558}',
                ],
                n,
            ),
            Self::HangulConsonant => alphabetic(
                [
                    '\u{3131}', '\u{3134}', '\u{3137}', '\u{3139}', '\u{3141}',
                    '\u{3142}', '\u{3145}', '\u{3147}', '\u{3148}', '\u{314A}',
                    '\u{314B}', '\u{314C}', '\u{314D}', '\u{314E}',
                ],
                n,
            ),
            Self::HanifiRohingya => numeric(
                [
                    '\u{10D30}',
                    '\u{10D31}',
                    '\u{10D32}',
                    '\u{10D33}',
                    '\u{10D34}',
                    '\u{10D35}',
                    '\u{10D36}',
                    '\u{10D37}',
                    '\u{10D38}',
                    '\u{10D39}',
                ],
                n,
            ),
            Self::Harari => alphabetic(
                [
                    '\u{1210}', '\u{1208}', '\u{1218}', '\u{1228}', '\u{1230}',
                    '\u{1238}', '\u{1240}', '\u{1260}', '\u{1270}', '\u{1278}',
                    '\u{1290}', '\u{1298}', '\u{1300}', '\u{1308}', '\u{1320}',
                    '\u{1328}', '\u{1348}',
                ],
                n,
            ),
            Self::Hebrew => additive(
                [
                    ("\u{5D9}\u{5F3}", 10000),
                    ("\u{5D8}\u{5F3}", 9000),
                    ("\u{5D7}\u{5F3}", 8000),
                    ("\u{5D6}\u{5F3}", 7000),
                    ("\u{5D5}\u{5F3}", 6000),
                    ("\u{5D4}\u{5F3}", 5000),
                    ("\u{5D3}\u{5F3}", 4000),
                    ("\u{5D2}\u{5F3}", 3000),
                    ("\u{5D1}\u{5F3}", 2000),
                    ("\u{5D0}\u{5F3}", 1000),
                    ("\u{5EA}", 400),
                    ("\u{5E9}", 300),
                    ("\u{5E8}", 200),
                    ("\u{5E7}", 100),
                    ("\u{5E6}", 90),
                    ("\u{5E4}", 80),
                    ("\u{5E2}", 70),
                    ("\u{5E1}", 60),
                    ("\u{5E0}", 50),
                    ("\u{5DE}", 40),
                    ("\u{5DC}", 30),
                    ("\u{5DB}", 20),
                    ("\u{5D9}\u{5D8}", 19),
                    ("\u{5D9}\u{5D7}", 18),
                    ("\u{5D9}\u{5D6}", 17),
                    ("\u{5D8}\u{5D6}", 16),
                    ("\u{5D8}\u{5D5}", 15),
                    ("\u{5D9}", 10),
                    ("\u{5D8}", 9),
                    ("\u{5D7}", 8),
                    ("\u{5D6}", 7),
                    ("\u{5D5}", 6),
                    ("\u{5D4}", 5),
                    ("\u{5D3}", 4),
                    ("\u{5D2}", 3),
                    ("\u{5D1}", 2),
                    ("\u{5D0}", 1),
                ],
                n,
            ),
            Self::Hindi => alphabetic(
                [
                    '\u{915}', '\u{916}', '\u{917}', '\u{918}', '\u{919}', '\u{91A}',
                    '\u{91B}', '\u{91C}', '\u{91D}', '\u{91E}', '\u{91F}', '\u{920}',
                    '\u{921}', '\u{922}', '\u{923}', '\u{924}', '\u{925}', '\u{926}',
                    '\u{927}', '\u{928}', '\u{92A}', '\u{92B}', '\u{92C}', '\u{92D}',
                    '\u{92E}', '\u{92F}', '\u{930}', '\u{932}', '\u{935}', '\u{936}',
                    '\u{937}', '\u{938}', '\u{939}',
                ],
                n,
            ),
            Self::Hiragana => alphabetic(
                [
                    '\u{3042}', '\u{3044}', '\u{3046}', '\u{3048}', '\u{304A}',
                    '\u{304B}', '\u{304D}', '\u{304F}', '\u{3051}', '\u{3053}',
                    '\u{3055}', '\u{3057}', '\u{3059}', '\u{305B}', '\u{305D}',
                    '\u{305F}', '\u{3061}', '\u{3064}', '\u{3066}', '\u{3068}',
                    '\u{306A}', '\u{306B}', '\u{306C}', '\u{306D}', '\u{306E}',
                    '\u{306F}', '\u{3072}', '\u{3075}', '\u{3078}', '\u{307B}',
                    '\u{307E}', '\u{307F}', '\u{3080}', '\u{3081}', '\u{3082}',
                    '\u{3084}', '\u{3086}', '\u{3088}', '\u{3089}', '\u{308A}',
                    '\u{308B}', '\u{308C}', '\u{308D}', '\u{308F}', '\u{3090}',
                    '\u{3091}', '\u{3092}', '\u{3093}',
                ],
                n,
            ),
            Self::HiraganaIroha => alphabetic(
                [
                    '\u{3044}', '\u{308D}', '\u{306F}', '\u{306B}', '\u{307B}',
                    '\u{3078}', '\u{3068}', '\u{3061}', '\u{308A}', '\u{306C}',
                    '\u{308B}', '\u{3092}', '\u{308F}', '\u{304B}', '\u{3088}',
                    '\u{305F}', '\u{308C}', '\u{305D}', '\u{3064}', '\u{306D}',
                    '\u{306A}', '\u{3089}', '\u{3080}', '\u{3046}', '\u{3090}',
                    '\u{306E}', '\u{304A}', '\u{304F}', '\u{3084}', '\u{307E}',
                    '\u{3051}', '\u{3075}', '\u{3053}', '\u{3048}', '\u{3066}',
                    '\u{3042}', '\u{3055}', '\u{304D}', '\u{3086}', '\u{3081}',
                    '\u{307F}', '\u{3057}', '\u{3091}', '\u{3072}', '\u{3082}',
                    '\u{305B}', '\u{3059}',
                ],
                n,
            ),
            Self::JapaneseFormal => additive(
                [
                    ("\u{4E5D}\u{9621}", 9000),
                    ("\u{516B}\u{9621}", 8000),
                    ("\u{4E03}\u{9621}", 7000),
                    ("\u{516D}\u{9621}", 6000),
                    ("\u{4F0D}\u{9621}", 5000),
                    ("\u{56DB}\u{9621}", 4000),
                    ("\u{53C2}\u{9621}", 3000),
                    ("\u{5F10}\u{9621}", 2000),
                    ("\u{58F1}\u{9621}", 1000),
                    ("\u{4E5D}\u{767E}", 900),
                    ("\u{516B}\u{767E}", 800),
                    ("\u{4E03}\u{767E}", 700),
                    ("\u{516D}\u{767E}", 600),
                    ("\u{4F0D}\u{767E}", 500),
                    ("\u{56DB}\u{767E}", 400),
                    ("\u{53C2}\u{767E}", 300),
                    ("\u{5F10}\u{767E}", 200),
                    ("\u{58F1}\u{767E}", 100),
                    ("\u{4E5D}\u{62FE}", 90),
                    ("\u{516B}\u{62FE}", 80),
                    ("\u{4E03}\u{62FE}", 70),
                    ("\u{516D}\u{62FE}", 60),
                    ("\u{4F0D}\u{62FE}", 50),
                    ("\u{56DB}\u{62FE}", 40),
                    ("\u{53C2}\u{62FE}", 30),
                    ("\u{5F10}\u{62FE}", 20),
                    ("\u{58F1}\u{62FE}", 10),
                    ("\u{4E5D}", 9),
                    ("\u{516B}", 8),
                    ("\u{4E03}", 7),
                    ("\u{516D}", 6),
                    ("\u{4F0D}", 5),
                    ("\u{56DB}", 4),
                    ("\u{53C2}", 3),
                    ("\u{5F10}", 2),
                    ("\u{58F1}", 1),
                    ("\u{96F6}", 0),
                ],
                n,
            ),
            Self::JapaneseInformal => additive(
                [
                    ("\u{4E5D}\u{5343}", 9000),
                    ("\u{516B}\u{5343}", 8000),
                    ("\u{4E03}\u{5343}", 7000),
                    ("\u{516D}\u{5343}", 6000),
                    ("\u{4E94}\u{5343}", 5000),
                    ("\u{56DB}\u{5343}", 4000),
                    ("\u{4E09}\u{5343}", 3000),
                    ("\u{4E8C}\u{5343}", 2000),
                    ("\u{5343}", 1000),
                    ("\u{4E5D}\u{767E}", 900),
                    ("\u{516B}\u{767E}", 800),
                    ("\u{4E03}\u{767E}", 700),
                    ("\u{516D}\u{767E}", 600),
                    ("\u{4E94}\u{767E}", 500),
                    ("\u{56DB}\u{767E}", 400),
                    ("\u{4E09}\u{767E}", 300),
                    ("\u{4E8C}\u{767E}", 200),
                    ("\u{767E}", 100),
                    ("\u{4E5D}\u{5341}", 90),
                    ("\u{516B}\u{5341}", 80),
                    ("\u{4E03}\u{5341}", 70),
                    ("\u{516D}\u{5341}", 60),
                    ("\u{4E94}\u{5341}", 50),
                    ("\u{56DB}\u{5341}", 40),
                    ("\u{4E09}\u{5341}", 30),
                    ("\u{4E8C}\u{5341}", 20),
                    ("\u{5341}", 10),
                    ("\u{4E5D}", 9),
                    ("\u{516B}", 8),
                    ("\u{4E03}", 7),
                    ("\u{516D}", 6),
                    ("\u{4E94}", 5),
                    ("\u{56DB}", 4),
                    ("\u{4E09}", 3),
                    ("\u{4E8C}", 2),
                    ("\u{4E00}", 1),
                    ("\u{3007}", 0),
                ],
                n,
            ),
            Self::Javanese => numeric(
                [
                    '\u{A9D0}', '\u{A9D1}', '\u{A9D2}', '\u{A9D3}', '\u{A9D4}',
                    '\u{A9D5}', '\u{A9D6}', '\u{A9D7}', '\u{A9D8}', '\u{A9D9}',
                ],
                n,
            ),
            Self::Kaffa => alphabetic(
                [
                    '\u{1200}', '\u{1208}', '\u{1210}', '\u{1218}', '\u{1220}',
                    '\u{1228}', '\u{1230}', '\u{1238}', '\u{1240}', '\u{1260}',
                    '\u{1270}', '\u{1278}', '\u{1280}', '\u{1290}', '\u{1300}',
                    '\u{1308}', '\u{1320}', '\u{1328}', '\u{1330}', '\u{1348}',
                    '\u{1350}',
                ],
                n,
            ),
            Self::Kannada => numeric(
                [
                    '\u{CE6}', '\u{CE7}', '\u{CE8}', '\u{CE9}', '\u{CEA}', '\u{CEB}',
                    '\u{CEC}', '\u{CED}', '\u{CEE}', '\u{CEF}',
                ],
                n,
            ),
            Self::KannadaAlpha => alphabetic(
                [
                    '\u{0C85}', '\u{0C86}', '\u{0C87}', '\u{0C88}', '\u{0C89}',
                    '\u{0C8A}', '\u{0C8B}', '\u{0C8E}', '\u{0C8F}', '\u{0C90}',
                    '\u{0C92}', '\u{0C93}', '\u{0C94}', '\u{0C95}', '\u{0C96}',
                    '\u{0C97}', '\u{0C98}', '\u{0C99}',
                ],
                n,
            ),
            Self::Kashmiri => alphabetic(
                [
                    '\u{0627}', '\u{0622}', '\u{0628}', '\u{067E}', '\u{062A}',
                    '\u{0679}', '\u{062B}', '\u{062C}', '\u{0686}', '\u{062D}',
                    '\u{062E}', '\u{062F}', '\u{0688}', '\u{0630}', '\u{0631}',
                    '\u{0691}', '\u{0632}', '\u{0698}', '\u{0633}', '\u{0634}',
                    '\u{0635}', '\u{0636}', '\u{0637}', '\u{0638}', '\u{0639}',
                    '\u{063A}', '\u{0641}', '\u{0642}', '\u{06A9}', '\u{06AF}',
                    '\u{0644}', '\u{0645}', '\u{0646}', '\u{06BA}', '\u{0648}',
                    '\u{06C1}', '\u{06BE}', '\u{0621}', '\u{06CC}', '\u{06D2}',
                    '\u{06C4}', '\u{0620}',
                ],
                n,
            ),
            Self::Katakana => alphabetic(
                [
                    '\u{30A2}', '\u{30A4}', '\u{30A6}', '\u{30A8}', '\u{30AA}',
                    '\u{30AB}', '\u{30AD}', '\u{30AF}', '\u{30B1}', '\u{30B3}',
                    '\u{30B5}', '\u{30B7}', '\u{30B9}', '\u{30BB}', '\u{30BD}',
                    '\u{30BF}', '\u{30C1}', '\u{30C4}', '\u{30C6}', '\u{30C8}',
                    '\u{30CA}', '\u{30CB}', '\u{30CC}', '\u{30CD}', '\u{30CE}',
                    '\u{30CF}', '\u{30D2}', '\u{30D5}', '\u{30D8}', '\u{30DB}',
                    '\u{30DE}', '\u{30DF}', '\u{30E0}', '\u{30E1}', '\u{30E2}',
                    '\u{30E4}', '\u{30E6}', '\u{30E8}', '\u{30E9}', '\u{30EA}',
                    '\u{30EB}', '\u{30EC}', '\u{30ED}', '\u{30EF}', '\u{30F0}',
                    '\u{30F1}', '\u{30F2}', '\u{30F3}',
                ],
                n,
            ),
            Self::KatakanaIroha => alphabetic(
                [
                    '\u{30A4}', '\u{30ED}', '\u{30CF}', '\u{30CB}', '\u{30DB}',
                    '\u{30D8}', '\u{30C8}', '\u{30C1}', '\u{30EA}', '\u{30CC}',
                    '\u{30EB}', '\u{30F2}', '\u{30EF}', '\u{30AB}', '\u{30E8}',
                    '\u{30BF}', '\u{30EC}', '\u{30BD}', '\u{30C4}', '\u{30CD}',
                    '\u{30CA}', '\u{30E9}', '\u{30E0}', '\u{30A6}', '\u{30F0}',
                    '\u{30CE}', '\u{30AA}', '\u{30AF}', '\u{30E4}', '\u{30DE}',
                    '\u{30B1}', '\u{30D5}', '\u{30B3}', '\u{30A8}', '\u{30C6}',
                    '\u{30A2}', '\u{30B5}', '\u{30AD}', '\u{30E6}', '\u{30E1}',
                    '\u{30DF}', '\u{30B7}', '\u{30F1}', '\u{30D2}', '\u{30E2}',
                    '\u{30BB}', '\u{30B9}',
                ],
                n,
            ),
            Self::KayahLi => numeric(
                [
                    '\u{A901}', '\u{A902}', '\u{A903}', '\u{A904}', '\u{A905}',
                    '\u{A906}', '\u{A907}', '\u{A908}', '\u{A909}', '\u{A900}',
                ],
                n,
            ),
            Self::Kebena => alphabetic(
                [
                    '\u{1200}', '\u{1208}', '\u{1218}', '\u{1228}', '\u{1230}',
                    '\u{1238}', '\u{1240}', '\u{1260}', '\u{1270}', '\u{1278}',
                    '\u{1290}', '\u{1300}', '\u{1308}', '\u{1320}', '\u{1328}',
                    '\u{1330}', '\u{1348}', '\u{1350}',
                ],
                n,
            ),
            Self::Kembata => alphabetic(
                [
                    '\u{1200}', '\u{1208}', '\u{1218}', '\u{1228}', '\u{1230}',
                    '\u{1238}', '\u{1240}', '\u{1260}', '\u{1268}', '\u{1270}',
                    '\u{1278}', '\u{1290}', '\u{1300}', '\u{1308}', '\u{1320}',
                    '\u{1328}', '\u{1330}', '\u{1348}',
                ],
                n,
            ),
            Self::Khmer => numeric(
                [
                    '\u{17E0}', '\u{17E1}', '\u{17E2}', '\u{17E3}', '\u{17E4}',
                    '\u{17E5}', '\u{17E6}', '\u{17E7}', '\u{17E8}', '\u{17E9}',
                ],
                n,
            ),
            Self::KhmerConsonant => alphabetic(
                [
                    '\u{1780}', '\u{1781}', '\u{1782}', '\u{1783}', '\u{1784}',
                    '\u{1785}', '\u{1786}', '\u{1787}', '\u{1788}', '\u{1789}',
                    '\u{178A}', '\u{178B}', '\u{178C}', '\u{178D}', '\u{178E}',
                    '\u{178F}', '\u{1790}', '\u{1791}', '\u{1792}', '\u{1793}',
                    '\u{1794}', '\u{1795}', '\u{1796}', '\u{1797}', '\u{1798}',
                    '\u{1799}', '\u{179A}', '\u{179B}', '\u{179C}', '\u{179F}',
                    '\u{17A0}', '\u{17A1}', '\u{17A2}',
                ],
                n,
            ),
            Self::Konkani => alphabetic(
                [
                    '\u{915}', '\u{916}', '\u{917}', '\u{918}', '\u{919}', '\u{91A}',
                    '\u{91B}', '\u{91C}', '\u{91D}', '\u{91E}', '\u{91F}', '\u{920}',
                    '\u{921}', '\u{922}', '\u{923}', '\u{924}', '\u{925}', '\u{926}',
                    '\u{927}', '\u{928}', '\u{92A}', '\u{92B}', '\u{92C}', '\u{92D}',
                    '\u{92E}', '\u{92F}', '\u{930}', '\u{932}', '\u{935}', '\u{936}',
                    '\u{937}', '\u{938}', '\u{939}', '\u{933}',
                ],
                n,
            ),
            Self::Konso => alphabetic(
                [
                    '\u{1200}', '\u{1208}', '\u{1218}', '\u{1228}', '\u{1230}',
                    '\u{1238}', '\u{1240}', '\u{1260}', '\u{1270}', '\u{1278}',
                    '\u{1290}', '\u{1298}', '\u{1300}', '\u{1348}', '\u{1350}',
                ],
                n,
            ),
            Self::KoreanConsonant => alphabetic(
                [
                    '\u{3131}', '\u{3134}', '\u{3137}', '\u{3139}', '\u{3141}',
                    '\u{3142}', '\u{3145}', '\u{3147}', '\u{3148}', '\u{314A}',
                    '\u{314B}', '\u{314C}', '\u{314D}', '\u{314E}',
                ],
                n,
            ),
            Self::KoreanHangulFormal => additive(
                [
                    ("\u{AD6C}\u{CC9C}", 9000),
                    ("\u{D314}\u{CC9C}", 8000),
                    ("\u{CE60}\u{CC9C}", 7000),
                    ("\u{C721}\u{CC9C}", 6000),
                    ("\u{C624}\u{CC9C}", 5000),
                    ("\u{C0AC}\u{CC9C}", 4000),
                    ("\u{C0BC}\u{CC9C}", 3000),
                    ("\u{C774}\u{CC9C}", 2000),
                    ("\u{C77C}\u{CC9C}", 1000),
                    ("\u{AD6C}\u{BC31}", 900),
                    ("\u{D314}\u{BC31}", 800),
                    ("\u{CE60}\u{BC31}", 700),
                    ("\u{C721}\u{BC31}", 600),
                    ("\u{C624}\u{BC31}", 500),
                    ("\u{C0AC}\u{BC31}", 400),
                    ("\u{C0BC}\u{BC31}", 300),
                    ("\u{C774}\u{BC31}", 200),
                    ("\u{C77C}\u{BC31}", 100),
                    ("\u{AD6C}\u{C2ED}", 90),
                    ("\u{D314}\u{C2ED}", 80),
                    ("\u{CE60}\u{C2ED}", 70),
                    ("\u{C721}\u{C2ED}", 60),
                    ("\u{C624}\u{C2ED}", 50),
                    ("\u{C0AC}\u{C2ED}", 40),
                    ("\u{C0BC}\u{C2ED}", 30),
                    ("\u{C774}\u{C2ED}", 20),
                    ("\u{C77C}\u{C2ED}", 10),
                    ("\u{AD6C}", 9),
                    ("\u{D314}", 8),
                    ("\u{CE60}", 7),
                    ("\u{C721}", 6),
                    ("\u{C624}", 5),
                    ("\u{C0AC}", 4),
                    ("\u{C0BC}", 3),
                    ("\u{C774}", 2),
                    ("\u{C77C}", 1),
                    ("\u{C601}", 0),
                ],
                n,
            ),
            Self::KoreanHanjaFormal => additive(
                [
                    ("\u{4E5D}\u{4EDF}", 9000),
                    ("\u{516B}\u{4EDF}", 8000),
                    ("\u{4E03}\u{4EDF}", 7000),
                    ("\u{516D}\u{4EDF}", 6000),
                    ("\u{4E94}\u{4EDF}", 5000),
                    ("\u{56DB}\u{4EDF}", 4000),
                    ("\u{53C3}\u{4EDF}", 3000),
                    ("\u{8CB3}\u{4EDF}", 2000),
                    ("\u{58F9}\u{4EDF}", 1000),
                    ("\u{4E5D}\u{767E}", 900),
                    ("\u{516B}\u{767E}", 800),
                    ("\u{4E03}\u{767E}", 700),
                    ("\u{516D}\u{767E}", 600),
                    ("\u{4E94}\u{767E}", 500),
                    ("\u{56DB}\u{767E}", 400),
                    ("\u{53C3}\u{767E}", 300),
                    ("\u{8CB3}\u{767E}", 200),
                    ("\u{58F9}\u{767E}", 100),
                    ("\u{4E5D}\u{62FE}", 90),
                    ("\u{516B}\u{62FE}", 80),
                    ("\u{4E03}\u{62FE}", 70),
                    ("\u{516D}\u{62FE}", 60),
                    ("\u{4E94}\u{62FE}", 50),
                    ("\u{56DB}\u{62FE}", 40),
                    ("\u{53C3}\u{62FE}", 30),
                    ("\u{8CB3}\u{62FE}", 20),
                    ("\u{58F9}\u{62FE}", 10),
                    ("\u{4E5D}", 9),
                    ("\u{516B}", 8),
                    ("\u{4E03}", 7),
                    ("\u{516D}", 6),
                    ("\u{4E94}", 5),
                    ("\u{56DB}", 4),
                    ("\u{53C3}", 3),
                    ("\u{8CB3}", 2),
                    ("\u{58F9}", 1),
                    ("\u{96F6}", 0),
                ],
                n,
            ),
            Self::KoreanHanjaInformal => additive(
                [
                    ("\u{4E5D}\u{5343}", 9000),
                    ("\u{516B}\u{5343}", 8000),
                    ("\u{4E03}\u{5343}", 7000),
                    ("\u{516D}\u{5343}", 6000),
                    ("\u{4E94}\u{5343}", 5000),
                    ("\u{56DB}\u{5343}", 4000),
                    ("\u{4E09}\u{5343}", 3000),
                    ("\u{4E8C}\u{5343}", 2000),
                    ("\u{5343}", 1000),
                    ("\u{4E5D}\u{767E}", 900),
                    ("\u{516B}\u{767E}", 800),
                    ("\u{4E03}\u{767E}", 700),
                    ("\u{516D}\u{767E}", 600),
                    ("\u{4E94}\u{767E}", 500),
                    ("\u{56DB}\u{767E}", 400),
                    ("\u{4E09}\u{767E}", 300),
                    ("\u{4E8C}\u{767E}", 200),
                    ("\u{767E}", 100),
                    ("\u{4E5D}\u{5341}", 90),
                    ("\u{516B}\u{5341}", 80),
                    ("\u{4E03}\u{5341}", 70),
                    ("\u{516D}\u{5341}", 60),
                    ("\u{4E94}\u{5341}", 50),
                    ("\u{56DB}\u{5341}", 40),
                    ("\u{4E09}\u{5341}", 30),
                    ("\u{4E8C}\u{5341}", 20),
                    ("\u{5341}", 10),
                    ("\u{4E5D}", 9),
                    ("\u{516B}", 8),
                    ("\u{4E03}", 7),
                    ("\u{516D}", 6),
                    ("\u{4E94}", 5),
                    ("\u{56DB}", 4),
                    ("\u{4E09}", 3),
                    ("\u{4E8C}", 2),
                    ("\u{4E00}", 1),
                    ("\u{96F6}", 0),
                ],
                n,
            ),
            Self::KoreanSyllable => alphabetic(
                [
                    '\u{AC00}', '\u{B098}', '\u{B2E4}', '\u{B77C}', '\u{B9C8}',
                    '\u{BC14}', '\u{C0AC}', '\u{C544}', '\u{C790}', '\u{CC28}',
                    '\u{CE74}', '\u{D0C0}', '\u{D30C}', '\u{D558}',
                ],
                n,
            ),
            Self::Kunama => alphabetic(
                [
                    '\u{1200}', '\u{1208}', '\u{1218}', '\u{1228}', '\u{1230}',
                    '\u{1238}', '\u{1260}', '\u{1270}', '\u{1278}', '\u{1290}',
                    '\u{1298}', '\u{1300}', '\u{1308}',
                ],
                n,
            ),
            Self::LannaHora => numeric(
                [
                    '\u{1A80}', '\u{1A81}', '\u{1A82}', '\u{1A83}', '\u{1A84}',
                    '\u{1A85}', '\u{1A86}', '\u{1A87}', '\u{1A88}', '\u{1A89}',
                ],
                n,
            ),
            Self::LannaTham => numeric(
                [
                    '\u{1A90}', '\u{1A91}', '\u{1A92}', '\u{1A93}', '\u{1A94}',
                    '\u{1A95}', '\u{1A96}', '\u{1A97}', '\u{1A98}', '\u{1A99}',
                ],
                n,
            ),
            Self::Lao => numeric(
                [
                    '\u{ED0}', '\u{ED1}', '\u{ED2}', '\u{ED3}', '\u{ED4}', '\u{ED5}',
                    '\u{ED6}', '\u{ED7}', '\u{ED8}', '\u{ED9}',
                ],
                n,
            ),
            Self::Lepcha => numeric(
                [
                    '\u{1C40}', '\u{1C41}', '\u{1C42}', '\u{1C43}', '\u{1C44}',
                    '\u{1C45}', '\u{1C46}', '\u{1C47}', '\u{1C48}', '\u{1C49}',
                ],
                n,
            ),
            Self::Limbu => numeric(
                [
                    '\u{1946}', '\u{1947}', '\u{1948}', '\u{1949}', '\u{194A}',
                    '\u{194B}', '\u{194C}', '\u{194D}', '\u{194E}', '\u{194F}',
                ],
                n,
            ),
            Self::LowerAlpha => alphabetic(
                [
                    '\u{61}', '\u{62}', '\u{63}', '\u{64}', '\u{65}', '\u{66}', '\u{67}',
                    '\u{68}', '\u{69}', '\u{6A}', '\u{6B}', '\u{6C}', '\u{6D}', '\u{6E}',
                    '\u{6F}', '\u{70}', '\u{71}', '\u{72}', '\u{73}', '\u{74}', '\u{75}',
                    '\u{76}', '\u{77}', '\u{78}', '\u{79}', '\u{7A}',
                ],
                n,
            ),
            Self::LowerAlphaSymbolic => symbolic(
                [
                    '\u{61}', '\u{62}', '\u{63}', '\u{64}', '\u{65}', '\u{66}', '\u{67}',
                    '\u{68}', '\u{69}', '\u{6A}', '\u{6B}', '\u{6C}', '\u{6D}', '\u{6E}',
                    '\u{6F}', '\u{70}', '\u{71}', '\u{72}', '\u{73}', '\u{74}', '\u{75}',
                    '\u{76}', '\u{77}', '\u{78}', '\u{79}', '\u{7A}',
                ],
                n,
            ),
            Self::LowerArmenian => additive(
                [
                    ("\u{584}", 9000),
                    ("\u{583}", 8000),
                    ("\u{582}", 7000),
                    ("\u{581}", 6000),
                    ("\u{580}", 5000),
                    ("\u{57F}", 4000),
                    ("\u{57E}", 3000),
                    ("\u{57D}", 2000),
                    ("\u{57C}", 1000),
                    ("\u{57B}", 900),
                    ("\u{57A}", 800),
                    ("\u{579}", 700),
                    ("\u{578}", 600),
                    ("\u{577}", 500),
                    ("\u{576}", 400),
                    ("\u{575}", 300),
                    ("\u{574}", 200),
                    ("\u{573}", 100),
                    ("\u{572}", 90),
                    ("\u{571}", 80),
                    ("\u{570}", 70),
                    ("\u{56F}", 60),
                    ("\u{56E}", 50),
                    ("\u{56D}", 40),
                    ("\u{56C}", 30),
                    ("\u{56B}", 20),
                    ("\u{56A}", 10),
                    ("\u{569}", 9),
                    ("\u{568}", 8),
                    ("\u{567}", 7),
                    ("\u{566}", 6),
                    ("\u{565}", 5),
                    ("\u{564}", 4),
                    ("\u{563}", 3),
                    ("\u{562}", 2),
                    ("\u{561}", 1),
                ],
                n,
            ),
            Self::LowerBelorussian => alphabetic(
                [
                    '\u{430}', '\u{431}', '\u{432}', '\u{433}', '\u{434}', '\u{435}',
                    '\u{451}', '\u{436}', '\u{437}', '\u{456}', '\u{439}', '\u{43A}',
                    '\u{43B}', '\u{43C}', '\u{43D}', '\u{43E}', '\u{43F}', '\u{440}',
                    '\u{441}', '\u{442}', '\u{443}', '\u{45E}', '\u{444}', '\u{445}',
                    '\u{446}', '\u{447}', '\u{448}', '\u{44B}', '\u{44C}', '\u{44D}',
                    '\u{44E}', '\u{44F}',
                ],
                n,
            ),
            Self::LowerBulgarian => alphabetic(
                [
                    '\u{430}', '\u{431}', '\u{432}', '\u{433}', '\u{434}', '\u{435}',
                    '\u{436}', '\u{437}', '\u{438}', '\u{439}', '\u{43A}', '\u{43B}',
                    '\u{43C}', '\u{43D}', '\u{43E}', '\u{43F}', '\u{440}', '\u{441}',
                    '\u{442}', '\u{443}', '\u{444}', '\u{445}', '\u{446}', '\u{447}',
                    '\u{448}', '\u{449}', '\u{44A}', '\u{44C}', '\u{44E}', '\u{44F}',
                ],
                n,
            ),
            Self::LowerGreek => alphabetic(
                [
                    '\u{3B1}', '\u{3B2}', '\u{3B3}', '\u{3B4}', '\u{3B5}', '\u{3B6}',
                    '\u{3B7}', '\u{3B8}', '\u{3B9}', '\u{3BA}', '\u{3BB}', '\u{3BC}',
                    '\u{3BD}', '\u{3BE}', '\u{3BF}', '\u{3C0}', '\u{3C1}', '\u{3C3}',
                    '\u{3C4}', '\u{3C5}', '\u{3C6}', '\u{3C7}', '\u{3C8}', '\u{3C9}',
                ],
                n,
            ),
            Self::LowerHexadecimal => numeric(
                [
                    '\u{30}', '\u{31}', '\u{32}', '\u{33}', '\u{34}', '\u{35}', '\u{36}',
                    '\u{37}', '\u{38}', '\u{39}', '\u{61}', '\u{62}', '\u{63}', '\u{64}',
                    '\u{65}', '\u{66}',
                ],
                n,
            ),
            Self::LowerMacedonian => alphabetic(
                [
                    '\u{430}', '\u{431}', '\u{432}', '\u{433}', '\u{434}', '\u{453}',
                    '\u{435}', '\u{436}', '\u{437}', '\u{455}', '\u{438}', '\u{458}',
                    '\u{43A}', '\u{43B}', '\u{459}', '\u{43C}', '\u{43D}', '\u{45A}',
                    '\u{43E}', '\u{43F}', '\u{440}', '\u{441}', '\u{442}', '\u{45C}',
                    '\u{443}', '\u{444}', '\u{445}', '\u{446}', '\u{447}', '\u{45F}',
                    '\u{448}',
                ],
                n,
            ),
            Self::LowerRoman => additive(
                [
                    ("\u{6D}", 1000),
                    ("\u{63}\u{6D}", 900),
                    ("\u{64}", 500),
                    ("\u{63}\u{64}", 400),
                    ("\u{63}", 100),
                    ("\u{78}\u{63}", 90),
                    ("\u{6C}", 50),
                    ("\u{78}\u{6C}", 40),
                    ("\u{78}", 10),
                    ("\u{69}\u{78}", 9),
                    ("\u{76}", 5),
                    ("\u{69}\u{76}", 4),
                    ("\u{69}", 1),
                ],
                n,
            ),
            Self::LowerRussian => alphabetic(
                [
                    '\u{430}', '\u{431}', '\u{432}', '\u{433}', '\u{434}', '\u{435}',
                    '\u{436}', '\u{437}', '\u{438}', '\u{43A}', '\u{43B}', '\u{43C}',
                    '\u{43D}', '\u{43E}', '\u{43F}', '\u{440}', '\u{441}', '\u{442}',
                    '\u{443}', '\u{444}', '\u{445}', '\u{446}', '\u{447}', '\u{448}',
                    '\u{449}', '\u{44D}', '\u{44E}', '\u{44F}',
                ],
                n,
            ),
            Self::LowerRussianFull => alphabetic(
                [
                    '\u{430}', '\u{431}', '\u{432}', '\u{433}', '\u{434}', '\u{435}',
                    '\u{451}', '\u{436}', '\u{437}', '\u{438}', '\u{439}', '\u{43A}',
                    '\u{43B}', '\u{43C}', '\u{43D}', '\u{43E}', '\u{43F}', '\u{440}',
                    '\u{441}', '\u{442}', '\u{443}', '\u{444}', '\u{445}', '\u{446}',
                    '\u{447}', '\u{448}', '\u{449}', '\u{44A}', '\u{44B}', '\u{44C}',
                    '\u{44D}', '\u{44E}', '\u{44F}',
                ],
                n,
            ),
            Self::LowerSerbian => alphabetic(
                [
                    '\u{430}', '\u{431}', '\u{432}', '\u{433}', '\u{434}', '\u{452}',
                    '\u{435}', '\u{436}', '\u{437}', '\u{438}', '\u{458}', '\u{43A}',
                    '\u{43B}', '\u{459}', '\u{43C}', '\u{43D}', '\u{45A}', '\u{43E}',
                    '\u{43F}', '\u{440}', '\u{441}', '\u{442}', '\u{45B}', '\u{443}',
                    '\u{444}', '\u{445}', '\u{446}', '\u{447}', '\u{45F}', '\u{448}',
                ],
                n,
            ),
            Self::LowerUkrainian => alphabetic(
                [
                    '\u{430}', '\u{431}', '\u{432}', '\u{433}', '\u{434}', '\u{435}',
                    '\u{454}', '\u{436}', '\u{437}', '\u{438}', '\u{456}', '\u{43A}',
                    '\u{43B}', '\u{43C}', '\u{43D}', '\u{43E}', '\u{43F}', '\u{440}',
                    '\u{441}', '\u{442}', '\u{443}', '\u{444}', '\u{445}', '\u{446}',
                    '\u{447}', '\u{448}', '\u{44E}', '\u{44F}',
                ],
                n,
            ),
            Self::LowerUkrainianFull => alphabetic(
                [
                    '\u{430}', '\u{431}', '\u{432}', '\u{433}', '\u{491}', '\u{434}',
                    '\u{435}', '\u{454}', '\u{436}', '\u{437}', '\u{438}', '\u{456}',
                    '\u{457}', '\u{439}', '\u{43A}', '\u{43B}', '\u{43C}', '\u{43D}',
                    '\u{43E}', '\u{43F}', '\u{440}', '\u{441}', '\u{442}', '\u{443}',
                    '\u{444}', '\u{445}', '\u{446}', '\u{447}', '\u{448}', '\u{449}',
                    '\u{44C}', '\u{44E}', '\u{44F}',
                ],
                n,
            ),
            Self::MaghrebiAbjad => fixed(
                [
                    '\u{627}', '\u{628}', '\u{62C}', '\u{62F}', '\u{647}', '\u{648}',
                    '\u{632}', '\u{62D}', '\u{637}', '\u{64A}', '\u{643}', '\u{644}',
                    '\u{645}', '\u{646}', '\u{636}', '\u{639}', '\u{641}', '\u{635}',
                    '\u{642}', '\u{631}', '\u{633}', '\u{62A}', '\u{62B}', '\u{62E}',
                    '\u{630}', '\u{638}', '\u{63A}', '\u{634}',
                ],
                n,
            ),
            Self::Maithili => alphabetic(
                [
                    '\u{915}', '\u{916}', '\u{917}', '\u{918}', '\u{919}', '\u{91A}',
                    '\u{91B}', '\u{91C}', '\u{91D}', '\u{91E}', '\u{91F}', '\u{920}',
                    '\u{921}', '\u{922}', '\u{923}', '\u{924}', '\u{925}', '\u{926}',
                    '\u{927}', '\u{928}', '\u{92A}', '\u{92B}', '\u{92C}', '\u{92D}',
                    '\u{92E}', '\u{92F}', '\u{930}', '\u{932}', '\u{935}', '\u{936}',
                    '\u{937}', '\u{938}', '\u{939}',
                ],
                n,
            ),
            Self::Malayalam => numeric(
                [
                    '\u{D66}', '\u{D67}', '\u{D68}', '\u{D69}', '\u{D6A}', '\u{D6B}',
                    '\u{D6C}', '\u{D6D}', '\u{D6E}', '\u{D6F}',
                ],
                n,
            ),
            Self::MalayalamAlpha => alphabetic(
                [
                    '\u{D15}', '\u{D7F}', '\u{D16}', '\u{D17}', '\u{D18}', '\u{D19}',
                    '\u{D1A}', '\u{D1B}', '\u{D1C}', '\u{D1D}', '\u{D1E}', '\u{D1F}',
                    '\u{D20}', '\u{D21}', '\u{D22}', '\u{D23}', '\u{D7A}', '\u{D24}',
                    '\u{D25}', '\u{D26}', '\u{D27}', '\u{D28}', '\u{D7B}', '\u{D2A}',
                    '\u{D2B}', '\u{D2C}', '\u{D2D}', '\u{D2E}', '\u{D2F}', '\u{D30}',
                    '\u{D7C}', '\u{D32}', '\u{D7D}', '\u{D35}', '\u{D36}', '\u{D37}',
                    '\u{D38}', '\u{D39}', '\u{D33}', '\u{D7E}', '\u{D34}', '\u{D31}',
                ],
                n,
            ),
            Self::Manipuri => alphabetic(
                [
                    '\u{ABC0}', '\u{ABC1}', '\u{ABC2}', '\u{ABC3}', '\u{ABC4}',
                    '\u{ABC5}', '\u{ABC6}', '\u{ABC7}', '\u{ABC8}', '\u{ABC9}',
                    '\u{ABCA}', '\u{ABCB}', '\u{ABCC}', '\u{ABCD}', '\u{ABCE}',
                    '\u{ABCF}', '\u{ABD0}', '\u{ABD1}', '\u{ABD2}', '\u{ABD3}',
                    '\u{ABD4}', '\u{ABD5}', '\u{ABD6}', '\u{ABD7}', '\u{ABD8}',
                    '\u{ABD9}', '\u{ABDA}',
                ],
                n,
            ),
            Self::Marathi => alphabetic(
                [
                    '\u{915}', '\u{916}', '\u{917}', '\u{918}', '\u{919}', '\u{91A}',
                    '\u{91B}', '\u{91C}', '\u{91D}', '\u{91E}', '\u{91F}', '\u{920}',
                    '\u{921}', '\u{922}', '\u{923}', '\u{924}', '\u{925}', '\u{926}',
                    '\u{927}', '\u{928}', '\u{92A}', '\u{92B}', '\u{92C}', '\u{92D}',
                    '\u{92E}', '\u{92F}', '\u{930}', '\u{932}', '\u{935}', '\u{936}',
                    '\u{937}', '\u{938}', '\u{939}', '\u{933}',
                ],
                n,
            ),
            Self::Meen => alphabetic(
                [
                    '\u{1200}', '\u{1208}', '\u{1218}', '\u{1228}', '\u{1230}',
                    '\u{1238}', '\u{1240}', '\u{1260}', '\u{1270}', '\u{1278}',
                    '\u{1280}', '\u{1290}', '\u{1298}', '\u{1300}', '\u{1308}',
                    '\u{1320}', '\u{1328}', '\u{1330}', '\u{1350}', '\u{1340}',
                ],
                n,
            ),
            Self::Meetei => numeric(
                [
                    '\u{ABF0}', '\u{ABF1}', '\u{ABF2}', '\u{ABF3}', '\u{ABF4}',
                    '\u{ABF5}', '\u{ABF6}', '\u{ABF7}', '\u{ABF8}', '\u{ABF9}',
                ],
                n,
            ),
            Self::Mongolian => numeric(
                [
                    '\u{1810}', '\u{1811}', '\u{1812}', '\u{1813}', '\u{1814}',
                    '\u{1815}', '\u{1816}', '\u{1817}', '\u{1818}', '\u{1819}',
                ],
                n,
            ),
            Self::Mro => numeric(
                [
                    '\u{016A60}',
                    '\u{016A61}',
                    '\u{016A62}',
                    '\u{016A63}',
                    '\u{016A64}',
                    '\u{016A65}',
                    '\u{016A66}',
                    '\u{016A67}',
                    '\u{016A68}',
                    '\u{016A69}',
                ],
                n,
            ),
            Self::Myanmar => numeric(
                [
                    '\u{1040}', '\u{1041}', '\u{1042}', '\u{1043}', '\u{1044}',
                    '\u{1045}', '\u{1046}', '\u{1047}', '\u{1048}', '\u{1049}',
                ],
                n,
            ),
            Self::NagMundari => numeric(
                [
                    '\u{01E4F0}',
                    '\u{01E4F1}',
                    '\u{01E4F2}',
                    '\u{01E4F3}',
                    '\u{01E4F4}',
                    '\u{01E4F5}',
                    '\u{01E4F6}',
                    '\u{01E4F7}',
                    '\u{01E4F8}',
                    '\u{01E4F9}',
                ],
                n,
            ),
            Self::NewBase60 => numeric(
                [
                    '\u{30}', '\u{31}', '\u{32}', '\u{33}', '\u{34}', '\u{35}', '\u{36}',
                    '\u{37}', '\u{38}', '\u{39}', '\u{41}', '\u{42}', '\u{43}', '\u{44}',
                    '\u{45}', '\u{46}', '\u{47}', '\u{48}', '\u{4A}', '\u{4B}', '\u{4C}',
                    '\u{4D}', '\u{4E}', '\u{50}', '\u{51}', '\u{52}', '\u{53}', '\u{54}',
                    '\u{55}', '\u{56}', '\u{57}', '\u{58}', '\u{59}', '\u{5A}', '\u{5F}',
                    '\u{61}', '\u{62}', '\u{63}', '\u{64}', '\u{65}', '\u{66}', '\u{67}',
                    '\u{68}', '\u{69}', '\u{6A}', '\u{6B}', '\u{6D}', '\u{6E}', '\u{6F}',
                    '\u{70}', '\u{71}', '\u{72}', '\u{73}', '\u{74}', '\u{75}', '\u{76}',
                    '\u{77}', '\u{78}', '\u{79}', '\u{7A}',
                ],
                n,
            ),
            Self::Newa => numeric(
                [
                    '\u{011450}',
                    '\u{011451}',
                    '\u{011452}',
                    '\u{011453}',
                    '\u{011454}',
                    '\u{011455}',
                    '\u{011456}',
                    '\u{011457}',
                    '\u{011458}',
                    '\u{011459}',
                ],
                n,
            ),
            Self::NkoCardinal => numeric(
                [
                    '\u{07C1}', '\u{07C2}', '\u{07C3}', '\u{07C4}', '\u{07C5}',
                    '\u{07C6}', '\u{07C7}', '\u{07C8}', '\u{07C9}', '\u{07C0}',
                ],
                n,
            ),
            Self::Octal => numeric(
                [
                    '\u{30}', '\u{31}', '\u{32}', '\u{33}', '\u{34}', '\u{35}', '\u{36}',
                    '\u{37}',
                ],
                n,
            ),
            Self::OlChiki => numeric(
                [
                    '\u{1C50}', '\u{1C51}', '\u{1C52}', '\u{1C53}', '\u{1C54}',
                    '\u{1C55}', '\u{1C56}', '\u{1C57}', '\u{1C58}', '\u{1C59}',
                ],
                n,
            ),
            Self::Oriya => numeric(
                [
                    '\u{B66}', '\u{B67}', '\u{B68}', '\u{B69}', '\u{B6A}', '\u{B6B}',
                    '\u{B6C}', '\u{B6D}', '\u{B6E}', '\u{B6F}',
                ],
                n,
            ),
            Self::Oromo => alphabetic(
                [
                    '\u{1200}', '\u{1208}', '\u{1218}', '\u{1228}', '\u{1230}',
                    '\u{1238}', '\u{1240}', '\u{1260}', '\u{1270}', '\u{1278}',
                    '\u{1290}', '\u{1298}', '\u{12A0}', '\u{12A8}', '\u{12C8}',
                    '\u{12E8}', '\u{12F0}', '\u{12F8}', '\u{1300}', '\u{1308}',
                    '\u{1320}', '\u{1328}', '\u{1330}', '\u{1338}', '\u{1348}',
                ],
                n,
            ),
            Self::ParenthesizedDecimal => fixed(
                [
                    '\u{2474}', '\u{2475}', '\u{2476}', '\u{2477}', '\u{2478}',
                    '\u{2479}', '\u{247A}', '\u{247B}', '\u{247C}', '\u{247D}',
                    '\u{247E}', '\u{247F}', '\u{2480}', '\u{2481}', '\u{2482}',
                    '\u{2483}', '\u{2484}', '\u{2485}', '\u{2486}', '\u{2487}',
                ],
                n,
            ),
            Self::ParenthesizedHangulConsonant => fixed(
                [
                    '\u{3200}', '\u{3201}', '\u{3202}', '\u{3203}', '\u{3204}',
                    '\u{3205}', '\u{3206}', '\u{3207}', '\u{3208}', '\u{3209}',
                    '\u{320A}', '\u{320B}', '\u{320C}', '\u{320D}',
                ],
                n,
            ),
            Self::ParenthesizedHangulSyllable => fixed(
                [
                    '\u{320E}', '\u{320F}', '\u{3210}', '\u{3211}', '\u{3212}',
                    '\u{3213}', '\u{3214}', '\u{3215}', '\u{3216}', '\u{3217}',
                    '\u{3218}', '\u{3219}', '\u{321A}',
                ],
                n,
            ),
            Self::ParenthesizedIdeograph => fixed(
                [
                    '\u{3220}', '\u{3221}', '\u{3222}', '\u{3223}', '\u{3224}',
                    '\u{3225}', '\u{3226}', '\u{3227}', '\u{3228}', '\u{3229}',
                ],
                n,
            ),
            Self::ParenthesizedLowerLatin => fixed(
                [
                    '\u{249C}', '\u{249D}', '\u{249E}', '\u{249F}', '\u{24A0}',
                    '\u{24A1}', '\u{24A2}', '\u{24A3}', '\u{24A4}', '\u{24A5}',
                    '\u{24A6}', '\u{24A7}', '\u{24A8}', '\u{24A9}', '\u{24AA}',
                    '\u{24AB}', '\u{24AC}', '\u{24AD}', '\u{24AE}', '\u{24AF}',
                    '\u{24B0}', '\u{24B1}', '\u{24B2}', '\u{24B3}', '\u{24B4}',
                    '\u{24B5}',
                ],
                n,
            ),
            Self::Persian => numeric(
                [
                    '\u{6F0}', '\u{6F1}', '\u{6F2}', '\u{6F3}', '\u{6F4}', '\u{6F5}',
                    '\u{6F6}', '\u{6F7}', '\u{6F8}', '\u{6F9}',
                ],
                n,
            ),
            Self::PersianAbjad => fixed(
                [
                    '\u{627}', '\u{628}', '\u{62C}', '\u{62F}', '\u{647}', '\u{648}',
                    '\u{632}', '\u{62D}', '\u{637}', '\u{6CC}', '\u{6A9}', '\u{644}',
                    '\u{645}', '\u{646}', '\u{633}', '\u{639}', '\u{641}', '\u{635}',
                    '\u{642}', '\u{631}', '\u{634}', '\u{62A}', '\u{62B}', '\u{62E}',
                    '\u{630}', '\u{636}', '\u{638}', '\u{63A}',
                ],
                n,
            ),
            Self::PersianAlphabetic => fixed(
                [
                    '\u{627}', '\u{628}', '\u{67E}', '\u{62A}', '\u{62B}', '\u{62C}',
                    '\u{686}', '\u{62D}', '\u{62E}', '\u{62F}', '\u{630}', '\u{631}',
                    '\u{632}', '\u{698}', '\u{633}', '\u{634}', '\u{635}', '\u{636}',
                    '\u{637}', '\u{638}', '\u{639}', '\u{63A}', '\u{641}', '\u{642}',
                    '\u{6A9}', '\u{6AF}', '\u{644}', '\u{645}', '\u{646}', '\u{648}',
                    '\u{647}', '\u{6CC}',
                ],
                n,
            ),
            Self::Punjabi => alphabetic(
                [
                    '\u{0A73}', '\u{0A05}', '\u{0A72}', '\u{0A38}', '\u{0A39}',
                    '\u{0A15}', '\u{0A16}', '\u{0A17}', '\u{0A18}', '\u{0A19}',
                    '\u{0A1A}', '\u{0A1B}', '\u{0A1C}', '\u{0A1D}', '\u{0A1E}',
                    '\u{0A1F}', '\u{0A20}', '\u{0A21}', '\u{0A22}', '\u{0A23}',
                    '\u{0A24}', '\u{0A25}', '\u{0A26}', '\u{0A27}', '\u{0A28}',
                    '\u{0A2A}', '\u{0A2B}', '\u{0A2C}', '\u{0A2D}', '\u{0A2E}',
                    '\u{0A2F}', '\u{0A30}', '\u{0A32}', '\u{0A35}', '\u{0A5C}',
                ],
                n,
            ),
            Self::Saho => alphabetic(
                [
                    '\u{1200}', '\u{1208}', '\u{1210}', '\u{1218}', '\u{1228}',
                    '\u{1230}', '\u{1240}', '\u{1260}', '\u{1270}', '\u{1290}',
                    '\u{1308}', '\u{1320}', '\u{1328}', '\u{1330}', '\u{1338}',
                    '\u{1348}',
                ],
                n,
            ),
            Self::Sanskrit => alphabetic(
                [
                    '\u{915}', '\u{916}', '\u{917}', '\u{918}', '\u{919}', '\u{91A}',
                    '\u{91B}', '\u{91C}', '\u{91D}', '\u{91E}', '\u{91F}', '\u{920}',
                    '\u{921}', '\u{922}', '\u{923}', '\u{924}', '\u{925}', '\u{926}',
                    '\u{927}', '\u{928}', '\u{92A}', '\u{92B}', '\u{92C}', '\u{92D}',
                    '\u{92E}', '\u{92F}', '\u{930}', '\u{932}', '\u{935}', '\u{936}',
                    '\u{937}', '\u{938}', '\u{939}',
                ],
                n,
            ),
            Self::Santali => alphabetic(
                [
                    '\u{1C5A}', '\u{1C5B}', '\u{1C5C}', '\u{1C5D}', '\u{1C5E}',
                    '\u{1C5F}', '\u{1C60}', '\u{1C61}', '\u{1C62}', '\u{1C63}',
                    '\u{1C64}', '\u{1C65}', '\u{1C66}', '\u{1C67}', '\u{1C68}',
                    '\u{1C69}', '\u{1C6A}', '\u{1C6B}', '\u{1C6C}', '\u{1C6D}',
                    '\u{1C6E}', '\u{1C6F}', '\u{1C70}', '\u{1C71}', '\u{1C72}',
                    '\u{1C73}', '\u{1C74}', '\u{1C75}', '\u{1C76}', '\u{1C77}',
                ],
                n,
            ),
            Self::Shan => numeric(
                [
                    '\u{1090}', '\u{1091}', '\u{1092}', '\u{1093}', '\u{1094}',
                    '\u{1095}', '\u{1096}', '\u{1097}', '\u{1098}', '\u{1099}',
                ],
                n,
            ),
            Self::Sidama => alphabetic(
                [
                    '\u{1200}', '\u{1208}', '\u{1218}', '\u{1228}', '\u{1230}',
                    '\u{1238}', '\u{1240}', '\u{1260}', '\u{1270}', '\u{1278}',
                    '\u{1290}', '\u{1298}', '\u{12A0}', '\u{12A8}', '\u{12C8}',
                    '\u{12E8}', '\u{12F0}', '\u{12F8}', '\u{1300}', '\u{1308}',
                    '\u{1320}', '\u{1328}', '\u{1330}', '\u{1338}', '\u{1348}',
                ],
                n,
            ),
            Self::Silti => alphabetic(
                [
                    '\u{1200}', '\u{1208}', '\u{1218}', '\u{1228}', '\u{1230}',
                    '\u{1238}', '\u{1240}', '\u{1260}', '\u{1270}', '\u{1278}',
                    '\u{1290}', '\u{1298}', '\u{1300}', '\u{1308}', '\u{1320}',
                    '\u{1328}', '\u{1330}', '\u{1348}',
                ],
                n,
            ),
            Self::SimpChineseFormal => {
                usize_to_chinese(ChineseVariant::Simple, ChineseCase::Upper, n).into()
            }
            Self::SimpChineseInformal => {
                usize_to_chinese(ChineseVariant::Simple, ChineseCase::Lower, n).into()
            }
            Self::SimpleLowerRoman => additive(
                [
                    ("\u{6D}", 1000),
                    ("\u{64}", 500),
                    ("\u{63}", 100),
                    ("\u{6C}", 50),
                    ("\u{78}", 10),
                    ("\u{76}", 5),
                    ("\u{69}", 1),
                ],
                n,
            ),
            Self::SimpleUpperRoman => additive(
                [
                    ("\u{4D}", 1000),
                    ("\u{44}", 500),
                    ("\u{43}", 100),
                    ("\u{4C}", 50),
                    ("\u{58}", 10),
                    ("\u{56}", 5),
                    ("\u{49}", 1),
                ],
                n,
            ),
            Self::Sundanese => numeric(
                [
                    '\u{1BB0}', '\u{1BB1}', '\u{1BB2}', '\u{1BB3}', '\u{1BB4}',
                    '\u{1BB5}', '\u{1BB6}', '\u{1BB7}', '\u{1BB8}', '\u{1BB9}',
                ],
                n,
            ),
            Self::SuperDecimal => numeric(
                [
                    '\u{2070}', '\u{B9}', '\u{B2}', '\u{B3}', '\u{2074}', '\u{2075}',
                    '\u{2076}', '\u{2077}', '\u{2078}', '\u{2079}',
                ],
                n,
            ),
            Self::Symbol => symbolic(['*', '†', '‡', '§', '¶', '‖'], n),
            Self::TaiLue => numeric(
                [
                    '\u{19D0}', '\u{19D1}', '\u{19D2}', '\u{19D3}', '\u{19D4}',
                    '\u{19D5}', '\u{19D6}', '\u{19D7}', '\u{19D8}', '\u{19D9}',
                ],
                n,
            ),
            Self::TallyMark => additive([("\u{1D378}", 5), ("\u{1D377}", 1)], n),
            Self::Tamil => numeric(
                [
                    '\u{BE6}', '\u{BE7}', '\u{BE8}', '\u{BE9}', '\u{BEA}', '\u{BEB}',
                    '\u{BEC}', '\u{BED}', '\u{BEE}', '\u{BEF}',
                ],
                n,
            ),
            Self::Telugu => numeric(
                [
                    '\u{C66}', '\u{C67}', '\u{C68}', '\u{C69}', '\u{C6A}', '\u{C6B}',
                    '\u{C6C}', '\u{C6D}', '\u{C6E}', '\u{C6F}',
                ],
                n,
            ),
            Self::TeluguAlpha => alphabetic(
                [
                    '\u{C15}', '\u{C16}', '\u{C17}', '\u{C18}', '\u{C19}', '\u{C1A}',
                    '\u{C58}', '\u{C1B}', '\u{C1C}', '\u{C1D}', '\u{C1E}', '\u{C1F}',
                    '\u{C20}', '\u{C21}', '\u{C22}', '\u{C23}', '\u{C24}', '\u{C25}',
                    '\u{C26}', '\u{C27}', '\u{C28}', '\u{C2A}', '\u{C2B}', '\u{C2C}',
                    '\u{C2D}', '\u{C2E}', '\u{C2F}', '\u{C30}', '\u{C31}', '\u{C32}',
                    '\u{C33}', '\u{C34}', '\u{C35}', '\u{C36}', '\u{C37}', '\u{C38}',
                    '\u{C39}',
                ],
                n,
            ),
            Self::Thai => numeric(
                [
                    '\u{E50}', '\u{E51}', '\u{E52}', '\u{E53}', '\u{E54}', '\u{E55}',
                    '\u{E56}', '\u{E57}', '\u{E58}', '\u{E59}',
                ],
                n,
            ),
            Self::ThaiAlpha => alphabetic(
                [
                    '\u{E01}', '\u{E02}', '\u{E04}', '\u{E07}', '\u{E08}', '\u{E09}',
                    '\u{E0A}', '\u{E0B}', '\u{E0C}', '\u{E0D}', '\u{E0E}', '\u{E0F}',
                    '\u{E10}', '\u{E11}', '\u{E12}', '\u{E13}', '\u{E14}', '\u{E15}',
                    '\u{E16}', '\u{E17}', '\u{E18}', '\u{E19}', '\u{E1A}', '\u{E1B}',
                    '\u{E1C}', '\u{E1D}', '\u{E1E}', '\u{E1F}', '\u{E20}', '\u{E21}',
                    '\u{E22}', '\u{E23}', '\u{E25}', '\u{E27}', '\u{E28}', '\u{E29}',
                    '\u{E2A}', '\u{E2B}', '\u{E2C}', '\u{E2D}', '\u{E2E}',
                ],
                n,
            ),
            Self::Tibetan => numeric(
                [
                    '\u{F20}', '\u{F21}', '\u{F22}', '\u{F23}', '\u{F24}', '\u{F25}',
                    '\u{F26}', '\u{F27}', '\u{F28}', '\u{F29}',
                ],
                n,
            ),
            Self::Tigre => alphabetic(
                [
                    '\u{1200}', '\u{1208}', '\u{1210}', '\u{1218}', '\u{1228}',
                    '\u{1230}', '\u{1238}', '\u{1240}', '\u{1260}', '\u{1270}',
                    '\u{1278}', '\u{1290}', '\u{12A0}', '\u{12A8}', '\u{12C8}',
                    '\u{12D0}', '\u{12D8}', '\u{12E8}', '\u{12F0}', '\u{1300}',
                    '\u{1308}', '\u{1320}', '\u{1328}', '\u{1330}', '\u{1338}',
                    '\u{1348}', '\u{1350}',
                ],
                n,
            ),
            Self::TradChineseFormal => {
                usize_to_chinese(ChineseVariant::Traditional, ChineseCase::Upper, n)
                    .into()
            }
            Self::TradChineseInformal => {
                usize_to_chinese(ChineseVariant::Traditional, ChineseCase::Lower, n)
                    .into()
            }
            Self::UpperAlpha => alphabetic(
                [
                    '\u{41}', '\u{42}', '\u{43}', '\u{44}', '\u{45}', '\u{46}', '\u{47}',
                    '\u{48}', '\u{49}', '\u{4A}', '\u{4B}', '\u{4C}', '\u{4D}', '\u{4E}',
                    '\u{4F}', '\u{50}', '\u{51}', '\u{52}', '\u{53}', '\u{54}', '\u{55}',
                    '\u{56}', '\u{57}', '\u{58}', '\u{59}', '\u{5A}',
                ],
                n,
            ),
            Self::UpperAlphaSymbolic => symbolic(
                [
                    '\u{41}', '\u{42}', '\u{43}', '\u{44}', '\u{45}', '\u{46}', '\u{47}',
                    '\u{48}', '\u{49}', '\u{4A}', '\u{4B}', '\u{4C}', '\u{4D}', '\u{4E}',
                    '\u{4F}', '\u{50}', '\u{51}', '\u{52}', '\u{53}', '\u{54}', '\u{55}',
                    '\u{56}', '\u{57}', '\u{58}', '\u{59}', '\u{5A}',
                ],
                n,
            ),
            Self::UpperArmenian => additive(
                [
                    ("\u{554}", 9000),
                    ("\u{553}", 8000),
                    ("\u{552}", 7000),
                    ("\u{551}", 6000),
                    ("\u{550}", 5000),
                    ("\u{54F}", 4000),
                    ("\u{54E}", 3000),
                    ("\u{54D}", 2000),
                    ("\u{54C}", 1000),
                    ("\u{54B}", 900),
                    ("\u{54A}", 800),
                    ("\u{549}", 700),
                    ("\u{548}", 600),
                    ("\u{547}", 500),
                    ("\u{546}", 400),
                    ("\u{545}", 300),
                    ("\u{544}", 200),
                    ("\u{543}", 100),
                    ("\u{542}", 90),
                    ("\u{541}", 80),
                    ("\u{540}", 70),
                    ("\u{53F}", 60),
                    ("\u{53E}", 50),
                    ("\u{53D}", 40),
                    ("\u{53C}", 30),
                    ("\u{53B}", 20),
                    ("\u{53A}", 10),
                    ("\u{539}", 9),
                    ("\u{538}", 8),
                    ("\u{537}", 7),
                    ("\u{536}", 6),
                    ("\u{535}", 5),
                    ("\u{534}", 4),
                    ("\u{533}", 3),
                    ("\u{532}", 2),
                    ("\u{531}", 1),
                ],
                n,
            ),
            Self::UpperBelorussian => alphabetic(
                [
                    '\u{410}', '\u{411}', '\u{412}', '\u{413}', '\u{414}', '\u{415}',
                    '\u{401}', '\u{416}', '\u{417}', '\u{406}', '\u{419}', '\u{41A}',
                    '\u{41B}', '\u{41C}', '\u{41D}', '\u{41E}', '\u{41F}', '\u{420}',
                    '\u{421}', '\u{422}', '\u{423}', '\u{40E}', '\u{424}', '\u{425}',
                    '\u{426}', '\u{427}', '\u{428}', '\u{42B}', '\u{42C}', '\u{42D}',
                    '\u{42E}', '\u{42F}',
                ],
                n,
            ),
            Self::UpperBulgarian => alphabetic(
                [
                    '\u{410}', '\u{411}', '\u{412}', '\u{413}', '\u{414}', '\u{415}',
                    '\u{416}', '\u{417}', '\u{418}', '\u{419}', '\u{41A}', '\u{41B}',
                    '\u{41C}', '\u{41D}', '\u{41E}', '\u{41F}', '\u{420}', '\u{421}',
                    '\u{422}', '\u{423}', '\u{424}', '\u{425}', '\u{426}', '\u{427}',
                    '\u{428}', '\u{429}', '\u{42A}', '\u{42C}', '\u{42E}', '\u{42F}',
                ],
                n,
            ),
            Self::UpperHexadecimal => numeric(
                [
                    '\u{30}', '\u{31}', '\u{32}', '\u{33}', '\u{34}', '\u{35}', '\u{36}',
                    '\u{37}', '\u{38}', '\u{39}', '\u{41}', '\u{42}', '\u{43}', '\u{44}',
                    '\u{45}', '\u{46}',
                ],
                n,
            ),
            Self::UpperMacedonian => alphabetic(
                [
                    '\u{410}', '\u{411}', '\u{412}', '\u{413}', '\u{414}', '\u{403}',
                    '\u{415}', '\u{416}', '\u{417}', '\u{405}', '\u{418}', '\u{408}',
                    '\u{41A}', '\u{41B}', '\u{409}', '\u{41C}', '\u{41D}', '\u{40A}',
                    '\u{41E}', '\u{41F}', '\u{420}', '\u{421}', '\u{422}', '\u{40C}',
                    '\u{423}', '\u{424}', '\u{425}', '\u{426}', '\u{427}', '\u{40F}',
                    '\u{428}',
                ],
                n,
            ),
            Self::UpperRoman => additive(
                [
                    ("\u{4D}", 1000),
                    ("\u{43}\u{4D}", 900),
                    ("\u{44}", 500),
                    ("\u{43}\u{44}", 400),
                    ("\u{43}", 100),
                    ("\u{58}\u{43}", 90),
                    ("\u{4C}", 50),
                    ("\u{58}\u{4C}", 40),
                    ("\u{58}", 10),
                    ("\u{49}\u{58}", 9),
                    ("\u{56}", 5),
                    ("\u{49}\u{56}", 4),
                    ("\u{49}", 1),
                ],
                n,
            ),
            Self::UpperRussian => alphabetic(
                [
                    '\u{410}', '\u{411}', '\u{412}', '\u{413}', '\u{414}', '\u{415}',
                    '\u{416}', '\u{417}', '\u{418}', '\u{41A}', '\u{41B}', '\u{41C}',
                    '\u{41D}', '\u{41E}', '\u{41F}', '\u{420}', '\u{421}', '\u{422}',
                    '\u{423}', '\u{424}', '\u{425}', '\u{426}', '\u{427}', '\u{428}',
                    '\u{429}', '\u{42D}', '\u{42E}', '\u{42F}',
                ],
                n,
            ),
            Self::UpperRussianFull => alphabetic(
                [
                    '\u{410}', '\u{411}', '\u{412}', '\u{413}', '\u{414}', '\u{415}',
                    '\u{401}', '\u{416}', '\u{417}', '\u{418}', '\u{419}', '\u{41A}',
                    '\u{41B}', '\u{41C}', '\u{41D}', '\u{41E}', '\u{41F}', '\u{420}',
                    '\u{421}', '\u{422}', '\u{423}', '\u{424}', '\u{425}', '\u{426}',
                    '\u{427}', '\u{428}', '\u{429}', '\u{42A}', '\u{42B}', '\u{42C}',
                    '\u{42D}', '\u{42E}', '\u{42F}',
                ],
                n,
            ),
            Self::UpperSerbian => alphabetic(
                [
                    '\u{410}', '\u{411}', '\u{412}', '\u{413}', '\u{414}', '\u{402}',
                    '\u{415}', '\u{416}', '\u{417}', '\u{418}', '\u{408}', '\u{41A}',
                    '\u{41B}', '\u{409}', '\u{41C}', '\u{41D}', '\u{40A}', '\u{41E}',
                    '\u{41F}', '\u{420}', '\u{421}', '\u{422}', '\u{40B}', '\u{423}',
                    '\u{424}', '\u{425}', '\u{426}', '\u{427}', '\u{40F}', '\u{428}',
                ],
                n,
            ),
            Self::UpperUkrainian => alphabetic(
                [
                    '\u{410}', '\u{411}', '\u{412}', '\u{413}', '\u{414}', '\u{415}',
                    '\u{404}', '\u{416}', '\u{417}', '\u{418}', '\u{406}', '\u{41A}',
                    '\u{41B}', '\u{41C}', '\u{41D}', '\u{41E}', '\u{41F}', '\u{420}',
                    '\u{421}', '\u{422}', '\u{423}', '\u{424}', '\u{425}', '\u{426}',
                    '\u{427}', '\u{428}', '\u{42E}', '\u{42F}',
                ],
                n,
            ),
            Self::UpperUkrainianFull => alphabetic(
                [
                    '\u{410}', '\u{411}', '\u{412}', '\u{413}', '\u{490}', '\u{414}',
                    '\u{415}', '\u{404}', '\u{416}', '\u{417}', '\u{418}', '\u{406}',
                    '\u{407}', '\u{419}', '\u{41A}', '\u{41B}', '\u{41C}', '\u{41D}',
                    '\u{41E}', '\u{41F}', '\u{420}', '\u{421}', '\u{422}', '\u{423}',
                    '\u{424}', '\u{425}', '\u{426}', '\u{427}', '\u{428}', '\u{429}',
                    '\u{42C}', '\u{42E}', '\u{42F}',
                ],
                n,
            ),
            Self::Urdu => numeric(
                [
                    '\u{6F0}', '\u{6F1}', '\u{6F2}', '\u{6F3}', '\u{6F4}', '\u{6F5}',
                    '\u{6F6}', '\u{6F7}', '\u{6F8}', '\u{6F9}',
                ],
                n,
            ),
            Self::UrduAbjad => fixed(
                [
                    '\u{0627}', '\u{0628}', '\u{062C}', '\u{062F}', '\u{06C1}',
                    '\u{0648}', '\u{0632}', '\u{062D}', '\u{0637}', '\u{06CC}',
                    '\u{06A9}', '\u{0644}', '\u{0645}', '\u{0646}', '\u{0633}',
                    '\u{0639}', '\u{0641}', '\u{0635}', '\u{0642}', '\u{0631}',
                    '\u{0634}', '\u{062A}', '\u{062B}', '\u{062E}', '\u{0630}',
                    '\u{0636}', '\u{0638}', '\u{063A}',
                ],
                n,
            ),
            Self::UrduAlphabetic => fixed(
                [
                    '\u{0627}', '\u{0628}', '\u{067E}', '\u{062A}', '\u{0679}',
                    '\u{062B}', '\u{062C}', '\u{0686}', '\u{062D}', '\u{062E}',
                    '\u{062F}', '\u{0688}', '\u{0630}', '\u{0631}', '\u{0691}',
                    '\u{0632}', '\u{0698}', '\u{0633}', '\u{0634}', '\u{0635}',
                    '\u{0636}', '\u{0637}', '\u{0638}', '\u{0639}', '\u{063A}',
                    '\u{0641}', '\u{0642}', '\u{06A9}', '\u{06AF}', '\u{0644}',
                    '\u{0645}', '\u{0646}', '\u{06BA}', '\u{0648}', '\u{06C1}',
                    '\u{06BE}', '\u{0621}', '\u{06CC}',
                ],
                n,
            ),
            Self::WarangCiti => numeric(
                [
                    '\u{118E0}',
                    '\u{118E1}',
                    '\u{118E2}',
                    '\u{118E3}',
                    '\u{118E4}',
                    '\u{118E5}',
                    '\u{118E6}',
                    '\u{118E7}',
                    '\u{118E8}',
                    '\u{118E9}',
                ],
                n,
            ),
            Self::Wolaita => alphabetic(
                [
                    '\u{1200}', '\u{1208}', '\u{1218}', '\u{1228}', '\u{1230}',
                    '\u{1238}', '\u{1240}', '\u{1260}', '\u{1270}', '\u{1278}',
                    '\u{1290}', '\u{1298}', '\u{1230}', '\u{1308}', '\u{1320}',
                    '\u{1328}', '\u{1330}', '\u{1338}', '\u{1340}', '\u{1348}',
                    '\u{1350}',
                ],
                n,
            ),
            Self::Yemsa => alphabetic(
                [
                    '\u{1200}', '\u{1208}', '\u{1218}', '\u{1228}', '\u{1230}',
                    '\u{1238}', '\u{1240}', '\u{1260}', '\u{1268}', '\u{1270}',
                    '\u{1278}', '\u{1290}', '\u{1298}', '\u{1300}', '\u{1308}',
                    '\u{1318}', '\u{1320}', '\u{1328}', '\u{1330}', '\u{1348}',
                    '\u{1350}',
                ],
                n,
            ),
            Self::Zhuyin => alphabetic(
                [
                    '\u{3105}', '\u{3106}', '\u{3107}', '\u{3108}', '\u{3109}',
                    '\u{310A}', '\u{310B}', '\u{310C}', '\u{310D}', '\u{310E}',
                    '\u{310F}', '\u{3110}', '\u{3111}', '\u{3112}', '\u{3113}',
                    '\u{3114}', '\u{3115}', '\u{3116}', '\u{3117}', '\u{3118}',
                    '\u{3119}', '\u{311A}', '\u{311B}', '\u{311C}', '\u{311D}',
                    '\u{311E}', '\u{311F}', '\u{3120}', '\u{3121}', '\u{3122}',
                    '\u{3123}', '\u{3124}', '\u{3125}', '\u{3126}', '\u{3127}',
                    '\u{3128}', '\u{3129}',
                ],
                n,
            ),
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
    if n == 0 {
        return '-'.into();
    }
    let mut s = EcoString::new();
    while n != 0 {
        n -= 1;
        s.push(symbols[n % N_DIGITS]);
        n /= N_DIGITS;
    }
    s.chars().rev().collect()
}

fn fixed<const N_DIGITS: usize>(symbols: [char; N_DIGITS], n: usize) -> EcoString {
    if n - 1 < N_DIGITS {
        return symbols[n - 1].into();
    }
    eco_format!("{n}")
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
    if n == 0 {
        return '-'.into();
    }
    EcoString::from(symbols[(n - 1) % N_DIGITS]).repeat(n.div_ceil(N_DIGITS))
}
