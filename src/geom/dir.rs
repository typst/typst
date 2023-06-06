use super::*;

/// The four directions into which content can be laid out.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub enum Dir {
    /// Left to right.
    LTR,
    /// Right to left.
    RTL,
    /// Top to bottom.
    TTB,
    /// Bottom to top.
    BTT,
}

impl Dir {
    /// The specific axis this direction belongs to.
    pub const fn axis(self) -> Axis {
        match self {
            Self::LTR | Self::RTL => Axis::X,
            Self::TTB | Self::BTT => Axis::Y,
        }
    }

    /// The side this direction starts at.
    pub const fn start(self) -> Side {
        match self {
            Self::LTR => Side::Left,
            Self::RTL => Side::Right,
            Self::TTB => Side::Top,
            Self::BTT => Side::Bottom,
        }
    }

    /// The side this direction ends at.
    pub const fn end(self) -> Side {
        match self {
            Self::LTR => Side::Right,
            Self::RTL => Side::Left,
            Self::TTB => Side::Bottom,
            Self::BTT => Side::Top,
        }
    }

    /// The inverse direction.
    pub const fn inv(self) -> Self {
        match self {
            Self::LTR => Self::RTL,
            Self::RTL => Self::LTR,
            Self::TTB => Self::BTT,
            Self::BTT => Self::TTB,
        }
    }

    /// Whether this direction points into the positive coordinate direction.
    ///
    /// The positive directions are left-to-right and top-to-bottom.
    pub const fn is_positive(self) -> bool {
        match self {
            Self::LTR | Self::TTB => true,
            Self::RTL | Self::BTT => false,
        }
    }
}

impl Debug for Dir {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(match self {
            Self::LTR => "ltr",
            Self::RTL => "rtl",
            Self::TTB => "ttb",
            Self::BTT => "btt",
        })
    }
}

cast! {
    type Dir: "direction",
}
