use std::str::FromStr;

use chinese_number::{
    from_usize_to_chinese_ten_thousand as usize_to_chinese, ChineseCase, ChineseVariant,
};
use comemo::Tracked;
use ecow::{eco_format, EcoString, EcoVec};

use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{cast, func, Context, Func, Str, Value};
use crate::text::Case;

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
    Adlam,
    ArabicIndic,
    ArabicAbjad,
    Kashmiri,
    MaghrebiAbjad,
    Persian,
    /// Arabic numerals (1, 2, 3, etc.).
    Arabic,
    /// Lowercase Latin letters (a, b, c, etc.). Items beyond z use base-26.
    LowerLatin,
    /// Uppercase Latin letters (A, B, C, etc.). Items beyond Z use base-26.
    UpperLatin,
    /// Lowercase Roman numerals (i, ii, iii, etc.).
    LowerRoman,
    /// Uppercase Roman numerals (I, II, III, etc.).
    UpperRoman,
    /// Lowercase Greek numerals (Α, Β, Γ, etc.).
    LowerGreek,
    /// Uppercase Greek numerals (α, β, γ, etc.).
    UpperGreek,
    /// Paragraph/note-like symbols: *, †, ‡, §, ¶, and ‖. Further items use
    /// repeated symbols.
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
    /// Circled numbers (①, ②, ③, etc.), up to 50.
    CircledNumber,
    /// Double-circled numbers (⓵, ⓶, ⓷, etc.), up to 10.
    DoubleCircledNumber,
}

impl NumberingKind {
    /// Create a numbering kind from a representative character.
    pub fn from_char(c: char) -> Option<Self> {
        Some(match c {
            '1' => NumberingKind::Arabic,
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
            '①' => NumberingKind::CircledNumber,
            '⓵' => NumberingKind::DoubleCircledNumber,
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
            "arabic" => NumberingKind::Arabic,
            "latin" => NumberingKind::LowerLatin,
            "Latin" => NumberingKind::UpperLatin,
            "roman" => NumberingKind::LowerRoman,
            "Roman" => NumberingKind::UpperRoman,
            "greek" => NumberingKind::LowerGreek,
            "Greek" => NumberingKind::UpperGreek,
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
            "circled-number" => NumberingKind::CircledNumber,
            "circled-number-double" => NumberingKind::DoubleCircledNumber,
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
            Self::Arabic => "arabic",
            Self::LowerLatin => "latin",
            Self::UpperLatin => "Latin",
            Self::LowerRoman => "roman",
            Self::UpperRoman => "Roman",
            Self::LowerGreek => "greek",
            Self::UpperGreek => "Greek",
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
            Self::CircledNumber => "circled-number",
            Self::DoubleCircledNumber => "circled-number-double",
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
            Self::Arabic => {
                numeric(['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'], n)
            }
            Self::LowerRoman => roman_numeral(n, Case::Lower),
            Self::UpperRoman => roman_numeral(n, Case::Upper),
            Self::LowerGreek => greek_numeral(n, Case::Lower),
            Self::UpperGreek => greek_numeral(n, Case::Upper),
            Self::Symbol => symbolic(['*', '†', '‡', '§', '¶', '‖'], n),

            Self::Hebrew => hebrew_numeral(n),

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
            Self::CircledNumber => alphabetic(
                [
                    '①', '②', '③', '④', '⑤', '⑥', '⑦', '⑧', '⑨', '⑩', '⑪', '⑫', '⑬', '⑭',
                    '⑮', '⑯', '⑰', '⑱', '⑲', '⑳', '㉑', '㉒', '㉓', '㉔', '㉕', '㉖',
                    '㉗', '㉘', '㉙', '㉚', '㉛', '㉜', '㉝', '㉞', '㉟', '㊱', '㊲',
                    '㊳', '㊴', '㊵', '㊶', '㊷', '㊸', '㊹', '㊺', '㊻', '㊼', '㊽',
                    '㊾', '㊿',
                ],
                n,
            ),
            Self::DoubleCircledNumber => {
                alphabetic(['⓵', '⓶', '⓷', '⓸', '⓹', '⓺', '⓻', '⓼', '⓽', '⓾'], n)
            }

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
        }
    }
}

/// Stringify an integer to a Hebrew number.
fn hebrew_numeral(mut n: usize) -> EcoString {
    if n == 0 {
        return '-'.into();
    }
    let mut fmt = EcoString::new();
    'outer: for (name, value) in [
        ('ת', 400),
        ('ש', 300),
        ('ר', 200),
        ('ק', 100),
        ('צ', 90),
        ('פ', 80),
        ('ע', 70),
        ('ס', 60),
        ('נ', 50),
        ('מ', 40),
        ('ל', 30),
        ('כ', 20),
        ('י', 10),
        ('ט', 9),
        ('ח', 8),
        ('ז', 7),
        ('ו', 6),
        ('ה', 5),
        ('ד', 4),
        ('ג', 3),
        ('ב', 2),
        ('א', 1),
    ] {
        while n >= value {
            match n {
                15 => fmt.push_str("ט״ו"),
                16 => fmt.push_str("ט״ז"),
                _ => {
                    let append_geresh = n == value && fmt.is_empty();
                    if n == value && !fmt.is_empty() {
                        fmt.push('״');
                    }
                    fmt.push(name);
                    if append_geresh {
                        fmt.push('׳');
                    }

                    n -= value;
                    continue;
                }
            }
            break 'outer;
        }
    }
    fmt
}

/// Stringify an integer to a Roman numeral.
fn roman_numeral(mut n: usize, case: Case) -> EcoString {
    if n == 0 {
        return match case {
            Case::Lower => 'n'.into(),
            Case::Upper => 'N'.into(),
        };
    }

    // Adapted from Yann Villessuzanne's roman.rs under the
    // Unlicense, at https://github.com/linfir/roman.rs/
    let mut fmt = EcoString::new();
    for &(name, value) in &[
        ("M̅", 1000000),
        ("D̅", 500000),
        ("C̅", 100000),
        ("L̅", 50000),
        ("X̅", 10000),
        ("V̅", 5000),
        ("I̅V̅", 4000),
        ("M", 1000),
        ("CM", 900),
        ("D", 500),
        ("CD", 400),
        ("C", 100),
        ("XC", 90),
        ("L", 50),
        ("XL", 40),
        ("X", 10),
        ("IX", 9),
        ("V", 5),
        ("IV", 4),
        ("I", 1),
    ] {
        while n >= value {
            n -= value;
            for c in name.chars() {
                match case {
                    Case::Lower => fmt.extend(c.to_lowercase()),
                    Case::Upper => fmt.push(c),
                }
            }
        }
    }

    fmt
}

/// Stringify an integer to Greek numbers.
///
/// Greek numbers use the Greek Alphabet to represent numbers; it is based on 10
/// (decimal). Here we implement the single digit M power representation from
/// [The Greek Number Converter][convert] and also described in
/// [Greek Numbers][numbers].
///
/// [converter]: https://www.russellcottrell.com/greek/utilities/GreekNumberConverter.htm
/// [numbers]: https://mathshistory.st-andrews.ac.uk/HistTopics/Greek_numbers/
fn greek_numeral(n: usize, case: Case) -> EcoString {
    let thousands = [
        ["͵α", "͵Α"],
        ["͵β", "͵Β"],
        ["͵γ", "͵Γ"],
        ["͵δ", "͵Δ"],
        ["͵ε", "͵Ε"],
        ["͵ϛ", "͵Ϛ"],
        ["͵ζ", "͵Ζ"],
        ["͵η", "͵Η"],
        ["͵θ", "͵Θ"],
    ];
    let hundreds = [
        ["ρ", "Ρ"],
        ["σ", "Σ"],
        ["τ", "Τ"],
        ["υ", "Υ"],
        ["φ", "Φ"],
        ["χ", "Χ"],
        ["ψ", "Ψ"],
        ["ω", "Ω"],
        ["ϡ", "Ϡ"],
    ];
    let tens = [
        ["ι", "Ι"],
        ["κ", "Κ"],
        ["λ", "Λ"],
        ["μ", "Μ"],
        ["ν", "Ν"],
        ["ξ", "Ξ"],
        ["ο", "Ο"],
        ["π", "Π"],
        ["ϙ", "Ϟ"],
    ];
    let ones = [
        ["α", "Α"],
        ["β", "Β"],
        ["γ", "Γ"],
        ["δ", "Δ"],
        ["ε", "Ε"],
        ["ϛ", "Ϛ"],
        ["ζ", "Ζ"],
        ["η", "Η"],
        ["θ", "Θ"],
    ];

    if n == 0 {
        // Greek Zero Sign
        return '𐆊'.into();
    }

    let mut fmt = EcoString::new();
    let case = match case {
        Case::Lower => 0,
        Case::Upper => 1,
    };

    // Extract a list of decimal digits from the number
    let mut decimal_digits: Vec<usize> = Vec::new();
    let mut n = n;
    while n > 0 {
        decimal_digits.push(n % 10);
        n /= 10;
    }

    // Pad the digits with leading zeros to ensure we can form groups of 4
    while decimal_digits.len() % 4 != 0 {
        decimal_digits.push(0);
    }
    decimal_digits.reverse();

    let mut m_power = decimal_digits.len() / 4;

    // M are used to represent 10000, M_power = 2 means 10000^2 = 10000 0000
    // The prefix of M is also made of Greek numerals but only be single digits, so it is 9 at max. This enables us
    // to represent up to (10000)^(9 + 1) - 1 = 10^40 -1  (9,999,999,999,999,999,999,999,999,999,999,999,999,999)
    let get_m_prefix = |m_power: usize| {
        if m_power == 0 {
            None
        } else {
            assert!(m_power <= 9);
            // the prefix of M is a single digit lowercase
            Some(ones[m_power - 1][0])
        }
    };

    let mut previous_has_number = false;
    for chunk in decimal_digits.chunks_exact(4) {
        // chunk must be exact 4 item
        assert_eq!(chunk.len(), 4);

        m_power = m_power.saturating_sub(1);

        // `th`ousan, `h`undred, `t`en and `o`ne
        let (th, h, t, o) = (chunk[0], chunk[1], chunk[2], chunk[3]);
        if th + h + t + o == 0 {
            continue;
        }

        if previous_has_number {
            fmt.push_str(", ");
        }

        if let Some(m_prefix) = get_m_prefix(m_power) {
            fmt.push_str(m_prefix);
            fmt.push_str("Μ");
        }
        if th != 0 {
            let thousand_digit = thousands[th - 1][case];
            fmt.push_str(thousand_digit);
        }
        if h != 0 {
            let hundred_digit = hundreds[h - 1][case];
            fmt.push_str(hundred_digit);
        }
        if t != 0 {
            let ten_digit = tens[t - 1][case];
            fmt.push_str(ten_digit);
        }
        if o != 0 {
            let one_digit = ones[o - 1][case];
            fmt.push_str(one_digit);
        }
        // if we do not have thousan, we need to append 'ʹ' at the end.
        if th == 0 {
            fmt.push_str("ʹ");
        }
        previous_has_number = true;
    }
    fmt
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
    EcoString::from(symbols[(n - 1) % N_DIGITS]).repeat((n).div_ceil(N_DIGITS))
}
