use super::*;

/// The four directions into which content can be laid out.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
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
    /// The side this direction starts at.
    pub fn start(self) -> Side {
        match self {
            Self::LTR => Side::Left,
            Self::RTL => Side::Right,
            Self::TTB => Side::Top,
            Self::BTT => Side::Bottom,
        }
    }

    /// The side this direction ends at.
    pub fn end(self) -> Side {
        match self {
            Self::LTR => Side::Right,
            Self::RTL => Side::Left,
            Self::TTB => Side::Bottom,
            Self::BTT => Side::Top,
        }
    }

    /// The specific axis this direction belongs to.
    pub fn axis(self) -> SpecAxis {
        match self {
            Self::LTR | Self::RTL => SpecAxis::Horizontal,
            Self::TTB | Self::BTT => SpecAxis::Vertical,
        }
    }

    /// Whether this direction points into the positive coordinate direction.
    ///
    /// The positive directions are left-to-right and top-to-bottom.
    pub fn is_positive(self) -> bool {
        match self {
            Self::LTR | Self::TTB => true,
            Self::RTL | Self::BTT => false,
        }
    }

    /// The factor for this direction.
    ///
    /// - `1.0` if the direction is positive.
    /// - `-1.0` if the direction is negative.
    pub fn factor(self) -> f64 {
        if self.is_positive() { 1.0 } else { -1.0 }
    }

    /// The inverse direction.
    pub fn inv(self) -> Self {
        match self {
            Self::LTR => Self::RTL,
            Self::RTL => Self::LTR,
            Self::TTB => Self::BTT,
            Self::BTT => Self::TTB,
        }
    }
}

impl Display for Dir {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(match self {
            Self::LTR => "ltr",
            Self::RTL => "rtl",
            Self::TTB => "ttb",
            Self::BTT => "btt",
        })
    }
}
