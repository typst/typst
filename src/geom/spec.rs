use super::*;

/// A container with a horizontal and vertical component.
#[derive(Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Spec<T> {
    /// The horizontal component.
    pub x: T,
    /// The vertical component.
    pub y: T,
}

impl<T> Spec<T> {
    /// Create a new instance from the two components.
    pub fn new(x: T, y: T) -> Self {
        Self { x, y }
    }

    /// Create a new instance with two equal components.
    pub fn splat(v: T) -> Self
    where
        T: Clone,
    {
        Self { x: v.clone(), y: v }
    }

    /// Maps the individual fields with `f`.
    pub fn map<F, U>(self, mut f: F) -> Spec<U>
    where
        F: FnMut(T) -> U,
    {
        Spec { x: f(self.x), y: f(self.y) }
    }

    /// Zip two instances into an instance over a tuple.
    pub fn zip<U>(self, other: impl Into<Spec<U>>) -> Spec<(T, U)> {
        let other = other.into();
        Spec {
            x: (self.x, other.x),
            y: (self.y, other.y),
        }
    }

    /// Whether a condition is true for at least one of fields.
    pub fn any<F>(self, mut f: F) -> bool
    where
        F: FnMut(&T) -> bool,
    {
        f(&self.x) || f(&self.y)
    }

    /// Whether a condition is true for both fields.
    pub fn all<F>(self, mut f: F) -> bool
    where
        F: FnMut(&T) -> bool,
    {
        f(&self.x) && f(&self.y)
    }

    /// Convert to the generic representation.
    pub fn to_gen(self, block: SpecAxis) -> Gen<T> {
        match block {
            SpecAxis::Horizontal => Gen::new(self.y, self.x),
            SpecAxis::Vertical => Gen::new(self.x, self.y),
        }
    }
}

impl From<Size> for Spec<Length> {
    fn from(size: Size) -> Self {
        size.to_spec()
    }
}

impl Spec<Length> {
    /// The zero value.
    pub fn zero() -> Self {
        Self { x: Length::zero(), y: Length::zero() }
    }

    /// Convert to a point.
    pub fn to_point(self) -> Point {
        Point::new(self.x, self.y)
    }

    /// Convert to a size.
    pub fn to_size(self) -> Size {
        Size::new(self.x, self.y)
    }
}

impl<T> Spec<Option<T>> {
    /// Unwrap the individual fields.
    pub fn unwrap_or(self, other: Spec<T>) -> Spec<T> {
        Spec {
            x: self.x.unwrap_or(other.x),
            y: self.y.unwrap_or(other.y),
        }
    }
}

impl<T> Get<SpecAxis> for Spec<T> {
    type Component = T;

    fn get(self, axis: SpecAxis) -> T {
        match axis {
            SpecAxis::Horizontal => self.x,
            SpecAxis::Vertical => self.y,
        }
    }

    fn get_mut(&mut self, axis: SpecAxis) -> &mut T {
        match axis {
            SpecAxis::Horizontal => &mut self.x,
            SpecAxis::Vertical => &mut self.y,
        }
    }
}

impl<T: Debug> Debug for Spec<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Spec({:?}, {:?})", self.x, self.y)
    }
}

/// The two specific layouting axes.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum SpecAxis {
    /// The horizontal layouting axis.
    Horizontal,
    /// The vertical layouting axis.
    Vertical,
}

impl SpecAxis {
    /// The direction with the given positivity for this axis.
    pub fn dir(self, positive: bool) -> Dir {
        match (self, positive) {
            (Self::Horizontal, true) => Dir::LTR,
            (Self::Horizontal, false) => Dir::RTL,
            (Self::Vertical, true) => Dir::TTB,
            (Self::Vertical, false) => Dir::BTT,
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
