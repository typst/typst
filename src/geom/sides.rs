use super::*;

/// A container with left, top, right and bottom components.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Sides<T> {
    /// The value for the left side.
    pub left: T,
    /// The value for the top side.
    pub top: T,
    /// The value for the right side.
    pub right: T,
    /// The value for the bottom side.
    pub bottom: T,
}

impl<T> Sides<T> {
    /// Create a new instance from the four components.
    pub const fn new(left: T, top: T, right: T, bottom: T) -> Self {
        Self { left, top, right, bottom }
    }

    /// Create an instance with four equal components.
    pub fn splat(value: T) -> Self
    where
        T: Clone,
    {
        Self {
            left: value.clone(),
            top: value.clone(),
            right: value.clone(),
            bottom: value,
        }
    }

    /// Map the individual fields with `f`.
    pub fn map<F, U>(self, mut f: F) -> Sides<U>
    where
        F: FnMut(T) -> U,
    {
        Sides {
            left: f(self.left),
            top: f(self.top),
            right: f(self.right),
            bottom: f(self.bottom),
        }
    }

    /// Zip two instances into an instance.
    pub fn zip<F, V, W>(self, other: Sides<V>, mut f: F) -> Sides<W>
    where
        F: FnMut(T, V) -> W,
    {
        Sides {
            left: f(self.left, other.left),
            top: f(self.top, other.top),
            right: f(self.right, other.right),
            bottom: f(self.bottom, other.bottom),
        }
    }

    /// An iterator over the sides, starting with the left side, clockwise.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        [&self.left, &self.top, &self.right, &self.bottom].into_iter()
    }

    /// Whether all sides are equal.
    pub fn is_uniform(&self) -> bool
    where
        T: PartialEq,
    {
        self.left == self.top && self.top == self.right && self.right == self.bottom
    }
}

impl<T: Add> Sides<T> {
    /// Sums up `left` and `right` into `x`, and `top` and `bottom` into `y`.
    pub fn sum_by_axis(self) -> Spec<T::Output> {
        Spec::new(self.left + self.right, self.top + self.bottom)
    }
}

impl Sides<Relative<Length>> {
    /// Evaluate the sides relative to the given `size`.
    pub fn relative_to(self, size: Size) -> Sides<Length> {
        Sides {
            left: self.left.relative_to(size.x),
            top: self.top.relative_to(size.y),
            right: self.right.relative_to(size.x),
            bottom: self.bottom.relative_to(size.y),
        }
    }
}

impl<T> Get<Side> for Sides<T> {
    type Component = T;

    fn get(self, side: Side) -> T {
        match side {
            Side::Left => self.left,
            Side::Top => self.top,
            Side::Right => self.right,
            Side::Bottom => self.bottom,
        }
    }

    fn get_mut(&mut self, side: Side) -> &mut T {
        match side {
            Side::Left => &mut self.left,
            Side::Top => &mut self.top,
            Side::Right => &mut self.right,
            Side::Bottom => &mut self.bottom,
        }
    }
}

/// The four sides of objects.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Side {
    /// The left side.
    Left,
    /// The top side.
    Top,
    /// The right side.
    Right,
    /// The bottom side.
    Bottom,
}

impl Side {
    /// The opposite side.
    pub fn inv(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Top => Self::Bottom,
            Self::Right => Self::Left,
            Self::Bottom => Self::Top,
        }
    }

    /// The next side, clockwise.
    pub fn next_cw(self) -> Self {
        match self {
            Self::Left => Self::Top,
            Self::Top => Self::Right,
            Self::Right => Self::Bottom,
            Self::Bottom => Self::Left,
        }
    }

    /// The next side, counter-clockwise.
    pub fn next_ccw(self) -> Self {
        match self {
            Self::Left => Self::Bottom,
            Self::Top => Self::Left,
            Self::Right => Self::Top,
            Self::Bottom => Self::Right,
        }
    }

    /// The first corner of the side in clockwise order.
    pub fn start_corner(self) -> Corner {
        match self {
            Self::Left => Corner::BottomLeft,
            Self::Top => Corner::TopLeft,
            Self::Right => Corner::TopRight,
            Self::Bottom => Corner::BottomRight,
        }
    }

    /// The second corner of the side in clockwise order.
    pub fn end_corner(self) -> Corner {
        self.next_cw().start_corner()
    }

    /// Return the corresponding axis.
    pub fn axis(self) -> SpecAxis {
        match self {
            Self::Left | Self::Right => SpecAxis::Vertical,
            Self::Top | Self::Bottom => SpecAxis::Horizontal,
        }
    }
}
