use std::cmp::Ordering;

use crate::prelude::*;

/// Horizontal spacing.
#[derive(Debug, Copy, Clone, Hash)]
pub struct HNode {
    /// The amount of horizontal spacing.
    pub amount: Spacing,
    /// Whether the node is weak, see also [`Behaviour`].
    pub weak: bool,
}

#[node(Behave)]
impl HNode {
    fn construct(_: &mut Vm, args: &mut Args) -> SourceResult<Content> {
        let amount = args.expect("spacing")?;
        let weak = args.named("weak")?.unwrap_or(false);
        Ok(Self { amount, weak }.pack())
    }
}

impl HNode {
    /// Normal strong spacing.
    pub fn strong(amount: Spacing) -> Self {
        Self { amount, weak: false }
    }

    /// User-created weak spacing.
    pub fn weak(amount: Spacing) -> Self {
        Self { amount, weak: true }
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
        let Some(prev) = prev.downcast::<Self>() else { return false };
        self.amount > prev.amount
    }
}

/// Vertical spacing.
#[derive(Debug, Copy, Clone, Hash, PartialEq, PartialOrd)]
pub struct VNode {
    /// The amount of vertical spacing.
    pub amount: Spacing,
    /// The node's weakness level, see also [`Behaviour`].
    pub weakness: u8,
}

#[node(Behave)]
impl VNode {
    fn construct(_: &mut Vm, args: &mut Args) -> SourceResult<Content> {
        let amount = args.expect("spacing")?;
        let node = if args.named("weak")?.unwrap_or(false) {
            Self::weak(amount)
        } else {
            Self::strong(amount)
        };
        Ok(node.pack())
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
        let Some(prev) = prev.downcast::<Self>() else { return false };
        self.amount > prev.amount
    }
}

/// Kinds of spacing.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Spacing {
    /// Spacing specified in absolute terms and relative to the parent's size.
    Relative(Rel<Length>),
    /// Spacing specified as a fraction of the remaining free space in the
    /// parent.
    Fractional(Fr),
}

impl Spacing {
    /// Whether this is fractional spacing.
    pub fn is_fractional(self) -> bool {
        matches!(self, Self::Fractional(_))
    }
}

impl From<Abs> for Spacing {
    fn from(abs: Abs) -> Self {
        Self::Relative(abs.into())
    }
}

impl From<Em> for Spacing {
    fn from(em: Em) -> Self {
        Self::Relative(Rel::new(Ratio::zero(), em.into()))
    }
}

impl PartialOrd for Spacing {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Self::Relative(a), Self::Relative(b)) => a.partial_cmp(b),
            (Self::Fractional(a), Self::Fractional(b)) => a.partial_cmp(b),
            _ => None,
        }
    }
}

castable! {
    Spacing,
    Expected: "relative length or fraction",
    Value::Length(v) => Self::Relative(v.into()),
    Value::Ratio(v) => Self::Relative(v.into()),
    Value::Relative(v) => Self::Relative(v),
    Value::Fraction(v) => Self::Fractional(v),
}
