use super::*;

/// A container with a main and cross component.
#[derive(Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Gen<T> {
    /// The cross component.
    pub cross: T,
    /// The main component.
    pub main: T,
}

impl<T> Gen<T> {
    /// Create a new instance from the two components.
    pub fn new(cross: T, main: T) -> Self {
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

    /// Convert to the specific representation.
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
            main: Length::zero(),
            cross: Length::zero(),
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

impl<T> Get<GenAxis> for Gen<T> {
    type Component = T;

    fn get(self, axis: GenAxis) -> T {
        match axis {
            GenAxis::Main => self.main,
            GenAxis::Cross => self.cross,
        }
    }

    fn get_mut(&mut self, axis: GenAxis) -> &mut T {
        match axis {
            GenAxis::Main => &mut self.main,
            GenAxis::Cross => &mut self.cross,
        }
    }
}

impl<T: Debug> Debug for Gen<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Gen({:?}, {:?})", self.main, self.cross)
    }
}

/// The two generic layouting axes.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum GenAxis {
    /// The axis pages and paragraphs are set along.
    Main,
    /// The axis words and lines are set along.
    Cross,
}

impl GenAxis {
    /// The other axis.
    pub fn other(self) -> Self {
        match self {
            Self::Main => Self::Cross,
            Self::Cross => Self::Main,
        }
    }
}

impl Display for GenAxis {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(match self {
            Self::Main => "main",
            Self::Cross => "cross",
        })
    }
}
