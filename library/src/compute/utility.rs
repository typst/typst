use std::str::FromStr;

use crate::prelude::*;
use crate::text::Case;

/// # Blind Text
/// Create blind text.
///
/// ## Parameters
/// - words: usize (positional, required)
///   The length of the blind text in words.
///
/// ## Category
/// utility
#[func]
pub fn lorem(args: &mut Args) -> SourceResult<Value> {
    let words: usize = args.expect("number of words")?;
    Ok(Value::Str(lipsum::lipsum(words).into()))
}

/// # Numbering
/// Apply a numbering pattern to a sequence of numbers.
///
/// ## Parameters
/// - pattern: NumberingPattern (positional, required)
///   A string that defines how the numbering works.
///
/// - numbers: NonZeroUsize (positional, variadic)
///   The numbers to apply the pattern to.
///
/// ## Category
/// utility
#[func]
pub fn numbering(args: &mut Args) -> SourceResult<Value> {
    let pattern = args.expect::<NumberingPattern>("pattern")?;
    let numbers = args.all::<NonZeroUsize>()?;
    Ok(Value::Str(pattern.apply(&numbers).into()))
}

/// How to turn a number into text.
///
/// A pattern consists of a prefix, followed by one of `1`, `a`, `A`, `i`, `I`
/// or `*`, and then a suffix.
///
/// Examples of valid patterns:
/// - `1)`
/// - `a.`
/// - `(I)`
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct NumberingPattern {
    pieces: Vec<(EcoString, NumberingKind, Case)>,
    suffix: EcoString,
}

impl NumberingPattern {
    /// Apply the pattern to the given number.
    pub fn apply(&self, numbers: &[NonZeroUsize]) -> EcoString {
        let mut fmt = EcoString::new();
        let mut numbers = numbers.into_iter();

        for ((prefix, kind, case), &n) in self.pieces.iter().zip(&mut numbers) {
            fmt.push_str(prefix);
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

        fmt.push_str(&self.suffix);
        fmt
    }
}

impl FromStr for NumberingPattern {
    type Err = &'static str;

    fn from_str(pattern: &str) -> Result<Self, Self::Err> {
        let mut pieces = vec![];
        let mut handled = 0;

        for (i, c) in pattern.char_indices() {
            let kind = match c.to_ascii_lowercase() {
                '1' => NumberingKind::Arabic,
                'a' => NumberingKind::Letter,
                'i' => NumberingKind::Roman,
                '*' => NumberingKind::Symbol,
                _ => continue,
            };

            let prefix = pattern[handled..i].into();
            let case = if c.is_uppercase() { Case::Upper } else { Case::Lower };
            pieces.push((prefix, kind, case));
            handled = i + 1;
        }

        let suffix = pattern[handled..].into();
        if pieces.is_empty() {
            Err("invalid numbering pattern")?;
        }

        Ok(Self { pieces, suffix })
    }
}

castable! {
    NumberingPattern,
    string: EcoString => string.parse()?,
}

/// Different kinds of numberings.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
enum NumberingKind {
    Arabic,
    Letter,
    Roman,
    Symbol,
}

impl NumberingKind {
    /// Apply the numbering to the given number.
    pub fn apply(self, n: NonZeroUsize, case: Case) -> EcoString {
        let mut n = n.get();
        match self {
            Self::Arabic => {
                format_eco!("{n}")
            }
            Self::Letter => {
                n -= 1;

                let mut letters = vec![];
                loop {
                    let c = b'a' + (n % 26) as u8;
                    letters.push(match case {
                        Case::Lower => c,
                        Case::Upper => c.to_ascii_uppercase(),
                    });
                    n /= 26;
                    if n == 0 {
                        break;
                    }
                }

                letters.reverse();
                String::from_utf8(letters).unwrap().into()
            }
            Self::Roman => {
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
                const SYMBOLS: &[char] = &['*', '†', '‡', '§', '¶', '‖'];
                let symbol = SYMBOLS[(n - 1) % SYMBOLS.len()];
                let amount = ((n - 1) / SYMBOLS.len()) + 1;
                std::iter::repeat(symbol).take(amount).collect()
            }
        }
    }
}
