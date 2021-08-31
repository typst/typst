use super::*;

/// Where to align something along an axis.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Align {
    /// Align at the start of the axis.
    Start,
    /// Align in the middle of the axis.
    Center,
    /// Align at the end of the axis.
    End,
    /// Align at the left side of the axis.
    Left,
    /// Align at the right side of the axis.
    Right,
    /// Align at the top side of the axis.
    Top,
    /// Align at the bottom side of the axis.
    Bottom,
}

impl Align {
    /// The axis this alignment belongs to if it is specific.
    pub fn axis(self) -> Option<SpecAxis> {
        match self {
            Self::Start => None,
            Self::Center => None,
            Self::End => None,
            Self::Left => Some(SpecAxis::Horizontal),
            Self::Right => Some(SpecAxis::Horizontal),
            Self::Top => Some(SpecAxis::Vertical),
            Self::Bottom => Some(SpecAxis::Vertical),
        }
    }

    /// The inverse alignment.
    pub fn inv(self) -> Self {
        match self {
            Self::Start => Self::End,
            Self::Center => Self::Center,
            Self::End => Self::Start,
            Self::Left => Self::Right,
            Self::Right => Self::Left,
            Self::Top => Self::Bottom,
            Self::Bottom => Self::Top,
        }
    }

    /// Returns the position of this alignment in the given range.
    pub fn resolve(self, dir: Dir, range: Range<Length>) -> Length {
        #[cfg(debug_assertions)]
        if let Some(axis) = self.axis() {
            debug_assert_eq!(axis, dir.axis())
        }

        match self {
            Self::Start => {
                if dir.is_positive() {
                    range.start
                } else {
                    range.end
                }
            }
            Self::Center => (range.start + range.end) / 2.0,
            Self::End => {
                if dir.is_positive() {
                    range.end
                } else {
                    range.start
                }
            }
            Self::Left | Self::Top => range.start,
            Self::Right | Self::Bottom => range.end,
        }
    }
}

impl Default for Align {
    fn default() -> Self {
        Self::Start
    }
}

impl Debug for Align {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(match self {
            Self::Start => "start",
            Self::Center => "center",
            Self::End => "end",
            Self::Left => "left",
            Self::Right => "right",
            Self::Top => "top",
            Self::Bottom => "bottom",
        })
    }
}
