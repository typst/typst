use std::str::FromStr;

use unscanny::Scanner;

use crate::prelude::*;

/// Create a blind text string.
pub fn lorem(_: &Vm, args: &mut Args) -> SourceResult<Value> {
    let words: usize = args.expect("number of words")?;
    Ok(Value::Str(lipsum::lipsum(words).into()))
}

/// Apply a numbering pattern to a number.
pub fn numbering(_: &Vm, args: &mut Args) -> SourceResult<Value> {
    let number = args.expect::<usize>("number")?;
    let pattern = args.expect::<NumberingPattern>("pattern")?;
    Ok(Value::Str(pattern.apply(number).into()))
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
    prefix: EcoString,
    numbering: NumberingKind,
    upper: bool,
    suffix: EcoString,
}

impl NumberingPattern {
    /// Apply the pattern to the given number.
    pub fn apply(&self, n: usize) -> EcoString {
        let fmt = self.numbering.apply(n);
        let mid = if self.upper { fmt.to_uppercase() } else { fmt.to_lowercase() };
        format_eco!("{}{}{}", self.prefix, mid, self.suffix)
    }
}

impl FromStr for NumberingPattern {
    type Err = &'static str;

    fn from_str(pattern: &str) -> Result<Self, Self::Err> {
        let mut s = Scanner::new(pattern);
        let mut prefix;
        let numbering = loop {
            prefix = s.before();
            match s.eat().map(|c| c.to_ascii_lowercase()) {
                Some('1') => break NumberingKind::Arabic,
                Some('a') => break NumberingKind::Letter,
                Some('i') => break NumberingKind::Roman,
                Some('*') => break NumberingKind::Symbol,
                Some(_) => {}
                None => Err("invalid numbering pattern")?,
            }
        };
        let upper = s.scout(-1).map_or(false, char::is_uppercase);
        let suffix = s.after().into();
        Ok(Self { prefix: prefix.into(), numbering, upper, suffix })
    }
}

castable! {
    NumberingPattern,
    Expected: "numbering pattern",
    Value::Str(s) => s.parse()?,
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
    pub fn apply(self, mut n: usize) -> EcoString {
        match self {
            Self::Arabic => {
                format_eco!("{n}")
            }
            Self::Letter => {
                if n == 0 {
                    return '-'.into();
                }

                n -= 1;

                let mut letters = vec![];
                loop {
                    letters.push(b'a' + (n % 26) as u8);
                    n /= 26;
                    if n == 0 {
                        break;
                    }
                }

                letters.reverse();
                String::from_utf8(letters).unwrap().into()
            }
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
                        fmt.push_str(name);
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
        }
    }
}
