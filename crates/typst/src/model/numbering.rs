use std::str::FromStr;

use chinese_number::{ChineseCase, ChineseCountMethod, ChineseVariant, NumberToChinese};
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
#[func]
pub fn numbering(
    /// The engine.
    engine: &mut Engine,
    /// The callsite context.
    context: &Context,
    /// Defines how the numbering works.
    ///
    /// **Counting symbols** are `1`, `a`, `A`, `i`, `I`, `一`, `壹`, `あ`, `い`, `ア`, `イ`, `א`, `가`,
    /// `ㄱ`, and `*`. They are replaced by the number in the sequence, in the
    /// given case.
    ///
    /// The `*` character means that symbols should be used to count, in the
    /// order of `*`, `†`, `‡`, `§`, `¶`, and `‖`. If there are more than six
    /// items, the number is represented using multiple symbols.
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
        context: &Context,
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
/// A pattern consists of a prefix, followed by one of
/// `1`, `a`, `A`, `i`, `I`, `一`, `壹`, `あ`, `い`, `ア`, `イ`, `א`, `가`, `ㄱ`, or `*`,
/// and then a suffix.
///
/// Examples of valid patterns:
/// - `1)`
/// - `a.`
/// - `(I)`
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct NumberingPattern {
    pub pieces: EcoVec<(EcoString, NumberingKind, Case)>,
    pub suffix: EcoString,
    trimmed: bool,
}

impl NumberingPattern {
    /// Apply the pattern to the given number.
    pub fn apply(&self, numbers: &[usize]) -> EcoString {
        let mut fmt = EcoString::new();
        let mut numbers = numbers.iter();

        for (i, ((prefix, kind, case), &n)) in
            self.pieces.iter().zip(&mut numbers).enumerate()
        {
            if i > 0 || !self.trimmed {
                fmt.push_str(prefix);
            }
            fmt.push_str(&kind.apply(n, *case));
        }

        for ((prefix, kind, case), &n) in
            self.pieces.last().into_iter().cycle().zip(numbers)
        {
            if prefix.is_empty() {
                fmt.push_str(&self.suffix);
            } else {
                fmt.push_str(prefix);
            }
            fmt.push_str(&kind.apply(n, *case));
        }

        if !self.trimmed {
            fmt.push_str(&self.suffix);
        }

        fmt
    }

    /// Apply only the k-th segment of the pattern to a number.
    pub fn apply_kth(&self, k: usize, number: usize) -> EcoString {
        let mut fmt = EcoString::new();
        if let Some((prefix, _, _)) = self.pieces.first() {
            fmt.push_str(prefix);
        }
        if let Some((_, kind, case)) = self
            .pieces
            .iter()
            .chain(self.pieces.last().into_iter().cycle())
            .nth(k)
        {
            fmt.push_str(&kind.apply(number, *case));
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
        let mut pieces = EcoVec::new();
        let mut handled = 0;

        for (i, c) in pattern.char_indices() {
            let Some(kind) = NumberingKind::from_char(c.to_ascii_lowercase()) else {
                continue;
            };

            let prefix = pattern[handled..i].into();
            let case =
                if c.is_uppercase() || c == '壹' { Case::Upper } else { Case::Lower };
            pieces.push((prefix, kind, case));
            handled = c.len_utf8() + i;
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
        for (prefix, kind, case) in &self.pieces {
            pat.push_str(prefix);
            let mut c = kind.to_char();
            if *case == Case::Upper {
                c = c.to_ascii_uppercase();
            }
            pat.push(c);
        }
        pat.push_str(&self.suffix);
        pat.into_value()
    },
    v: Str => v.parse()?,
}

/// Different kinds of numberings.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum NumberingKind {
    Arabic,
    Letter,
    Roman,
    Symbol,
    Hebrew,
    SimplifiedChinese,
    // TODO: Pick the numbering pattern based on languages choice.
    // As the `1st` numbering character of Chinese (Simplified) and
    // Chinese (Traditional) is same, we are unable to determine
    // if the context is Simplified or Traditional by only this
    // character.
    #[allow(unused)]
    TraditionalChinese,
    HiraganaAiueo,
    HiraganaIroha,
    KatakanaAiueo,
    KatakanaIroha,
    KoreanJamo,
    KoreanSyllable,
}

impl NumberingKind {
    /// Create a numbering kind from a lowercase character.
    pub fn from_char(c: char) -> Option<Self> {
        Some(match c {
            '1' => NumberingKind::Arabic,
            'a' => NumberingKind::Letter,
            'i' => NumberingKind::Roman,
            '*' => NumberingKind::Symbol,
            'א' => NumberingKind::Hebrew,
            '一' | '壹' => NumberingKind::SimplifiedChinese,
            'あ' => NumberingKind::HiraganaAiueo,
            'い' => NumberingKind::HiraganaIroha,
            'ア' => NumberingKind::KatakanaAiueo,
            'イ' => NumberingKind::KatakanaIroha,
            'ㄱ' => NumberingKind::KoreanJamo,
            '가' => NumberingKind::KoreanSyllable,
            _ => return None,
        })
    }

    /// The lowercase character for this numbering kind.
    pub fn to_char(self) -> char {
        match self {
            Self::Arabic => '1',
            Self::Letter => 'a',
            Self::Roman => 'i',
            Self::Symbol => '*',
            Self::Hebrew => 'א',
            Self::SimplifiedChinese => '一',
            Self::TraditionalChinese => '一',
            Self::HiraganaAiueo => 'あ',
            Self::HiraganaIroha => 'い',
            Self::KatakanaAiueo => 'ア',
            Self::KatakanaIroha => 'イ',
            Self::KoreanJamo => 'ㄱ',
            Self::KoreanSyllable => '가',
        }
    }

    /// Apply the numbering to the given number.
    pub fn apply(self, mut n: usize, case: Case) -> EcoString {
        match self {
            Self::Arabic => {
                eco_format!("{n}")
            }
            Self::Letter => zeroless::<26>(
                |x| match case {
                    Case::Lower => char::from(b'a' + x as u8),
                    Case::Upper => char::from(b'A' + x as u8),
                },
                n,
            ),
            Self::HiraganaAiueo => zeroless::<46>(
                |x| {
                    [
                        'あ', 'い', 'う', 'え', 'お', 'か', 'き', 'く', 'け', 'こ', 'さ',
                        'し', 'す', 'せ', 'そ', 'た', 'ち', 'つ', 'て', 'と', 'な', 'に',
                        'ぬ', 'ね', 'の', 'は', 'ひ', 'ふ', 'へ', 'ほ', 'ま', 'み', 'む',
                        'め', 'も', 'や', 'ゆ', 'よ', 'ら', 'り', 'る', 'れ', 'ろ', 'わ',
                        'を', 'ん',
                    ][x]
                },
                n,
            ),
            Self::HiraganaIroha => zeroless::<47>(
                |x| {
                    [
                        'い', 'ろ', 'は', 'に', 'ほ', 'へ', 'と', 'ち', 'り', 'ぬ', 'る',
                        'を', 'わ', 'か', 'よ', 'た', 'れ', 'そ', 'つ', 'ね', 'な', 'ら',
                        'む', 'う', 'ゐ', 'の', 'お', 'く', 'や', 'ま', 'け', 'ふ', 'こ',
                        'え', 'て', 'あ', 'さ', 'き', 'ゆ', 'め', 'み', 'し', 'ゑ', 'ひ',
                        'も', 'せ', 'す',
                    ][x]
                },
                n,
            ),
            Self::KatakanaAiueo => zeroless::<46>(
                |x| {
                    [
                        'ア', 'イ', 'ウ', 'エ', 'オ', 'カ', 'キ', 'ク', 'ケ', 'コ', 'サ',
                        'シ', 'ス', 'セ', 'ソ', 'タ', 'チ', 'ツ', 'テ', 'ト', 'ナ', 'ニ',
                        'ヌ', 'ネ', 'ノ', 'ハ', 'ヒ', 'フ', 'ヘ', 'ホ', 'マ', 'ミ', 'ム',
                        'メ', 'モ', 'ヤ', 'ユ', 'ヨ', 'ラ', 'リ', 'ル', 'レ', 'ロ', 'ワ',
                        'ヲ', 'ン',
                    ][x]
                },
                n,
            ),
            Self::KatakanaIroha => zeroless::<47>(
                |x| {
                    [
                        'イ', 'ロ', 'ハ', 'ニ', 'ホ', 'ヘ', 'ト', 'チ', 'リ', 'ヌ', 'ル',
                        'ヲ', 'ワ', 'カ', 'ヨ', 'タ', 'レ', 'ソ', 'ツ', 'ネ', 'ナ', 'ラ',
                        'ム', 'ウ', 'ヰ', 'ノ', 'オ', 'ク', 'ヤ', 'マ', 'ケ', 'フ', 'コ',
                        'エ', 'テ', 'ア', 'サ', 'キ', 'ユ', 'メ', 'ミ', 'シ', 'ヱ', 'ヒ',
                        'モ', 'セ', 'ス',
                    ][x]
                },
                n,
            ),
            Self::Roman => {
                if n == 0 {
                    return 'N'.into();
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
            Self::Symbol => {
                if n == 0 {
                    return '-'.into();
                }

                const SYMBOLS: &[char] = &['*', '†', '‡', '§', '¶', '‖'];
                let symbol = SYMBOLS[(n - 1) % SYMBOLS.len()];
                let amount = ((n - 1) / SYMBOLS.len()) + 1;
                std::iter::repeat(symbol).take(amount).collect()
            }
            Self::Hebrew => {
                if n == 0 {
                    return '-'.into();
                }

                let mut fmt = EcoString::new();
                'outer: for &(name, value) in &[
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
            l @ (Self::SimplifiedChinese | Self::TraditionalChinese) => {
                let chinese_case = match case {
                    Case::Lower => ChineseCase::Lower,
                    Case::Upper => ChineseCase::Upper,
                };

                match (n as u64).to_chinese(
                    match l {
                        Self::SimplifiedChinese => ChineseVariant::Simple,
                        Self::TraditionalChinese => ChineseVariant::Traditional,
                        _ => unreachable!(),
                    },
                    chinese_case,
                    ChineseCountMethod::TenThousand,
                ) {
                    Ok(num_str) => EcoString::from(num_str),
                    Err(_) => '-'.into(),
                }
            }
            Self::KoreanJamo => zeroless::<14>(
                |x| {
                    [
                        'ㄱ', 'ㄴ', 'ㄷ', 'ㄹ', 'ㅁ', 'ㅂ', 'ㅅ', 'ㅇ', 'ㅈ', 'ㅊ', 'ㅋ',
                        'ㅌ', 'ㅍ', 'ㅎ',
                    ][x]
                },
                n,
            ),
            Self::KoreanSyllable => zeroless::<14>(
                |x| {
                    [
                        '가', '나', '다', '라', '마', '바', '사', '아', '자', '차', '카',
                        '타', '파', '하',
                    ][x]
                },
                n,
            ),
        }
    }
}

/// Stringify a number using a base-N counting system with no zero digit.
///
/// This is best explained by example.  Suppose our digits are 'A', 'B', and 'C'.
/// we would get the following:
///
/// ```text
///  1 =>   "A"
///  2 =>   "B"
///  3 =>   "C"
///  4 =>  "AA"
///  5 =>  "AB"
///  6 =>  "AC"
///  7 =>  "BA"
///  8 =>  "BB"
///  9 =>  "BC"
/// 10 =>  "CA"
/// 11 =>  "CB"
/// 12 =>  "CC"
/// 13 => "AAA"
///    etc.
/// ```
///
/// You might be familiar with this scheme from the way spreadsheet software
/// tends to label its columns.
fn zeroless<const N_DIGITS: usize>(
    mk_digit: impl Fn(usize) -> char,
    mut n: usize,
) -> EcoString {
    if n == 0 {
        return '-'.into();
    }
    let mut cs = vec![];
    while n > 0 {
        n -= 1;
        cs.push(mk_digit(n % N_DIGITS));
        n /= N_DIGITS;
    }
    cs.into_iter().rev().collect()
}
