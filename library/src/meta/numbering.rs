use std::str::FromStr;

use crate::prelude::*;
use crate::text::Case;

/// # Numbering
/// Apply a numbering to a sequence of numbers.
///
/// A numbering defines how a sequence of numbers should be displayed as
/// content. It is defined either through a pattern string or an arbitrary
/// function.
///
/// A numbering pattern consists of counting symbols, for which the actual
/// number is substituted, their prefixes, and one suffix. The prefixes and the
/// suffix are repeated as-is.
///
/// ## Example
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
/// ## Parameters
/// - numbering: `Numbering` (positional, required)
///   Defines how the numbering works.
///
///   **Counting symbols** are `1`, `a`, `A`, `i`, `I` and `*`. They are
///   replaced by the number in the sequence, in the given case.
///
///   The `*` character means that symbols should be used to count, in the
///   order of `*`, `†`, `‡`, `§`, `¶`, and `‖`. If there are more than six
///   items, the number is represented using multiple symbols.
///
///   **Suffixes** are all characters after the last counting symbol. They are
///   repeated as-is at the end of any rendered number.
///
///   **Prefixes** are all characters that are neither counting symbols nor
///   suffixes. They are repeated as-is at in front of their rendered
///   equivalent of their counting symbol.
///
///   This parameter can also be an arbitrary function that gets each number as
///   an individual argument. When given a function, the `numbering` function
///   just forwards the arguments to that function. While this is not
///   particularly useful in itself, it means that you can just give arbitrary
///   numberings to the `numbering` function without caring whether they are
///   defined as a pattern or function.
///
/// - numbers: `NonZeroUsize` (positional, variadic)
///   The numbers to apply the numbering to. Must be positive.
///
///   If `numbering` is a pattern and more numbers than counting symbols are
///   given, the last counting symbol with its prefix is repeated.
///
/// - returns: any
///
/// ## Category
/// meta
#[func]
pub fn numbering(vm: &Vm, args: &mut Args) -> SourceResult<Value> {
    let numbering = args.expect::<Numbering>("pattern or function")?;
    let numbers = args.all::<NonZeroUsize>()?;
    numbering.apply(vm.world(), &numbers)
}

/// How to number an enumeration.
#[derive(Debug, Clone, Hash)]
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
        world: Tracked<dyn World>,
        numbers: &[NonZeroUsize],
    ) -> SourceResult<Value> {
        Ok(match self {
            Self::Pattern(pattern) => Value::Str(pattern.apply(numbers).into()),
            Self::Func(func) => {
                let args = Args::new(
                    func.span(),
                    numbers.iter().map(|n| Value::Int(n.get() as i64)),
                );
                func.call_detached(world, args)?
            }
        })
    }
}

castable! {
    Numbering,
    v: Str => Self::Pattern(v.parse()?),
    v: Func => Self::Func(v),
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

    /// Apply only the k-th segment of the pattern to a number.
    pub fn apply_kth(&self, k: usize, number: NonZeroUsize) -> EcoString {
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
                eco_format!("{n}")
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
