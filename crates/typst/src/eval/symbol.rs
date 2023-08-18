use std::cmp::Reverse;
use std::collections::BTreeSet;
use std::fmt::{self, Debug, Display, Formatter, Write};
use std::sync::Arc;

use ecow::EcoString;
use serde::{Serialize, Serializer};

use crate::diag::{bail, StrResult};

/// A symbol, possibly with variants.
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct Symbol(Repr);

/// The internal representation.
#[derive(Clone, Eq, PartialEq, Hash)]
enum Repr {
    Single(char),
    Const(&'static [(&'static str, char)]),
    Multi(Arc<(List, EcoString)>),
}

/// A collection of symbols.
#[derive(Clone, Eq, PartialEq, Hash)]
enum List {
    Static(&'static [(&'static str, char)]),
    Runtime(Box<[(EcoString, char)]>),
}

impl Symbol {
    /// Create a new symbol from a single character.
    pub const fn new(c: char) -> Self {
        Self(Repr::Single(c))
    }

    /// Create a symbol with a static variant list.
    #[track_caller]
    pub const fn list(list: &'static [(&'static str, char)]) -> Self {
        debug_assert!(!list.is_empty());
        Self(Repr::Const(list))
    }

    /// Create a symbol with a runtime variant list.
    #[track_caller]
    pub fn runtime(list: Box<[(EcoString, char)]>) -> Self {
        debug_assert!(!list.is_empty());
        Self(Repr::Multi(Arc::new((List::Runtime(list), EcoString::new()))))
    }

    /// Get the symbol's text.
    pub fn get(&self) -> char {
        match &self.0 {
            Repr::Single(c) => *c,
            Repr::Const(_) => find(self.variants(), "").unwrap(),
            Repr::Multi(arc) => find(self.variants(), &arc.1).unwrap(),
        }
    }

    /// Apply a modifier to the symbol.
    pub fn modified(mut self, modifier: &str) -> StrResult<Self> {
        if let Repr::Const(list) = self.0 {
            self.0 = Repr::Multi(Arc::new((List::Static(list), EcoString::new())));
        }

        if let Repr::Multi(arc) = &mut self.0 {
            let (list, modifiers) = Arc::make_mut(arc);
            if !modifiers.is_empty() {
                modifiers.push('.');
            }
            modifiers.push_str(modifier);
            if find(list.variants(), modifiers).is_some() {
                return Ok(self);
            }
        }

        bail!("unknown symbol modifier")
    }

    /// The characters that are covered by this symbol.
    pub fn variants(&self) -> impl Iterator<Item = (&str, char)> {
        match &self.0 {
            Repr::Single(c) => Variants::Single(Some(*c).into_iter()),
            Repr::Const(list) => Variants::Static(list.iter()),
            Repr::Multi(arc) => arc.0.variants(),
        }
    }

    /// Possible modifiers.
    pub fn modifiers(&self) -> impl Iterator<Item = &str> + '_ {
        let mut set = BTreeSet::new();
        let modifiers = match &self.0 {
            Repr::Multi(arc) => arc.1.as_str(),
            _ => "",
        };
        for modifier in self.variants().flat_map(|(name, _)| name.split('.')) {
            if !modifier.is_empty() && !contained(modifiers, modifier) {
                set.insert(modifier);
            }
        }
        set.into_iter()
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
            '\u{20db}' => '\u{20db}',
            '\u{20dc}' => '\u{20dc}',
            '\u{030a}' | '∘' | '○' => '\u{030a}',
            '\u{030b}' | '˝' => '\u{030b}',
            '\u{030c}' | 'ˇ' => '\u{030c}',
            '\u{20d6}' | '←' => '\u{20d6}',
            '\u{20d7}' | '→' | '⟶' => '\u{20d7}',
            _ => return None,
        })
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

impl Serialize for Symbol {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_char(self.get())
    }
}

impl List {
    /// The characters that are covered by this list.
    fn variants(&self) -> Variants<'_> {
        match self {
            List::Static(list) => Variants::Static(list.iter()),
            List::Runtime(list) => Variants::Runtime(list.iter()),
        }
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
