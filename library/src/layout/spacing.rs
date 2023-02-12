use std::cmp::Ordering;

use crate::prelude::*;

/// # Spacing (H)
/// Insert horizontal spacing into a paragraph.
///
/// The spacing can be a length or a fractional. In the latter case, the
/// remaining space on the line is distributed among all fractional spacings
/// according to their relative fractions.
///
/// ## Example
/// ```example
/// #circle(fill: red)
/// #h(1fr)
/// #circle(fill: yellow)
/// #h(2fr)
/// #circle(fill: green)
/// ```
///
/// ## Mathematical Spacing
/// In [mathematical formulas]($category/math), you can additionally use these
/// constants to add spacing between elements: `thin`, `med`, `thick, `quad`.
///
/// ## Parameters
/// - amount: `Spacing` (positional, required)
///   How much spacing to insert.
///
/// - weak: `bool` (named)
///   If true, the spacing collapses at the start or end of a paragraph.
///   Moreover, from multiple adjacent weak spacings all but the largest one
///   collapse.
///
///   ```example
///   #h(1cm, weak: true)
///   We identified a group of
///   _weak_ specimens that fail to
///   manifest in most cases. However,
///   when #h(8pt, weak: true)
///   supported
///   #h(8pt, weak: true) on both
///   sides, they do show up.
///   ```
///
/// ## Category
/// layout
#[func]
#[capable(Behave)]
#[derive(Debug, Copy, Clone, Hash)]
pub struct HNode {
    /// The amount of horizontal spacing.
    pub amount: Spacing,
    /// Whether the node is weak, see also [`Behaviour`].
    pub weak: bool,
}

#[node]
impl HNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let amount = args.expect("amount")?;
        let weak = args.named("weak")?.unwrap_or(false);
        Ok(Self { amount, weak }.pack())
    }

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "amount" => Some(self.amount.encode()),
            "weak" => Some(Value::Bool(self.weak)),
            _ => None,
        }
    }
}

impl HNode {
    /// Normal strong spacing.
    pub fn strong(amount: impl Into<Spacing>) -> Self {
        Self { amount: amount.into(), weak: false }
    }

    /// User-created weak spacing.
    pub fn weak(amount: impl Into<Spacing>) -> Self {
        Self { amount: amount.into(), weak: true }
    }
}

impl Behave for HNode {
    fn behaviour(&self) -> Behaviour {
        if self.amount.is_fractional() {
            Behaviour::Destructive
        } else if self.weak {
            Behaviour::Weak(1)
        } else {
            Behaviour::Ignorant
        }
    }

    fn larger(&self, prev: &Content) -> bool {
        let Some(prev) = prev.to::<Self>() else { return false };
        self.amount > prev.amount
    }
}

/// # Spacing (V)
/// Insert vertical spacing.
///
/// The spacing can be a length or a fractional. In the latter case, the
/// remaining space on the page is distributed among all fractional spacings
/// according to their relative fractions.
///
/// ## Example
/// ```example
/// In this report, we will explore
/// the various ethical
/// considerations that must be
/// taken into account when
/// conducting psychological
/// research:
/// #v(5mm)
///
/// - Informed consent
/// - Participant confidentiality
/// - The use of
///   vulnerable populations.
/// ```
///
/// ## Parameters
/// - amount: `Spacing` (positional, required)
///   How much spacing to insert.
///
/// - weak: `bool` (named)
///   If true, the spacing collapses at the start or end of a flow. Moreover,
///   from multiple adjacent weak spacings all but the largest one collapse.
///   Weak spacings will always collapse adjacent paragraph spacing, even if the
///   paragraph spacing is larger.
///
///   ```example
///   The following theorem is
///   foundational to the field:
///   #v(4pt, weak: true)
///   $ x^2 + y^2 = r^2 $
///   #v(4pt, weak: true)
///   The proof is simple:
///   ```
/// ## Category
/// layout
#[func]
#[capable(Behave)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, PartialOrd)]
pub struct VNode {
    /// The amount of vertical spacing.
    pub amount: Spacing,
    /// The node's weakness level, see also [`Behaviour`].
    pub weakness: u8,
}

#[node]
impl VNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let amount = args.expect("spacing")?;
        let node = if args.named("weak")?.unwrap_or(false) {
            Self::weak(amount)
        } else {
            Self::strong(amount)
        };
        Ok(node.pack())
    }

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "amount" => Some(self.amount.encode()),
            "weak" => Some(Value::Bool(self.weakness != 0)),
            _ => None,
        }
    }
}

impl VNode {
    /// Normal strong spacing.
    pub fn strong(amount: Spacing) -> Self {
        Self { amount, weakness: 0 }
    }

    /// User-created weak spacing.
    pub fn weak(amount: Spacing) -> Self {
        Self { amount, weakness: 1 }
    }

    /// Weak spacing with list attach weakness.
    pub fn list_attach(amount: Spacing) -> Self {
        Self { amount, weakness: 2 }
    }

    /// Weak spacing with BlockNode::ABOVE/BELOW weakness.
    pub fn block_around(amount: Spacing) -> Self {
        Self { amount, weakness: 3 }
    }

    /// Weak spacing with BlockNode::SPACING weakness.
    pub fn block_spacing(amount: Spacing) -> Self {
        Self { amount, weakness: 4 }
    }
}

impl Behave for VNode {
    fn behaviour(&self) -> Behaviour {
        if self.amount.is_fractional() {
            Behaviour::Destructive
        } else if self.weakness > 0 {
            Behaviour::Weak(self.weakness)
        } else {
            Behaviour::Ignorant
        }
    }

    fn larger(&self, prev: &Content) -> bool {
        let Some(prev) = prev.to::<Self>() else { return false };
        self.amount > prev.amount
    }
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

    /// Encode into a value.
    pub fn encode(self) -> Value {
        match self {
            Self::Rel(rel) => {
                if rel.rel.is_zero() {
                    Value::Length(rel.abs)
                } else if rel.abs.is_zero() {
                    Value::Ratio(rel.rel)
                } else {
                    Value::Relative(rel)
                }
            }
            Self::Fr(fr) => Value::Fraction(fr),
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

impl PartialOrd for Spacing {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Self::Rel(a), Self::Rel(b)) => a.partial_cmp(b),
            (Self::Fr(a), Self::Fr(b)) => a.partial_cmp(b),
            _ => None,
        }
    }
}

castable! {
    Spacing,
    v: Rel<Length> => Self::Rel(v),
    v: Fr => Self::Fr(v),
}
