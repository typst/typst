use std::fmt::{Display, Write};
use std::str::FromStr;

use codex::numeral_systems::{NamedNumeralSystem, RepresentationError};
use comemo::Tracked;
use ecow::{EcoString, EcoVec};
use typst_syntax::Span;

use crate::diag::{At, SourceResult, StrResult, bail, warning};
use crate::engine::Engine;
use crate::foundations::{Context, Func, Str, Value, cast, func};

/// Applies a numbering to a sequence of numbers.
///
/// A numbering defines how a sequence of numbers should be displayed as
/// content. It is defined either through a pattern string or an arbitrary
/// function.
///
/// A numbering pattern consists of counting symbols, for which the actual
/// number is substituted, their prefixes, and one suffix. The prefixes and the
/// suffix are displayed as-is.
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
    engine: &mut Engine,
    context: Tracked<Context>,
    span: Span,
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
    /// displayed as-is at the end of any rendered number.
    ///
    /// **Prefixes** are all characters that are neither counting symbols nor
    /// suffixes. They are displayed as-is at in front of their rendered
    /// equivalent of their counting symbol.
    ///
    /// This parameter can also be an arbitrary function that gets each number
    /// as an individual argument. When given a function, the `numbering`
    /// function just forwards the arguments to that function. While this is not
    /// particularly useful in itself, it means that you can just give arbitrary
    /// numberings to the `numbering` function without caring whether they are
    /// defined as a pattern or function.
    numbering: Numbering,
    /// The numbers to apply the numbering to. Must be non-negative.
    ///
    /// In general, numbers are counted from one. A number of zero indicates
    /// that the first element has not yet appeared.
    ///
    /// If `numbering` is a pattern and more numbers than counting symbols are
    /// given, the last counting symbol with its prefix is repeated.
    #[variadic]
    numbers: Vec<u64>,
) -> SourceResult<Value> {
    numbering.apply(engine, context, span, &numbers)
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
        span: Span,
        numbers: &[u64],
    ) -> SourceResult<Value> {
        Ok(match self {
            Self::Pattern(pattern) => {
                Value::Str(pattern.apply(Some((engine, span)), numbers).at(span)?.into())
            }
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
    pub pieces: EcoVec<(EcoString, NamedNumeralSystem)>,
    pub suffix: EcoString,
    trimmed: bool,
}

impl NumberingPattern {
    /// Apply the pattern to the given number.
    ///
    /// If `warning_context` is not [`None`], when an error would normally be
    /// returned, a warning is emitted instead and the returned value uses
    /// Arabic numerals in place of the numeral system that caused the error.
    pub fn apply(
        &self,
        warning_context: Option<(&mut Engine, Span)>,
        numbers: &[u64],
    ) -> StrResult<EcoString> {
        if let Some((engine, span)) = warning_context {
            self.apply_with(numbers, |system, n| {
                Ok(apply_system_with_fallback(engine, span, system, n))
            })
        } else {
            self.apply_with(numbers, apply_system)
        }
    }

    /// Auxiliary method for [`NumberingPattern::apply`].
    ///
    /// Can be removed when the deprecation warnings are turned into hard
    /// errors.
    fn apply_with<D: Display>(
        &self,
        numbers: &[u64],
        mut apply_system: impl FnMut(NamedNumeralSystem, u64) -> StrResult<D>,
    ) -> StrResult<EcoString> {
        let mut fmt = EcoString::new();
        let mut numbers = numbers.iter();

        for (i, ((prefix, system), &n)) in
            self.pieces.iter().zip(&mut numbers).enumerate()
        {
            if i > 0 || !self.trimmed {
                fmt.push_str(prefix);
            }
            write!(fmt, "{}", apply_system(*system, n)?).unwrap();
        }

        for ((prefix, system), &n) in self.pieces.last().into_iter().cycle().zip(numbers)
        {
            if prefix.is_empty() {
                fmt.push_str(&self.suffix);
            } else {
                fmt.push_str(prefix);
            }
            write!(fmt, "{}", apply_system(*system, n)?).unwrap();
        }

        if !self.trimmed {
            fmt.push_str(&self.suffix);
        }

        Ok(fmt)
    }

    /// Apply only the k-th segment of the pattern to a number.
    pub fn apply_kth(
        &self,
        engine: &mut Engine,
        span: Span,
        k: usize,
        number: u64,
    ) -> EcoString {
        let mut fmt = EcoString::new();
        if let Some((prefix, _)) = self.pieces.first() {
            fmt.push_str(prefix);
        }
        if let Some((_, system)) = self
            .pieces
            .iter()
            .chain(self.pieces.last().into_iter().cycle())
            .nth(k)
        {
            let represented_number =
                apply_system_with_fallback(engine, span, *system, number);
            write!(fmt, "{represented_number}").unwrap()
        }
        fmt.push_str(&self.suffix);
        fmt
    }

    /// How many counting symbols this pattern has.
    pub fn pieces(&self) -> usize {
        self.pieces.len()
    }
}

fn apply_system(system: NamedNumeralSystem, number: u64) -> StrResult<impl Display> {
    match system.system().represent(number) {
        Ok(represented) => Ok(represented),
        Err(RepresentationError::Zero) => {
            bail!("the numeral system `{}` cannot represent zero", system.name())
        }
        Err(RepresentationError::TooLarge) => {
            bail!(
                "the number {} is too large to be represented with the `{}` numeral system",
                number,
                system.name(),
            )
        }
    }
}

/// Applies a numeral system to a number. In case of an error, fall back to
/// Arabic numerals.
///
/// This is a temporary function that should be replaced by [`apply_system`]
/// when the deprecation warning is turned into a hard error.
fn apply_system_with_fallback(
    engine: &mut Engine,
    span: Span,
    system: NamedNumeralSystem,
    number: u64,
) -> impl Display + use<> {
    apply_system(system, number).unwrap_or_else(|err| {
        engine.sink.warn(warning!(
            span,
            "{err}";
            hint: "this will become a hard error in the future";
        ));
        apply_system(NamedNumeralSystem::Arabic, number)
            .unwrap_or_else(|_| panic!("`arabic` should be able to represent {number}"))
    })
}

impl FromStr for NumberingPattern {
    type Err = &'static str;

    fn from_str(pattern: &str) -> Result<Self, Self::Err> {
        let mut pieces = EcoVec::new();
        let mut handled = 0;

        for (i, c) in pattern.char_indices() {
            let Some(kind) =
                NamedNumeralSystem::from_shorthand(c.encode_utf8(&mut [0; 4]))
            else {
                continue;
            };

            let prefix = pattern[handled..i].into();
            pieces.push((prefix, kind));
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
        for (prefix, system) in &self.pieces {
            pat.push_str(prefix);
            pat.push_str(
                system
                    .shorthand()
                    .expect("it is not possible to construct numbering systems that don't have a shorthand within Typst for now"),
            );
        }
        pat.push_str(&self.suffix);
        pat.into_value()
    },
    v: Str => v.parse()?,
}
