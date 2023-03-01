use std::cmp::Reverse;
use std::collections::BTreeSet;
use std::fmt::{self, Debug, Display, Formatter, Write};

use ecow::{EcoString, EcoVec};

use crate::diag::StrResult;

#[doc(inline)]
pub use typst_macros::symbols;

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
    Static(&'static [(&'static str, char)]),
    Runtime(EcoVec<(EcoString, char)>),
}

impl Symbol {
    /// Create a new symbol from a single character.
    pub const fn new(c: char) -> Self {
        Self { repr: Repr::Single(c), modifiers: EcoString::new() }
    }

    /// Create a symbol with a static variant list.
    #[track_caller]
    pub const fn list(list: &'static [(&'static str, char)]) -> Self {
        debug_assert!(!list.is_empty());
        Self {
            repr: Repr::Static(list),
            modifiers: EcoString::new(),
        }
    }

    /// Create a symbol with a runtime variant list.
    #[track_caller]
    pub fn runtime(list: EcoVec<(EcoString, char)>) -> Self {
        debug_assert!(!list.is_empty());
        Self {
            repr: Repr::Runtime(list),
            modifiers: EcoString::new(),
        }
    }

    /// Get the symbol's text.
    pub fn get(&self) -> char {
        match self.repr {
            Repr::Single(c) => c,
            _ => find(self.variants(), &self.modifiers).unwrap(),
        }
    }

    /// Apply a modifier to the symbol.
    pub fn modified(mut self, modifier: &str) -> StrResult<Self> {
        if !self.modifiers.is_empty() {
            self.modifiers.push('.');
        }
        self.modifiers.push_str(modifier);
        if find(self.variants(), &self.modifiers).is_none() {
            Err("unknown modifier")?
        }
        Ok(self)
    }

    /// The characters that are covered by this symbol.
    pub fn variants(&self) -> impl Iterator<Item = (&str, char)> {
        match &self.repr {
            Repr::Single(c) => Variants::Single(Some(*c).into_iter()),
            Repr::Static(list) => Variants::Static(list.iter()),
            Repr::Runtime(list) => Variants::Runtime(list.iter()),
        }
    }

    /// Possible modifiers.
    pub fn modifiers(&self) -> impl Iterator<Item = &str> + '_ {
        let mut set = BTreeSet::new();
        for modifier in self.variants().flat_map(|(name, _)| name.split('.')) {
            if !modifier.is_empty() && !contained(&self.modifiers, modifier) {
                set.insert(modifier);
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

/// Iterator over variants.
enum Variants<'a> {
    Single(std::option::IntoIter<char>),
    Static(std::slice::Iter<'static, (&'static str, char)>),
    Runtime(std::slice::Iter<'a, (EcoString, char)>),
}

impl<'a> Iterator for Variants<'a> {
    type Item = (&'a str, char);

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Single(iter) => Some(("", iter.next()?)),
            Self::Static(list) => list.next().copied(),
            Self::Runtime(list) => list.next().map(|(s, c)| (s.as_str(), *c)),
        }
    }
}

/// Find the best symbol from the list.
fn find<'a>(
    variants: impl Iterator<Item = (&'a str, char)>,
    modifiers: &str,
) -> Option<char> {
    let mut best = None;
    let mut best_score = None;

    // Find the best table entry with this name.
    'outer: for candidate in variants {
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
        '\u{20d6}' | '←' => '\u{20d6}',
        '\u{20d7}' | '→' | '⟶' => '\u{20d7}',
        _ => return None,
    })
}
