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
    /// Temporary method to unwrap a solid color used for text rendering.
    pub fn unwrap_solid(&self) -> Color {
        // TODO: Implement gradients on text.
        match self {
            Self::Solid(color) => *color,
            Self::Gradient(_) => panic!("expected solid color"),
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
