use crate::foundations::{cast, elem, Content, Packed, Resolve, StyleChain};
use crate::layout::{Abs, Em, Fr, Length, Ratio, Rel};
use crate::realize::{Behave, Behaviour};
use crate::util::Numeric;

/// Inserts horizontal spacing into a paragraph.
///
/// The spacing can be absolute, relative, or fractional. In the last case, the
/// remaining space on the line is distributed among all fractional spacings
/// according to their relative fractions.
///
/// # Example
/// ```example
/// First #h(1cm) Second \
/// First #h(30%) Second \
/// First #h(2fr) Second #h(1fr) Third
/// ```
///
/// # Mathematical Spacing { #math-spacing }
/// In [mathematical formulas]($category/math), you can additionally use these
/// constants to add spacing between elements: `thin`, `med`, `thick`, `quad`, `wide`.
#[elem(title = "Spacing (H)", Behave)]
pub struct HElem {
    /// How much spacing to insert.
    #[required]
    pub amount: Spacing,

    /// If `{true}`, the spacing collapses at the start or end of a paragraph.
    /// Moreover, from multiple adjacent weak spacings all but the largest one
    /// collapse.
    ///
    /// Weak spacing in markup also causes all adjacent markup spaces to be
    /// removed, regardless of the amount of spacing inserted. To force a space
    /// next to weak spacing, you can explicitly write `[#" "]` (for a normal
    /// space) or `[~]` (for a non-breaking space). The latter can be useful to
    /// create a construct that always attaches to the preceding word with one
    /// non-breaking space, independently of whether a markup space existed in
    /// front or not.
    ///
    /// ```example
    /// #h(1cm, weak: true)
    /// We identified a group of _weak_
    /// specimens that fail to manifest
    /// in most cases. However, when
    /// #h(8pt, weak: true) supported
    /// #h(8pt, weak: true) on both sides,
    /// they do show up.
    ///
    /// Further #h(0pt, weak: true) more,
    /// even the smallest of them swallow
    /// adjacent markup spaces.
    /// ```
    #[default(false)]
    pub weak: bool,
}

impl HElem {
    /// Zero-width horizontal weak spacing that eats surrounding spaces.
    pub fn hole() -> Self {
        Self::new(Abs::zero().into()).with_weak(true)
    }
}

impl Behave for Packed<HElem> {
    fn behaviour(&self) -> Behaviour {
        if self.amount().is_fractional() {
            Behaviour::Destructive
        } else if self.weak(StyleChain::default()) {
            Behaviour::Weak(1)
        } else {
            Behaviour::Ignorant
        }
    }

    fn larger(&self, prev: &(&Content, StyleChain), styles: StyleChain) -> bool {
        let Some(other) = prev.0.to_packed::<HElem>() else { return false };
        match (self.amount(), other.amount()) {
            (Spacing::Fr(this), Spacing::Fr(other)) => this > other,
            (Spacing::Rel(this), Spacing::Rel(other)) => {
                this.resolve(styles) > other.resolve(prev.1)
            }
            _ => false,
        }
    }
}

/// Inserts vertical spacing into a flow of blocks.
///
/// The spacing can be absolute, relative, or fractional. In the last case,
/// the remaining space on the page is distributed among all fractional spacings
/// according to their relative fractions.
///
/// # Example
/// ```example
/// #grid(
///   rows: 3cm,
///   columns: 6,
///   gutter: 1fr,
///   [A #parbreak() B],
///   [A #v(0pt) B],
///   [A #v(10pt) B],
///   [A #v(0pt, weak: true) B],
///   [A #v(40%, weak: true) B],
///   [A #v(1fr) B],
/// )
/// ```
#[elem(title = "Spacing (V)", Behave)]
pub struct VElem {
    /// How much spacing to insert.
    #[required]
    pub amount: Spacing,

    /// If `{true}`, the spacing collapses at the start or end of a flow.
    /// Moreover, from multiple adjacent weak spacings all but the largest one
    /// collapse. Weak spacings will always collapse adjacent paragraph spacing,
    /// even if the paragraph spacing is larger.
    ///
    /// ```example
    /// The following theorem is
    /// foundational to the field:
    /// #v(4pt, weak: true)
    /// $ x^2 + y^2 = r^2 $
    /// #v(4pt, weak: true)
    /// The proof is simple:
    /// ```
    #[external]
    pub weak: bool,

    /// The element's weakness level, see also [`Behaviour`].
    #[internal]
    #[parse(args.named("weak")?.map(|v: bool| v as usize))]
    pub weakness: usize,
}

impl VElem {
    /// Normal strong spacing.
    pub fn strong(amount: Spacing) -> Self {
        Self::new(amount).with_weakness(0)
    }

    /// User-created weak spacing.
    pub fn weak(amount: Spacing) -> Self {
        Self::new(amount).with_weakness(1)
    }

    /// Weak spacing with list attach weakness.
    pub fn list_attach(amount: Spacing) -> Self {
        Self::new(amount).with_weakness(2)
    }

    /// Weak spacing with BlockElem::ABOVE/BELOW weakness.
    pub fn block_around(amount: Spacing) -> Self {
        Self::new(amount).with_weakness(3)
    }

    /// Weak spacing with BlockElem::SPACING weakness.
    pub fn block_spacing(amount: Spacing) -> Self {
        Self::new(amount).with_weakness(4)
    }
}

impl Behave for Packed<VElem> {
    fn behaviour(&self) -> Behaviour {
        if self.amount().is_fractional() {
            Behaviour::Destructive
        } else if self.weakness(StyleChain::default()) > 0 {
            Behaviour::Weak(self.weakness(StyleChain::default()))
        } else {
            Behaviour::Ignorant
        }
    }

    fn larger(&self, prev: &(&Content, StyleChain), styles: StyleChain) -> bool {
        let Some(other) = prev.0.to_packed::<VElem>() else { return false };
        match (self.amount(), other.amount()) {
            (Spacing::Fr(this), Spacing::Fr(other)) => this > other,
            (Spacing::Rel(this), Spacing::Rel(other)) => {
                this.resolve(styles) > other.resolve(prev.1)
            }
            _ => false,
        }
    }
}

cast! {
    VElem,
    v: Content => v.unpack::<Self>().map_err(|_| "expected `v` element")?,
}

/// Kinds of spacing.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Spacing {
    /// Spacing specified in absolute terms and relative to the parent's size.
    Rel(Rel<Length>),
    /// Spacing specified as a fraction of the remaining free space in the
    /// parent.
    Fr(Fr),
}

impl Spacing {
    /// Whether this is fractional spacing.
    pub fn is_fractional(self) -> bool {
        matches!(self, Self::Fr(_))
    }

    /// Whether the spacing is actually no spacing.
    pub fn is_zero(&self) -> bool {
        match self {
            Self::Rel(rel) => rel.is_zero(),
            Self::Fr(fr) => fr.is_zero(),
        }
    }
}

impl From<Abs> for Spacing {
    fn from(abs: Abs) -> Self {
        Self::Rel(abs.into())
    }
}

impl From<Em> for Spacing {
    fn from(em: Em) -> Self {
        Self::Rel(Rel::new(Ratio::zero(), em.into()))
    }
}

impl From<Length> for Spacing {
    fn from(length: Length) -> Self {
        Self::Rel(length.into())
    }
}

impl From<Fr> for Spacing {
    fn from(fr: Fr) -> Self {
        Self::Fr(fr)
    }
}

cast! {
    Spacing,
    self => match self {
        Self::Rel(rel) => {
            if rel.rel.is_zero() {
                rel.abs.into_value()
            } else if rel.abs.is_zero() {
                rel.rel.into_value()
            } else {
                rel.into_value()
            }
        }
        Self::Fr(fr) => fr.into_value(),
    },
    v: Rel<Length> => Self::Rel(v),
    v: Fr => Self::Fr(v),
}
