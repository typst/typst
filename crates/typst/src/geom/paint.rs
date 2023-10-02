use super::*;
use ecow::EcoString;

/// How a fill or stroke should be painted.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Paint {
    /// A solid color.
    Solid(Color),
}

impl<T: Into<Color>> From<T> for Paint {
    fn from(t: T) -> Self {
        Self::Solid(t.into())
    }
}

impl Repr for Paint {
    fn repr(&self) -> EcoString {
        match self {
            Self::Solid(color) => color.repr(),
        }
    }
}

cast! {
    Paint,
    self => match self {
        Self::Solid(color) => Value::Color(color),
    },
    color: Color => Self::Solid(color),
}
