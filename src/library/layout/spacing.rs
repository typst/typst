use std::cmp::Ordering;

use crate::library::prelude::*;
use crate::library::text::ParNode;

/// Horizontal spacing.
pub struct HNode;

#[node]
impl HNode {
    fn construct(_: &mut Context, args: &mut Args) -> TypResult<Content> {
        let amount = args.expect("spacing")?;
        let weak = args.named("weak")?.unwrap_or(false);
        Ok(Content::Horizontal { amount, weak })
    }
}

/// Vertical spacing.
pub struct VNode;

#[node]
impl VNode {
    fn construct(_: &mut Context, args: &mut Args) -> TypResult<Content> {
        let amount = args.expect("spacing")?;
        let weak = args.named("weak")?.unwrap_or(false);
        Ok(Content::Vertical { amount, weak, generated: false })
    }
}

/// Kinds of spacing.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Spacing {
    /// Spacing specified in absolute terms and relative to the parent's size.
    Relative(Relative<RawLength>),
    /// Spacing specified as a fraction of the remaining free space in the
    /// parent.
    Fractional(Fraction),
}

impl Spacing {
    /// Whether this is fractional spacing.
    pub fn is_fractional(self) -> bool {
        matches!(self, Self::Fractional(_))
    }
}

impl From<Length> for Spacing {
    fn from(length: Length) -> Self {
        Self::Relative(length.into())
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

/// Spacing around and between block-level nodes, relative to paragraph spacing.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct BlockSpacing(Relative<RawLength>);

castable!(BlockSpacing: Relative<RawLength>);

impl Resolve for BlockSpacing {
    type Output = Length;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        let whole = styles.get(ParNode::SPACING);
        self.0.resolve(styles).relative_to(whole)
    }
}

impl From<Ratio> for BlockSpacing {
    fn from(ratio: Ratio) -> Self {
        Self(ratio.into())
    }
}
