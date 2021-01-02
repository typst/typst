use std::fmt::{self, Debug, Formatter};

use super::*;
use crate::eval::Softness;

/// A  spacing node.
#[derive(Copy, Clone, PartialEq)]
pub struct NodeSpacing {
    /// The amount of spacing to insert.
    pub amount: Length,
    /// Defines how spacing interacts with surrounding spacing.
    ///
    /// Hard spacing assures that a fixed amount of spacing will always be
    /// inserted. Soft spacing will be consumed by previous soft spacing or
    /// neighbouring hard spacing and can be used to insert overridable spacing,
    /// e.g. between words or paragraphs.
    ///
    /// This field is only used in evaluation, not in layouting.
    pub softness: Softness,
}

impl Layout for NodeSpacing {
    fn layout(&self, _: &mut LayoutContext, _: &Areas) -> Layouted {
        Layouted::Spacing(self.amount)
    }
}

impl Debug for NodeSpacing {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self.softness {
            Softness::Soft => write!(f, "Soft({})", self.amount),
            Softness::Hard => write!(f, "Hard({})", self.amount),
        }
    }
}

impl From<NodeSpacing> for Node {
    fn from(spacing: NodeSpacing) -> Self {
        Self::Spacing(spacing)
    }
}
