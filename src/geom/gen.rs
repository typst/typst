use super::*;

/// A container with an inline and a block component.
#[derive(Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Gen<T> {
    /// The inline component.
    pub inline: T,
    /// The block component.
    pub block: T,
}

impl<T> Gen<T> {
    /// Create a new instance from the two components.
    pub fn new(inline: T, block: T) -> Self {
        Self { inline, block }
    }

    /// Create a new instance with two equal components.
    pub fn splat(value: T) -> Self
    where
        T: Clone,
    {
        Self { inline: value.clone(), block: value }
    }

    /// Maps the individual fields with `f`.
    pub fn map<F, U>(self, mut f: F) -> Gen<U>
    where
        F: FnMut(T) -> U,
    {
        Gen {
            inline: f(self.inline),
            block: f(self.block),
        }
    }

    /// Convert to the specific representation, given the current block axis.
    pub fn to_spec(self, block: SpecAxis) -> Spec<T> {
        match block {
            SpecAxis::Horizontal => Spec::new(self.block, self.inline),
            SpecAxis::Vertical => Spec::new(self.inline, self.block),
        }
    }
}

impl Gen<Length> {
    /// The zero value.
    pub fn zero() -> Self {
        Self {
            inline: Length::zero(),
            block: Length::zero(),
        }
    }

    /// Convert to a point.
    pub fn to_point(self, block: SpecAxis) -> Point {
        self.to_spec(block).to_point()
    }

    /// Convert to a size.
    pub fn to_size(self, block: SpecAxis) -> Size {
        self.to_spec(block).to_size()
    }
}

impl<T> Gen<Option<T>> {
    /// Unwrap the individual fields.
    pub fn unwrap_or(self, other: Gen<T>) -> Gen<T> {
        Gen {
            inline: self.inline.unwrap_or(other.inline),
            block: self.block.unwrap_or(other.block),
        }
    }
}

impl<T> Get<GenAxis> for Gen<T> {
    type Component = T;

    fn get(self, axis: GenAxis) -> T {
        match axis {
            GenAxis::Inline => self.inline,
            GenAxis::Block => self.block,
        }
    }

    fn get_mut(&mut self, axis: GenAxis) -> &mut T {
        match axis {
            GenAxis::Inline => &mut self.inline,
            GenAxis::Block => &mut self.block,
        }
    }
}

impl<T: Debug> Debug for Gen<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Gen({:?}, {:?})", self.inline, self.block)
    }
}

/// The two generic layouting axes.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum GenAxis {
    /// The axis words and lines are set along.
    Inline,
    /// The axis paragraphs and pages are set along.
    Block,
}

impl GenAxis {
    /// The other axis.
    pub fn other(self) -> Self {
        match self {
            Self::Inline => Self::Block,
            Self::Block => Self::Inline,
        }
    }
}
