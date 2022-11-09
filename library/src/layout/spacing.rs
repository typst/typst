use std::cmp::Ordering;

use crate::prelude::*;

/// Horizontal spacing.
#[derive(Debug, Copy, Clone, Hash)]
pub struct HNode {
    pub amount: Spacing,
    pub weak: bool,
}

#[node]
impl HNode {
    fn construct(_: &mut Vm, args: &mut Args) -> SourceResult<Content> {
        let amount = args.expect("spacing")?;
        let weak = args.named("weak")?.unwrap_or(false);
        Ok(Self { amount, weak }.pack())
    }
}

/// Vertical spacing.
#[derive(Debug, Copy, Clone, Hash, PartialEq, PartialOrd)]
pub struct VNode {
    pub amount: Spacing,
    pub weak: bool,
}

impl VNode {
    /// Create weak vertical spacing.
    pub fn weak(amount: Spacing) -> Self {
        Self { amount, weak: true }
    }

    /// Create strong vertical spacing.
    pub fn strong(amount: Spacing) -> Self {
        Self { amount, weak: false }
    }
}

#[node]
impl VNode {
    fn construct(_: &mut Vm, args: &mut Args) -> SourceResult<Content> {
        let amount = args.expect("spacing")?;
        let weak = args.named("weak")?.unwrap_or(false);
        Ok(Self { amount, weak }.pack())
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
