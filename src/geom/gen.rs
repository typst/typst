use super::*;

/// A container with a main and cross component.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub struct Gen<T> {
    /// The main component.
    pub main: T,
    /// The cross component.
    pub cross: T,
}

impl<T> Gen<T> {
    /// Create a new instance from the two components.
    pub fn new(main: T, cross: T) -> Self {
        Self { main, cross }
    }

    /// Create a new instance with two equal components.
    pub fn uniform(value: T) -> Self
    where
        T: Clone,
    {
        Self { main: value.clone(), cross: value }
    }
}

impl Gen<Length> {
    /// The zero value.
    pub const ZERO: Self = Self { main: Length::ZERO, cross: Length::ZERO };
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

impl<T> Switch for Gen<T> {
    type Other = Spec<T>;

    fn switch(self, dirs: LayoutDirs) -> Self::Other {
        match dirs.main.axis() {
            SpecAxis::Horizontal => Spec::new(self.main, self.cross),
            SpecAxis::Vertical => Spec::new(self.cross, self.main),
        }
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

impl Switch for GenAxis {
    type Other = SpecAxis;

    fn switch(self, dirs: LayoutDirs) -> Self::Other {
        match self {
            Self::Main => dirs.main.axis(),
            Self::Cross => dirs.cross.axis(),
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
