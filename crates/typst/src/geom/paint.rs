use super::*;

/// How a fill or stroke should be painted.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Paint {
    /// A solid color.
    Solid(Color),
    /// A gradient.
    Gradient(Gradient),
}

impl Paint {
    /// Unwraps a solid color used for text rendering.
    pub fn unwrap_solid(&self) -> Color {
        match self {
            Self::Solid(color) => *color,
            Self::Gradient(_) => panic!("expected solid color"),
        }
    }

    /// Turns this paint into a paint for a text decoration.
    ///
    /// If this paint is a gradient, it will be converted to a gradient with
    /// relative set to [`Relative::Parent`].
    pub fn as_decoration(&self) -> Self {
        match self {
            Self::Solid(color) => Self::Solid(*color),
            Self::Gradient(gradient) => {
                Self::Gradient(gradient.clone().with_relative(Relative::Parent))
            }
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

impl Repr for Paint {
    fn repr(&self) -> EcoString {
        match self {
            Self::Solid(color) => color.repr(),
            Self::Gradient(gradient) => gradient.repr(),
        }
    }
}

cast! {
    Paint,
    self => match self {
        Self::Solid(color) => Value::Color(color),
        Self::Gradient(gradient) => Value::Gradient(gradient),
    },
    color: Color => Self::Solid(color),
    gradient: Gradient => Self::Gradient(gradient),
}
