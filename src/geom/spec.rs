use super::*;

/// A container with a horizontal and vertical component.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
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
    pub fn uniform(value: T) -> Self
    where
        T: Clone,
    {
        Self {
            horizontal: value.clone(),
            vertical: value,
        }
    }
}

impl Spec<Length> {
    /// The zero value.
    pub const ZERO: Self = Self {
        horizontal: Length::ZERO,
        vertical: Length::ZERO,
    };

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

impl<T> Switch for Spec<T> {
    type Other = Gen<T>;

    fn switch(self, flow: Flow) -> Self::Other {
        match flow.main.axis() {
            SpecAxis::Horizontal => Gen::new(self.horizontal, self.vertical),
            SpecAxis::Vertical => Gen::new(self.vertical, self.horizontal),
        }
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
    /// The other axis.
    pub fn other(self) -> Self {
        match self {
            Self::Horizontal => Self::Vertical,
            Self::Vertical => Self::Horizontal,
        }
    }
}

impl Switch for SpecAxis {
    type Other = GenAxis;

    fn switch(self, flow: Flow) -> Self::Other {
        if self == flow.main.axis() {
            GenAxis::Main
        } else {
            debug_assert_eq!(self, flow.cross.axis());
            GenAxis::Cross
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
