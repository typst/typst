use super::*;

/// An enum with the length that a grid cell may have.
#[derive(Copy, Clone, PartialEq, Hash)]
pub enum TrackSizing {
    /// A length stated in absolute values and fractions of the parent's size.
    Linear(Linear),
    /// A length that is the fraction of the remaining free space in the parent.
    Fractional(Fractional),
    /// The cell will fit its contents.
    Auto,
}

impl TrackSizing {
    pub fn is_zero(&self) -> bool {
        match self {
            Self::Linear(l) => l.is_zero(),
            Self::Fractional(f) => f.is_zero(),
            Self::Auto => false,
        }
    }

    pub fn preliminary_length(&self, resolve: Length) -> Length {
        match self {
            Self::Linear(l) => l.resolve(resolve),
            _ => resolve,
        }
    }
}

impl Display for TrackSizing {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Linear(x) => <Linear as Display>::fmt(x, f),
            Self::Fractional(x) => <Fractional as Display>::fmt(x, f),
            Self::Auto => write!(f, "auto"),
        }
    }
}

impl Debug for TrackSizing {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Linear(x) => <Linear as Debug>::fmt(x, f),
            Self::Fractional(x) => <Fractional as Debug>::fmt(x, f),
            Self::Auto => write!(f, "auto"),
        }
    }
}

impl From<Length> for TrackSizing {
    fn from(abs: Length) -> Self {
        Self::Linear(abs.into())
    }
}

impl From<Relative> for TrackSizing {
    fn from(rel: Relative) -> Self {
        Self::Linear(rel.into())
    }
}

impl From<Linear> for TrackSizing {
    fn from(lin: Linear) -> Self {
        Self::Linear(lin)
    }
}

impl From<Fractional> for TrackSizing {
    fn from(fr: Fractional) -> Self {
        Self::Fractional(fr)
    }
}
