use super::*;

/// Where to align something along a directed axis.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Align {
    /// Align at the start of the axis.
    Start,
    /// Align in the middle of the axis.
    Center,
    /// Align at the end of the axis.
    End,
}

impl Align {
    /// Returns the position of this alignment in the given range.
    pub fn resolve(self, dir: Dir, range: Range<Length>) -> Length {
        match if dir.is_positive() { self } else { self.inv() } {
            Self::Start => range.start,
            Self::Center => (range.start + range.end) / 2.0,
            Self::End => range.end,
        }
    }

    /// The inverse alignment.
    pub fn inv(self) -> Self {
        match self {
            Self::Start => Self::End,
            Self::Center => Self::Center,
            Self::End => Self::Start,
        }
    }
}

impl Default for Align {
    fn default() -> Self {
        Self::Start
    }
}

impl Display for Align {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(match self {
            Self::Start => "start",
            Self::Center => "center",
            Self::End => "end",
        })
    }
}
