use super::*;

/// Where to align something along an axis.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Align {
    /// Align at the left side.
    Left,
    /// Align in the horizontal middle.
    Center,
    /// Align at the right side.
    Right,
    /// Align at the top side.
    Top,
    /// Align in the vertical middle.
    Horizon,
    /// Align at the bottom side.
    Bottom,
}

impl Align {
    /// The axis this alignment belongs to.
    pub const fn axis(self) -> SpecAxis {
        match self {
            Self::Left | Self::Center | Self::Right => SpecAxis::Horizontal,
            Self::Top | Self::Horizon | Self::Bottom => SpecAxis::Vertical,
        }
    }

    /// The inverse alignment.
    pub const fn inv(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Center => Self::Center,
            Self::Right => Self::Left,
            Self::Top => Self::Bottom,
            Self::Horizon => Self::Horizon,
            Self::Bottom => Self::Top,
        }
    }

    /// Returns the position of this alignment in the given range.
    pub fn resolve(self, range: Range<Length>) -> Length {
        match self {
            Self::Left | Self::Top => range.start,
            Self::Center | Self::Horizon => (range.start + range.end) / 2.0,
            Self::Right | Self::Bottom => range.end,
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
            Self::Center => "center",
            Self::Right => "right",
            Self::Top => "top",
            Self::Horizon => "horizon",
            Self::Bottom => "bottom",
        })
    }
}
