use std::cmp::Ordering;

use crate::prelude::*;

/// Insert horizontal spacing into a paragraph.
///
/// The spacing can be absolute, relative, or fractional. In the last case, the
/// remaining space on the line is distributed among all fractional spacings
/// according to their relative fractions.
///
/// ## Example
/// ```example
/// First #h(1cm) Second \
/// First #h(30%) Second \
/// First #h(2fr) Second #h(1fr) Third
/// ```
///
/// ## Mathematical Spacing
/// In [mathematical formulas]($category/math), you can additionally use these
/// constants to add spacing between elements: `thin`, `med`, `thick`, `quad`.
///
/// Display: Spacing (H)
/// Category: layout
#[node(Behave)]
pub struct HNode {
    /// How much spacing to insert.
    #[required]
    pub amount: Spacing,

    /// If true, the spacing collapses at the start or end of a paragraph.
    /// Moreover, from multiple adjacent weak spacings all but the largest one
    /// collapse.
    ///
    /// ```example
    /// #h(1cm, weak: true)
    /// We identified a group of
    /// _weak_ specimens that fail to
    /// manifest in most cases. However,
    /// when #h(8pt, weak: true)
    /// supported
    /// #h(8pt, weak: true) on both
    /// sides, they do show up.
    /// ```
    #[default(false)]
    pub weak: bool,
}

impl Behave for HNode {
    fn behaviour(&self) -> Behaviour {
        if self.amount().is_fractional() {
            Behaviour::Destructive
        } else if self.weak(StyleChain::default()) {
            Behaviour::Weak(1)
        } else {
            Behaviour::Ignorant
        }
    }

    fn larger(&self, prev: &Content) -> bool {
        let Some(prev) = prev.to::<Self>() else { return false };
        self.amount() > prev.amount()
    }
}

/// Insert vertical spacing into a flow of blocks.
///
/// The spacing can be absolute, relative, or fractional. In the last case,
/// the remaining space on the page is distributed among all fractional spacings
/// according to their relative fractions.
///
/// ## Example
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
///
/// Display: Spacing (V)
/// Category: layout
#[node(Behave)]
pub struct VNode {
    /// How much spacing to insert.
    #[required]
    pub amount: Spacing,

    /// If true, the spacing collapses at the start or end of a flow. Moreover,
    /// from multiple adjacent weak spacings all but the largest one collapse.
    /// Weak spacings will always collapse adjacent paragraph spacing, even if the
    /// paragraph spacing is larger.
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

    /// The node's weakness level, see also [`Behaviour`].
    #[internal]
    #[parse(args.named("weak")?.map(|v: bool| v as usize))]
    pub weakness: usize,
}

impl VNode {
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

    /// Weak spacing with BlockNode::ABOVE/BELOW weakness.
    pub fn block_around(amount: Spacing) -> Self {
        Self::new(amount).with_weakness(3)
    }

    /// Weak spacing with BlockNode::SPACING weakness.
    pub fn block_spacing(amount: Spacing) -> Self {
        Self::new(amount).with_weakness(4)
    }
}

impl Behave for VNode {
    fn behaviour(&self) -> Behaviour {
        if self.amount().is_fractional() {
            Behaviour::Destructive
        } else if self.weakness(StyleChain::default()) > 0 {
            Behaviour::Weak(self.weakness(StyleChain::default()))
        } else {
            Behaviour::Ignorant
        }
    }

    fn larger(&self, prev: &Content) -> bool {
        let Some(prev) = prev.to::<Self>() else { return false };
        self.amount() > prev.amount()
    }
}

cast_from_value! {
    VNode,
    v: Content => v.to::<Self>().cloned().ok_or("expected vnode")?,
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

impl PartialOrd for Spacing {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Self::Rel(a), Self::Rel(b)) => a.partial_cmp(b),
            (Self::Fr(a), Self::Fr(b)) => a.partial_cmp(b),
            _ => None,
        }
    }
}

cast_from_value! {
    Spacing,
    v: Rel<Length> => Self::Rel(v),
    v: Fr => Self::Fr(v),
}

cast_to_value! {
    v: Spacing => match v {
        Spacing::Rel(rel) => {
            if rel.rel.is_zero() {
                Value::Length(rel.abs)
            } else if rel.abs.is_zero() {
                Value::Ratio(rel.rel)
            } else {
                Value::Relative(rel)
            }
        }
        Spacing::Fr(fr) => Value::Fraction(fr),
    }
}
