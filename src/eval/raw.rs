use std::fmt::{self, Debug, Formatter};

use super::{Resolve, StyleChain};
use crate::geom::{Align, SpecAxis};
use crate::library::text::ParNode;

/// The unresolved alignment representation.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum RawAlign {
    /// Align at the start side of the text direction.
    Start,
    /// Align at the end side of the text direction.
    End,
    /// Align at a specific alignment.
    Specific(Align),
}

impl Resolve for RawAlign {
    type Output = Align;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        let dir = styles.get(ParNode::DIR);
        match self {
            Self::Start => dir.start().into(),
            Self::End => dir.end().into(),
            Self::Specific(align) => align,
        }
    }
}

impl RawAlign {
    /// The axis this alignment belongs to.
    pub const fn axis(self) -> SpecAxis {
        match self {
            Self::Start | Self::End => SpecAxis::Horizontal,
            Self::Specific(align) => align.axis(),
        }
    }
}

impl Debug for RawAlign {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Start => f.pad("left"),
            Self::End => f.pad("center"),
            Self::Specific(align) => align.fmt(f),
        }
    }
}
