use super::*;

/// A container with a horizontal and vertical component.
#[derive(Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Spec<T> {
    /// The horizontal component.
    pub horizontal: T,
    /// The vertical component.
    pub vertical: T,
}

impl<T> Spec<T> {
    /// Create a new instance from the two components.
    pub fn new(horizontal: T, vertical: T) -> Self {
        Self { horizontal, vertical }
    }

    /// Create a new instance with two equal components.
    pub fn splat(value: T) -> Self
    where
        T: Clone,
    {
        Self {
            horizontal: value.clone(),
            vertical: value,
        }
    }

    /// Convert to the generic representation.
    pub fn to_gen(self, main: SpecAxis) -> Gen<T> {
        match main {
            SpecAxis::Horizontal => Gen::new(self.vertical, self.horizontal),
            SpecAxis::Vertical => Gen::new(self.horizontal, self.vertical),
        }
    }
}

impl Spec<Length> {
    /// The zero value.
    pub fn zero() -> Self {
        Self {
            horizontal: Length::zero(),
            vertical: Length::zero(),
        }
    }

    /// Convert to a point.
    pub fn to_point(self) -> Point {
        Point::new(self.horizontal, self.vertical)
    }

    /// Convert to a size.
    pub fn to_size(self) -> Size {
        Size::new(self.horizontal, self.vertical)
    }
}

impl<T> Get<SpecAxis> for Spec<T> {
    type Component = T;

    fn get(self, axis: SpecAxis) -> T {
        match axis {
            SpecAxis::Horizontal => self.horizontal,
            SpecAxis::Vertical => self.vertical,
        }
    }

    fn get_mut(&mut self, axis: SpecAxis) -> &mut T {
        match axis {
            SpecAxis::Horizontal => &mut self.horizontal,
            SpecAxis::Vertical => &mut self.vertical,
        }
    }
}

impl<T: Debug> Debug for Spec<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Spec({:?}, {:?})", self.horizontal, self.vertical)
    }
}

/// The two specific layouting axes.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum SpecAxis {
    /// The vertical layouting axis.
    Vertical,
    /// The horizontal layouting axis.
    Horizontal,
}

impl SpecAxis {
    /// The direction with the given positivity for this axis.
    pub fn dir(self, positive: bool) -> Dir {
        match (self, positive) {
            (Self::Vertical, true) => Dir::TTB,
            (Self::Vertical, false) => Dir::BTT,
            (Self::Horizontal, true) => Dir::LTR,
            (Self::Horizontal, false) => Dir::RTL,
        }
    }

    /// The other axis.
    pub fn other(self) -> Self {
        match self {
            Self::Horizontal => Self::Vertical,
            Self::Vertical => Self::Horizontal,
        }
    }
}

impl Display for SpecAxis {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(match self {
            Self::Vertical => "vertical",
            Self::Horizontal => "horizontal",
        })
    }
}
