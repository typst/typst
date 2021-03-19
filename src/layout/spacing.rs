use std::fmt::{self, Debug, Formatter};

use super::*;

/// A node that adds spacing to its parent.
#[derive(Copy, Clone, PartialEq)]
pub struct SpacingNode {
    /// The amount of spacing to insert.
    pub amount: Length,
    /// Defines how spacing interacts with surrounding spacing.
    ///
    /// Hard spacing (`softness = 0`) assures that a fixed amount of spacing
    /// will always be inserted. Soft spacing (`softness >= 1`) will be consumed
    /// by other spacing with lower softness and can be used to insert
    /// overridable spacing, e.g. between words or paragraphs.
    pub softness: u8,
}

impl Layout for SpacingNode {
    fn layout(&self, _: &mut LayoutContext, _: &Areas) -> Fragment {
        Fragment::Spacing(self.amount)
    }
}

impl Debug for SpacingNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Spacing({}, {})", self.amount, self.softness)
    }
}

impl From<SpacingNode> for Node {
    fn from(spacing: SpacingNode) -> Self {
        Self::Spacing(spacing)
    }
}
