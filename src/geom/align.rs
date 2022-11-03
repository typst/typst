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
    pub const LEFT_TOP: Axes<Self> = Axes { x: Align::Left, y: Align::Top };

    /// Center-horizon alignment.
    pub const CENTER_HORIZON: Axes<Self> = Axes { x: Align::Center, y: Align::Horizon };

    /// The axis this alignment belongs to.
    pub const fn axis(self) -> Axis {
        match self {
            Self::Left | Self::Center | Self::Right => Axis::X,
            Self::Top | Self::Horizon | Self::Bottom => Axis::Y,
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

    /// Returns the position of this alignment in a container with the given
    /// extentq.
    pub fn position(self, extent: Abs) -> Abs {
        match self {
            Self::Left | Self::Top => Abs::zero(),
            Self::Center | Self::Horizon => extent / 2.0,
            Self::Right | Self::Bottom => extent,
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

/// The generic alignment representation.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum GenAlign {
    /// Align at the start side of the text direction.
    Start,
    /// Align at the end side of the text direction.
    End,
    /// Align at a specific alignment.
    Specific(Align),
}

impl GenAlign {
    /// The axis this alignment belongs to.
    pub const fn axis(self) -> Axis {
        match self {
            Self::Start | Self::End => Axis::X,
            Self::Specific(align) => align.axis(),
        }
    }
}

impl From<Align> for GenAlign {
    fn from(align: Align) -> Self {
        Self::Specific(align)
    }
}

impl Debug for GenAlign {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Start => f.pad("start"),
            Self::End => f.pad("end"),
            Self::Specific(align) => align.fmt(f),
        }
    }
}
