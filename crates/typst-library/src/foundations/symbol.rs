use std::cmp::Reverse;
use std::collections::BTreeSet;
use std::fmt::{self, Debug, Display, Formatter, Write};
use std::sync::Arc;

use ecow::{eco_format, EcoString};
use serde::{Serialize, Serializer};
use typst_syntax::{Span, Spanned};

use crate::diag::{bail, SourceResult, StrResult};
use crate::foundations::{cast, func, scope, ty, Array, Func};

/// A Unicode symbol.
///
/// Typst defines common symbols so that they can easily be written with
/// standard keyboards. The symbols are defined in modules, from which they can
/// be accessed using [field access notation]($scripting/#fields):
///
/// - General symbols are defined in the [`sym` module]($category/symbols/sym)
/// - Emoji are defined in the [`emoji` module]($category/symbols/emoji)
///
/// Moreover, you can define custom symbols with this type's constructor
/// function.
///
/// ```example
/// #sym.arrow.r \
/// #sym.gt.eq.not \
/// $gt.eq.not$ \
/// #emoji.face.halo
/// ```
///
/// Many symbols have different variants, which can be selected by appending the
/// modifiers with dot notation. The order of the modifiers is not relevant.
/// Visit the documentation pages of the symbol modules and click on a symbol to
/// see its available variants.
///
/// ```example
/// $arrow.l$ \
/// $arrow.r$ \
/// $arrow.t.quad$
/// ```
#[ty(scope, cast)]
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Symbol(Repr);

/// The character of a symbol, possibly with a function.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct SymChar(char, Option<fn() -> Func>);

/// The internal representation.
#[derive(Clone, Eq, PartialEq, Hash)]
enum Repr {
    Single(SymChar),
    Const(Box<[(&'static str, SymChar)]>),
    Multi(Arc<(List, EcoString)>),
}

/// A collection of symbols.
#[derive(Clone, Eq, PartialEq, Hash)]
enum List {
    Static(Box<[(&'static str, SymChar)]>),
    Runtime(Box<[(EcoString, SymChar)]>),
}

impl Symbol {
    /// Create a new symbol from a single character.
    pub const fn single(c: SymChar) -> Self {
        Self(Repr::Single(c))
    }

    /// Create a symbol with a static variant list.
    #[track_caller]
    pub const fn list(list: Box<[(&'static str, SymChar)]>) -> Self {
        debug_assert!(!list.is_empty());
        Self(Repr::Const(list))
    }

    /// Create a symbol with a runtime variant list.
    #[track_caller]
    pub fn runtime(list: Box<[(EcoString, SymChar)]>) -> Self {
        debug_assert!(!list.is_empty());
        Self(Repr::Multi(Arc::new((List::Runtime(list), EcoString::new()))))
    }

    /// Get the symbol's char.
    pub fn get(&self) -> char {
        self.sym().char()
    }

    /// Resolve the symbol's `SymChar`.
    pub fn sym(&self) -> SymChar {
        match &self.0 {
            Repr::Single(c) => *c,
            Repr::Const(_) => find(self.variants(), "").unwrap(),
            Repr::Multi(arc) => find(self.variants(), &arc.1).unwrap(),
        }
    }

    /// Try to get the function associated with the symbol, if any.
    pub fn func(&self) -> StrResult<Func> {
        self.sym()
            .func()
            .ok_or_else(|| eco_format!("symbol {self} is not callable"))
    }

    /// Apply a modifier to the symbol.
    pub fn modified(mut self, modifier: &str) -> StrResult<Self> {
        if modifier.is_empty() {
            return Ok(self);
        }

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
    pub fn variants(&self) -> impl Iterator<Item = (&str, SymChar)> {
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
}

#[scope]
impl Symbol {
    /// Create a custom symbol with modifiers.
    ///
    /// ```example
    /// #let envelope = symbol(
    ///   "🖂",
    ///   ("stamped", "🖃"),
    ///   ("stamped.pen", "🖆"),
    ///   ("lightning", "🖄"),
    ///   ("fly", "🖅"),
    /// )
    ///
    /// #envelope
    /// #envelope.stamped
    /// #envelope.stamped.pen
    /// #envelope.lightning
    /// #envelope.fly
    /// ```
    #[func(constructor)]
    pub fn construct(
        /// The callsite span.
        span: Span,
        /// The variants of the symbol.
        ///
        /// Can be a just a string consisting of a single character for the
        /// modifierless variant or an array with two strings specifying the modifiers
        /// and the symbol. Individual modifiers should be separated by dots. When
        /// displaying a symbol, Typst selects the first from the variants that have
        /// all attached modifiers and the minimum number of other modifiers.
        #[variadic]
        variants: Vec<Spanned<SymbolVariant>>,
    ) -> SourceResult<Symbol> {
        let mut list = Vec::new();
        if variants.is_empty() {
            bail!(span, "expected at least one variant");
        }
        for Spanned { v, span } in variants {
            if list.iter().any(|(prev, _)| &v.0 == prev) {
                bail!(span, "duplicate variant");
            }
            list.push((v.0, SymChar::pure(v.1)));
        }
        Ok(Symbol::runtime(list.into_boxed_slice()))
    }
}

impl Display for Symbol {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_char(self.get())
    }
}

impl SymChar {
    /// Create a symbol character without a function.
    pub const fn pure(c: char) -> Self {
        Self(c, None)
    }

    /// Create a symbol character with a function.
    pub const fn with_func(c: char, func: fn() -> Func) -> Self {
        Self(c, Some(func))
    }

    /// Get the character of the symbol.
    pub const fn char(&self) -> char {
        self.0
    }

    /// Get the function associated with the symbol.
    pub fn func(&self) -> Option<Func> {
        self.1.map(|f| f())
    }
}

impl From<char> for SymChar {
    fn from(c: char) -> Self {
        SymChar(c, None)
    }
}

impl Debug for Repr {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Single(c) => Debug::fmt(c, f),
            Self::Const(list) => list.fmt(f),
            Self::Multi(lists) => lists.fmt(f),
        }
    }
}

impl Debug for List {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Static(list) => list.fmt(f),
            Self::Runtime(list) => list.fmt(f),
        }
    }
}

impl crate::foundations::Repr for Symbol {
    fn repr(&self) -> EcoString {
        eco_format!("\"{}\"", self.get())
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

/// A value that can be cast to a symbol.
pub struct SymbolVariant(EcoString, char);

cast! {
    SymbolVariant,
    c: char => Self(EcoString::new(), c),
    array: Array => {
        let mut iter = array.into_iter();
        match (iter.next(), iter.next(), iter.next()) {
            (Some(a), Some(b), None) => Self(a.cast()?, b.cast()?),
            _ => Err("point array must contain exactly two entries")?,
        }
    },
}

/// Iterator over variants.
enum Variants<'a> {
    Single(std::option::IntoIter<SymChar>),
    Static(std::slice::Iter<'a, (&'static str, SymChar)>),
    Runtime(std::slice::Iter<'a, (EcoString, SymChar)>),
}

impl<'a> Iterator for Variants<'a> {
    type Item = (&'a str, SymChar);

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
    variants: impl Iterator<Item = (&'a str, SymChar)>,
    modifiers: &str,
) -> Option<SymChar> {
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
