use crate::layout::{Point, Size};

/// A rectangle in 2D.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Rect {
    /// The top left corner (minimum coordinate).
    pub min: Point,
    /// The bottom right corner (maximum coordinate).
    pub max: Point,
}

impl Rect {
    /// Create a new rectangle from the minimum/maximum coordinate.
    pub fn new(min: Point, max: Point) -> Self {
        Self { min, max }
    }

    /// Create a new rectangle from the position and size.
    pub fn from_pos_size(pos: Point, size: Size) -> Self {
        Self { min: pos, max: pos + size.to_point() }
    }

    /// Compute the size of the rectangle.
    pub fn size(&self) -> Size {
        Size::new(self.max.x - self.min.x, self.max.y - self.min.y)
    }
}
