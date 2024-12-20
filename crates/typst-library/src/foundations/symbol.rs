use std::cmp::Reverse;
use std::collections::{BTreeSet, HashMap};
use std::fmt::{self, Debug, Display, Formatter, Write};
use std::sync::Arc;

use ecow::{eco_format, EcoString};
use serde::{Serialize, Serializer};
use typst_syntax::{is_ident, Span, Spanned};
use typst_utils::hash128;

use crate::diag::{bail, SourceResult, StrResult};
use crate::foundations::{cast, func, scope, ty, Array, Func, NativeFunc, Repr as _};

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

/// The internal representation.
#[derive(Clone, Eq, PartialEq, Hash)]
enum Repr {
    /// A native symbol that has no named variant.
    Single(char),
    /// A native symbol with multiple named variants.
    Complex(&'static [(&'static str, char)]),
    /// A symbol with multiple named variants, where some modifiers may have
    /// been applied. Also used for symbols defined at runtime by the user with
    /// no modifier applied.
    Modified(Arc<(List, EcoString)>),
}

/// A collection of symbols.
#[derive(Clone, Eq, PartialEq, Hash)]
enum List {
    Static(&'static [(&'static str, char)]),
    Runtime(Box<[(EcoString, char)]>),
}

impl Symbol {
    /// Create a new symbol from a single character.
    pub const fn single(c: char) -> Self {
        Self(Repr::Single(c))
    }

    /// Create a symbol with a static variant list.
    #[track_caller]
    pub const fn list(list: &'static [(&'static str, char)]) -> Self {
        debug_assert!(!list.is_empty());
        Self(Repr::Complex(list))
    }

    /// Create a symbol with a runtime variant list.
    #[track_caller]
    pub fn runtime(list: Box<[(EcoString, char)]>) -> Self {
        debug_assert!(!list.is_empty());
        Self(Repr::Modified(Arc::new((List::Runtime(list), EcoString::new()))))
    }

    /// Get the symbol's character.
    pub fn get(&self) -> char {
        match &self.0 {
            Repr::Single(c) => *c,
            Repr::Complex(_) => find(self.variants(), "").unwrap(),
            Repr::Modified(arc) => find(self.variants(), &arc.1).unwrap(),
        }
    }

    /// Try to get the function associated with the symbol, if any.
    pub fn func(&self) -> StrResult<Func> {
        match self.get() {
            '⌈' => Ok(crate::math::ceil::func()),
            '⌊' => Ok(crate::math::floor::func()),
            '–' => Ok(crate::math::accent::dash::func()),
            '⋅' | '\u{0307}' => Ok(crate::math::accent::dot::func()),
            '¨' => Ok(crate::math::accent::dot_double::func()),
            '\u{20db}' => Ok(crate::math::accent::dot_triple::func()),
            '\u{20dc}' => Ok(crate::math::accent::dot_quad::func()),
            '∼' => Ok(crate::math::accent::tilde::func()),
            '´' => Ok(crate::math::accent::acute::func()),
            '˝' => Ok(crate::math::accent::acute_double::func()),
            '˘' => Ok(crate::math::accent::breve::func()),
            'ˇ' => Ok(crate::math::accent::caron::func()),
            '^' => Ok(crate::math::accent::hat::func()),
            '`' => Ok(crate::math::accent::grave::func()),
            '¯' => Ok(crate::math::accent::macron::func()),
            '○' => Ok(crate::math::accent::circle::func()),
            '→' => Ok(crate::math::accent::arrow::func()),
            '←' => Ok(crate::math::accent::arrow_l::func()),
            '↔' => Ok(crate::math::accent::arrow_l_r::func()),
            '⇀' => Ok(crate::math::accent::harpoon::func()),
            '↼' => Ok(crate::math::accent::harpoon_lt::func()),
            _ => bail!("symbol {self} is not callable"),
        }
    }

    /// Apply a modifier to the symbol.
    pub fn modified(mut self, modifier: &str) -> StrResult<Self> {
        if let Repr::Complex(list) = self.0 {
            self.0 = Repr::Modified(Arc::new((List::Static(list), EcoString::new())));
        }

        if let Repr::Modified(arc) = &mut self.0 {
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
            Repr::Complex(list) => Variants::Static(list.iter()),
            Repr::Modified(arc) => arc.0.variants(),
        }
    }

    /// Possible modifiers.
    pub fn modifiers(&self) -> impl Iterator<Item = &str> + '_ {
        let mut set = BTreeSet::new();
        let modifiers = match &self.0 {
            Repr::Modified(arc) => arc.1.as_str(),
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
        if variants.is_empty() {
            bail!(span, "expected at least one variant");
        }

        // Maps from canonicalized 128-bit hashes to indices of variants we've
        // seen before.
        let mut seen = HashMap::<u128, usize>::new();

        // A list of modifiers, cleared & reused in each iteration.
        let mut modifiers = Vec::new();

        // Validate the variants.
        for (i, &Spanned { ref v, span }) in variants.iter().enumerate() {
            modifiers.clear();

            if !v.0.is_empty() {
                // Collect all modifiers.
                for modifier in v.0.split('.') {
                    if !is_ident(modifier) {
                        bail!(span, "invalid symbol modifier: {}", modifier.repr());
                    }
                    modifiers.push(modifier);
                }
            }

            // Canonicalize the modifier order.
            modifiers.sort();

            // Ensure that there are no duplicate modifiers.
            if let Some(ms) = modifiers.windows(2).find(|ms| ms[0] == ms[1]) {
                bail!(
                    span, "duplicate modifier within variant: {}", ms[0].repr();
                    hint: "modifiers are not ordered, so each one may appear only once"
                )
            }

            // Check whether we had this set of modifiers before.
            let hash = hash128(&modifiers);
            if let Some(&i) = seen.get(&hash) {
                if v.0.is_empty() {
                    bail!(span, "duplicate default variant");
                } else if v.0 == variants[i].v.0 {
                    bail!(span, "duplicate variant: {}", v.0.repr());
                } else {
                    bail!(
                        span, "duplicate variant: {}", v.0.repr();
                        hint: "variants with the same modifiers are identical, regardless of their order"
                    )
                }
            }

            seen.insert(hash, i);
        }

        let list = variants.into_iter().map(|s| (s.v.0, s.v.1)).collect();
        Ok(Symbol::runtime(list))
    }
}

impl Display for Symbol {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_char(self.get())
    }
}

impl Debug for Repr {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Single(c) => Debug::fmt(c, f),
            Self::Complex(list) => list.fmt(f),
            Self::Modified(lists) => lists.fmt(f),
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
        match &self.0 {
            Repr::Single(c) => eco_format!("symbol(\"{}\")", *c),
            Repr::Complex(variants) => {
                eco_format!("symbol{}", repr_variants(variants.iter().copied(), ""))
            }
            Repr::Modified(arc) => {
                let (list, modifiers) = arc.as_ref();
                if modifiers.is_empty() {
                    eco_format!("symbol{}", repr_variants(list.variants(), ""))
                } else {
                    eco_format!("symbol{}", repr_variants(list.variants(), modifiers))
                }
            }
        }
    }
}

fn repr_variants<'a>(
    variants: impl Iterator<Item = (&'a str, char)>,
    applied_modifiers: &str,
) -> String {
    crate::foundations::repr::pretty_array_like(
        &variants
            .filter(|(variant, _)| {
                // Only keep variants that can still be accessed, i.e., variants
                // that contain all applied modifiers.
                parts(applied_modifiers).all(|am| variant.split('.').any(|m| m == am))
            })
            .map(|(variant, c)| {
                let trimmed_variant = variant
                    .split('.')
                    .filter(|&m| parts(applied_modifiers).all(|am| m != am));
                if trimmed_variant.clone().all(|m| m.is_empty()) {
                    eco_format!("\"{c}\"")
                } else {
                    let trimmed_modifiers = trimmed_variant.collect::<Vec<_>>().join(".");
                    eco_format!("(\"{}\", \"{}\")", trimmed_modifiers, c)
                }
            })
            .collect::<Vec<_>>(),
        false,
    )
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
            _ => Err("variant array must contain exactly two entries")?,
        }
    },
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
