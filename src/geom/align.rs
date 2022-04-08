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
    /// Top-left alignment.
    pub const LEFT_TOP: Spec<Self> = Spec { x: Align::Left, y: Align::Top };

    /// Center-horizon alignment.
    pub const CENTER_HORIZON: Spec<Self> = Spec { x: Align::Center, y: Align::Horizon };

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

    /// Returns the position of this alignment in the given length.
    pub fn position(self, length: Length) -> Length {
        match self {
            Self::Left | Self::Top => Length::zero(),
            Self::Center | Self::Horizon => length / 2.0,
            Self::Right | Self::Bottom => length,
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
