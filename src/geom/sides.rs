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
}

impl Sides<Length> {
    /// A size with `left` and `right` summed into `width`, and `top` and
    /// `bottom` summed into `height`.
    pub fn size(self) -> Size {
        Size::new(self.left + self.right, self.top + self.bottom)
    }
}

impl Sides<Linear> {
    /// Resolve the linear sides relative to the given `size`.
    pub fn resolve(self, size: Size) -> Sides<Length> {
        Sides {
            left: self.left.resolve(size.w),
            top: self.top.resolve(size.h),
            right: self.right.resolve(size.w),
            bottom: self.bottom.resolve(size.h),
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
}
