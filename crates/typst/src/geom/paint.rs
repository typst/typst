use super::*;

/// How a fill or stroke should be painted.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Paint {
    /// A solid color.
    Solid(Color),
    /// A gradient.
    Gradient(Gradient),
    /// A pattern.
    Pattern(Pattern),
}

impl Paint {
    /// Unwraps a solid color used for text rendering.
    pub fn unwrap_solid(&self) -> Color {
        match self {
            Self::Solid(color) => *color,
            Self::Gradient(_) | Self::Pattern(_) => panic!("expected solid color"),
        }
    }

    /// Gets the relative coordinate system for this paint.
    pub fn relative(&self) -> Smart<Relative> {
        match self {
            Self::Solid(_) => Smart::Auto,
            Self::Gradient(gradient) => gradient.relative(),
            Self::Pattern(pattern) => pattern.relative,
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
            Self::Pattern(pattern) => {
                Self::Pattern(pattern.clone().with_relative(Relative::Parent))
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

impl From<Pattern> for Paint {
    fn from(pattern: Pattern) -> Self {
        Self::Pattern(pattern)
    }
}

impl Repr for Paint {
    fn repr(&self) -> EcoString {
        match self {
            Self::Solid(color) => color.repr(),
            Self::Gradient(gradient) => gradient.repr(),
            Self::Pattern(pattern) => pattern.repr(),
        }
    }
}

cast! {
    Paint,
    self => match self {
        Self::Solid(color) => Value::Color(color),
        Self::Gradient(gradient) => Value::Gradient(gradient),
        Self::Pattern(pattern) => Value::Pattern(pattern),
    },
    color: Color => Self::Solid(color),
    gradient: Gradient => Self::Gradient(gradient),
    pattern: Pattern => Self::Pattern(pattern),
}
