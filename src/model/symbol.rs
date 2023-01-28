use std::cmp::Reverse;
use std::collections::BTreeSet;
use std::fmt::{self, Debug, Display, Formatter, Write};

use crate::diag::StrResult;
use crate::util::EcoString;

/// Define a list of symbols.
#[macro_export]
#[doc(hidden)]
macro_rules! __symbols {
    ($func:ident, $($name:ident: $value:tt),* $(,)?) => {
        pub(super) fn $func(scope: &mut $crate::model::Scope) {
            $(scope.define(stringify!($name), $crate::model::symbols!(@one $value));)*
        }
    };
    (@one $c:literal) => { $crate::model::Symbol::new($c) };
    (@one [$($first:literal $(: $second:literal)?),* $(,)?]) => {
        $crate::model::Symbol::list(&[
            $($crate::model::symbols!(@pair $first $(: $second)?)),*
        ])
    };
    (@pair $first:literal) => { ("", $first) };
    (@pair $first:literal: $second:literal) => { ($first, $second) };
}

#[doc(inline)]
pub use crate::__symbols as symbols;

/// A symbol.
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct Symbol {
    repr: Repr,
    modifiers: EcoString,
}

/// A collection of symbols.
#[derive(Clone, Eq, PartialEq, Hash)]
enum Repr {
    Single(char),
    List(&'static [(&'static str, char)]),
}

impl Symbol {
    /// Create a new symbol from a single character.
    pub fn new(c: char) -> Self {
        Self { repr: Repr::Single(c), modifiers: EcoString::new() }
    }

    /// Create a symbol with variants.
    #[track_caller]
    pub fn list(list: &'static [(&'static str, char)]) -> Self {
        debug_assert!(!list.is_empty());
        Self {
            repr: Repr::List(list),
            modifiers: EcoString::new(),
        }
    }

    /// Get the symbol's text.
    pub fn get(&self) -> char {
        match self.repr {
            Repr::Single(c) => c,
            Repr::List(list) => find(list, &self.modifiers).unwrap(),
        }
    }

    /// Apply a modifier to the symbol.
    pub fn modified(mut self, modifier: &str) -> StrResult<Self> {
        if !self.modifiers.is_empty() {
            self.modifiers.push('.');
        }
        self.modifiers.push_str(modifier);
        if match self.repr {
            Repr::Single(_) => true,
            Repr::List(list) => find(list, &self.modifiers).is_none(),
        } {
            Err("unknown modifier")?
        }
        Ok(self)
    }

    /// The characters that are covered by this symbol.
    pub fn variants(&self) -> impl Iterator<Item = (&str, char)> {
        let (first, slice) = match self.repr {
            Repr::Single(c) => (Some(("", c)), [].as_slice()),
            Repr::List(list) => (None, list),
        };
        first.into_iter().chain(slice.iter().copied())
    }

    /// Possible modifiers.
    pub fn modifiers(&self) -> impl Iterator<Item = &str> + '_ {
        let mut set = BTreeSet::new();
        if let Repr::List(list) = self.repr {
            for modifier in list.iter().flat_map(|(name, _)| name.split('.')) {
                if !modifier.is_empty() && !contained(&self.modifiers, modifier) {
                    set.insert(modifier);
                }
            }
        }
        set.into_iter()
    }
}

impl Debug for Symbol {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_char(self.get())
    }
}

impl Display for Symbol {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_char(self.get())
    }
}

/// Find the best symbol from the list.
fn find(list: &[(&str, char)], modifiers: &str) -> Option<char> {
    let mut best = None;
    let mut best_score = None;

    // Find the best table entry with this name.
    'outer: for candidate in list {
        for modifier in parts(modifiers) {
            if !contained(candidate.0, modifier) {
                continue 'outer;
            }
        }

        let mut matching = 0;
        let mut total = 0;
        for modifier in parts(candidate.0) {
            if contained(modifiers, modifier) {
                matching += 1;
            }
            total += 1;
        }

        let score = (matching, Reverse(total));
        if best_score.map_or(true, |b| score > b) {
            best = Some(candidate.1);
            best_score = Some(score);
        }
    }

    best
}

/// Split a modifier list into its parts.
fn parts(modifiers: &str) -> impl Iterator<Item = &str> {
    modifiers.split('.').filter(|s| !s.is_empty())
}

/// Whether the modifier string contains the modifier `m`.
fn contained(modifiers: &str, m: &str) -> bool {
    parts(modifiers).any(|part| part == m)
}

/// Normalize an accent to a combining one.
///
/// https://www.w3.org/TR/mathml-core/#combining-character-equivalences
pub fn combining_accent(c: char) -> Option<char> {
    Some(match c {
        '\u{0300}' | '`' => '\u{0300}',
        '\u{0301}' | '´' => '\u{0301}',
        '\u{0302}' | '^' | 'ˆ' => '\u{0302}',
        '\u{0303}' | '~' | '∼' | '˜' => '\u{0303}',
        '\u{0304}' | '¯' => '\u{0304}',
        '\u{0305}' | '-' | '‾' | '−' => '\u{0305}',
        '\u{0306}' | '˘' => '\u{0306}',
        '\u{0307}' | '.' | '˙' | '⋅' => '\u{0307}',
        '\u{0308}' | '¨' => '\u{0308}',
        '\u{030a}' | '∘' | '○' => '\u{030a}',
        '\u{030b}' | '˝' => '\u{030b}',
        '\u{030c}' | 'ˇ' => '\u{030c}',
        '\u{0327}' | '¸' => '\u{0327}',
        '\u{0328}' | '˛' => '\u{0328}',
        '\u{0332}' | '_' => '\u{0332}',
        '\u{20d6}' | '←' => '\u{20d6}',
        '\u{20d7}' | '→' | '⟶' => '\u{20d7}',
        '⏞' | '⏟' | '⎴' | '⎵' | '⏜' | '⏝' | '⏠' | '⏡' => c,
        _ => return None,
    })
}
