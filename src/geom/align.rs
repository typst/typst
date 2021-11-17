use super::*;

/// Where to align something along an axis.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Align {
    /// Align at the left side of the axis.
    Left,
    /// Align at the top side of the axis.
    Top,
    /// Align in the middle of the axis.
    Center,
    /// Align at the right side of the axis.
    Right,
    /// Align at the bottom side of the axis.
    Bottom,
}

impl Align {
    /// The axis this alignment belongs to if it is specific.
    pub fn axis(self) -> Option<SpecAxis> {
        match self {
            Self::Left => Some(SpecAxis::Horizontal),
            Self::Top => Some(SpecAxis::Vertical),
            Self::Center => None,
            Self::Right => Some(SpecAxis::Horizontal),
            Self::Bottom => Some(SpecAxis::Vertical),
        }
    }

    /// The inverse alignment.
    pub fn inv(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Top => Self::Bottom,
            Self::Center => Self::Center,
            Self::Right => Self::Left,
            Self::Bottom => Self::Top,
        }
    }

    /// Returns the position of this alignment in the given range.
    pub fn resolve(self, range: Range<Length>) -> Length {
        match self {
            Self::Left | Self::Top => range.start,
            Self::Right | Self::Bottom => range.end,
            Self::Center => (range.start + range.end) / 2.0,
        }
    }
}

impl From<Side> for Align {
    fn from(side: Side) -> Self {
        match side {
            Side::Left => Self::Left,
            Side::Top => Self::Top,
            Side::Right => Self::Right,
            Side::Bottom => Self::Bottom,
        }
    }
}

impl Debug for Align {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(match self {
            Self::Left => "left",
            Self::Top => "top",
            Self::Center => "center",
            Self::Right => "right",
            Self::Bottom => "bottom",
        })
    }
}
