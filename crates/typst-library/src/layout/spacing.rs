use typst_utils::Numeric;

use crate::foundations::{Content, cast, elem};
use crate::layout::{Abs, Em, Fr, Length, Ratio, Rel};

/// Inserts horizontal spacing into a paragraph.
///
/// The spacing can be absolute, relative, or fractional. In the last case, the
/// remaining space on the line is distributed among all fractional spacings
/// according to their relative fractions.
///
/// # Example
/// ```example
/// First #h(1cm) Second \
/// First #h(30%) Second
/// ```
///
/// # Fractional spacing
/// With fractional spacing, you can align things within a line without forcing
/// a paragraph break (like [`align`] would). Each fractionally sized element
/// gets space based on the ratio of its fraction to the sum of all fractions.
///
/// ```example
/// First #h(1fr) Second \
/// First #h(1fr) Second #h(1fr) Third \
/// First #h(2fr) Second #h(1fr) Third
/// ```
///
/// # Mathematical Spacing { #math-spacing }
/// In [mathematical formulas]($category/math), you can additionally use these
/// constants to add spacing between elements: `thin` (1/6 em), `med` (2/9 em),
/// `thick` (5/18 em), `quad` (1 em), `wide` (2 em).
#[elem(title = "Spacing (H)")]
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
#[elem(title = "Spacing (V)")]
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
    pub weak: bool,

    /// Whether the spacing collapses if not immediately preceded by a
    /// paragraph.
    #[internal]
    #[parse(Some(false))]
    pub attach: bool,
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
