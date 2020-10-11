use std::fmt::{self, Debug, Formatter};

use super::*;

/// A node that inserts spacing.
#[derive(Copy, Clone, PartialEq)]
pub struct Spacing {
    pub amount: Length,
    pub softness: Softness,
}

#[async_trait(?Send)]
impl Layout for Spacing {
    async fn layout(&self, _: &mut LayoutContext, _: LayoutConstraints) -> Vec<Layouted> {
        vec![Layouted::Spacing(self.amount)]
    }
}

impl Debug for Spacing {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self.softness {
            Softness::Soft => write!(f, "Soft({})", self.amount),
            Softness::Hard => write!(f, "Hard({})", self.amount),
        }
    }
}

impl From<Spacing> for LayoutNode {
    fn from(spacing: Spacing) -> Self {
        Self::Spacing(spacing)
    }
}

/// Defines how spacing interacts with surrounding spacing.
///
/// Hard spacing assures that a fixed amount of spacing will always be inserted.
/// Soft spacing will be consumed by previous soft spacing or neighbouring hard
/// spacing and can be used to insert overridable spacing, e.g. between words or
/// paragraphs.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum Softness {
    /// Soft spacing is not laid out if it directly follows other soft spacing
    /// or if it touches hard spacing.
    Soft,
    /// Hard spacing is always laid out and consumes surrounding soft spacing.
    Hard,
}
