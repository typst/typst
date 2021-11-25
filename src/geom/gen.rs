use super::*;

/// A container with a main and cross component.
#[derive(Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Gen<T> {
    /// The main component.
    pub cross: T,
    /// The cross component.
    pub main: T,
}

impl<T> Gen<T> {
    /// Create a new instance from the two components.
    pub const fn new(cross: T, main: T) -> Self {
        Self { cross, main }
    }

    /// Create a new instance with two equal components.
    pub fn splat(value: T) -> Self
    where
        T: Clone,
    {
        Self { cross: value.clone(), main: value }
    }

    /// Maps the individual fields with `f`.
    pub fn map<F, U>(self, mut f: F) -> Gen<U>
    where
        F: FnMut(T) -> U,
    {
        Gen { cross: f(self.cross), main: f(self.main) }
    }

    /// Convert to the specific representation, given the current block axis.
    pub fn to_spec(self, main: SpecAxis) -> Spec<T> {
        match main {
            SpecAxis::Horizontal => Spec::new(self.main, self.cross),
            SpecAxis::Vertical => Spec::new(self.cross, self.main),
        }
    }
}

impl Gen<Length> {
    /// The zero value.
    pub fn zero() -> Self {
        Self {
            cross: Length::zero(),
            main: Length::zero(),
        }
    }

    /// Convert to a point.
    pub fn to_point(self, main: SpecAxis) -> Point {
        self.to_spec(main).to_point()
    }

    /// Convert to a size.
    pub fn to_size(self, main: SpecAxis) -> Size {
        self.to_spec(main).to_size()
    }
}

impl<T> Gen<Option<T>> {
    /// Unwrap the individual fields.
    pub fn unwrap_or(self, other: Gen<T>) -> Gen<T> {
        Gen {
            cross: self.cross.unwrap_or(other.cross),
            main: self.main.unwrap_or(other.main),
        }
    }
}

impl<T> Get<GenAxis> for Gen<T> {
    type Component = T;

    fn get(self, axis: GenAxis) -> T {
        match axis {
            GenAxis::Cross => self.cross,
            GenAxis::Main => self.main,
        }
    }

    fn get_mut(&mut self, axis: GenAxis) -> &mut T {
        match axis {
            GenAxis::Cross => &mut self.cross,
            GenAxis::Main => &mut self.main,
        }
    }
}

impl<T: Debug> Debug for Gen<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Gen({:?}, {:?})", self.cross, self.main)
    }
}

/// Two generic axes of a container.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum GenAxis {
    /// The minor axis.
    Cross,
    /// The major axis.
    Main,
}

impl GenAxis {
    /// The other axis.
    pub fn other(self) -> Self {
        match self {
            Self::Cross => Self::Main,
            Self::Main => Self::Cross,
        }
    }
}
