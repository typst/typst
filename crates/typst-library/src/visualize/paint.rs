use std::fmt::{self, Debug, Formatter};

use ecow::EcoString;

use crate::foundations::{Repr, Smart, cast};
use crate::visualize::{Color, Gradient, RelativeTo, Tiling};

/// How a fill or stroke should be painted.
#[derive(Clone, Eq, PartialEq, Hash)]
pub enum Paint {
    /// A solid color.
    Solid(Color),
    /// A gradient.
    Gradient(Gradient),
    /// A tiling.
    Tiling(Tiling),
}

impl Paint {
    /// Unwraps a solid color used for text rendering.
    pub fn unwrap_solid(&self) -> Color {
        match self {
            Self::Solid(color) => *color,
            Self::Gradient(_) | Self::Tiling(_) => panic!("expected solid color"),
        }
    }

    /// Gets the relative coordinate system for this paint.
    pub fn relative(&self) -> Smart<RelativeTo> {
        match self {
            Self::Solid(_) => Smart::Auto,
            Self::Gradient(gradient) => gradient.relative(),
            Self::Tiling(tiling) => tiling.relative(),
        }
    }

    /// Turns this paint into a paint for a text decoration.
    ///
    /// If this paint is a gradient, it will be converted to a gradient with
    /// relative set to [`RelativeTo::Parent`].
    pub fn as_decoration(&self) -> Self {
        match self {
            Self::Solid(color) => Self::Solid(*color),
            Self::Gradient(gradient) => {
                Self::Gradient(gradient.clone().with_relative(RelativeTo::Parent))
            }
            Self::Tiling(tiling) => {
                Self::Tiling(tiling.clone().with_relative(RelativeTo::Parent))
            }
        }
    }
}

impl Debug for Paint {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Solid(v) => v.fmt(f),
            Self::Gradient(v) => v.fmt(f),
            Self::Tiling(v) => v.fmt(f),
        }
    }
}

impl From<Tiling> for Paint {
    fn from(tiling: Tiling) -> Self {
        Self::Tiling(tiling)
    }
}

impl Repr for Paint {
    fn repr(&self) -> EcoString {
        match self {
            Self::Solid(color) => color.repr(),
            Self::Gradient(gradient) => gradient.repr(),
            Self::Tiling(tiling) => tiling.repr(),
        }
    }
}

impl<T: Into<Color>> From<T> for Paint {
    fn from(t: T) -> Self {
        Self::Solid(t.into())
    }
}

impl From<Gradient> for Paint {
    fn from(gradient: Gradient) -> Self {
        Self::Gradient(gradient)
    }
}

cast! {
    Paint,
    self => match self {
        Self::Solid(color) => color.into_value(),
        Self::Gradient(gradient) => gradient.into_value(),
        Self::Tiling(tiling) => tiling.into_value(),
    },
    color: Color => Self::Solid(color),
    gradient: Gradient => Self::Gradient(gradient),
    tiling: Tiling => Self::Tiling(tiling),
}
