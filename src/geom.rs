//! Geometrical types.

#[doc(no_inline)]
pub use kurbo::*;

use crate::layout::primitive::{Dir, GenAlign, LayoutAlign, LayoutSystem, SpecAxis};

/// Additional methods for [sizes].
///
/// [sizes]: ../../kurbo/struct.Size.html
pub trait SizeExt {
    /// Return the primary component of this specialized size.
    fn primary(self, sys: LayoutSystem) -> f64;

    /// Borrow the primary component of this specialized size mutably.
    fn primary_mut(&mut self, sys: LayoutSystem) -> &mut f64;

    /// Return the secondary component of this specialized size.
    fn secondary(self, sys: LayoutSystem) -> f64;

    /// Borrow the secondary component of this specialized size mutably.
    fn secondary_mut(&mut self, sys: LayoutSystem) -> &mut f64;

    /// Returns the generalized version of a `Size` based on the layouting
    /// system, that is:
    /// - `x` describes the primary axis instead of the horizontal one.
    /// - `y` describes the secondary axis instead of the vertical one.
    fn generalized(self, sys: LayoutSystem) -> Self;

    /// Returns the specialized version of this generalized Size2D (inverse to
    /// `generalized`).
    fn specialized(self, sys: LayoutSystem) -> Self;

    /// Whether the given size fits into this one, that is, both coordinate
    /// values are smaller or equal.
    fn fits(self, other: Self) -> bool;

    /// The anchor position along the given axis for an item with the given
    /// alignment in a container with this size.
    ///
    /// This assumes the size to be generalized such that `x` corresponds to the
    /// primary axis.
    fn anchor(self, align: LayoutAlign, sys: LayoutSystem) -> Point;
}

impl SizeExt for Size {
    fn primary(self, sys: LayoutSystem) -> f64 {
        if sys.primary.axis() == SpecAxis::Horizontal {
            self.width
        } else {
            self.height
        }
    }

    fn primary_mut(&mut self, sys: LayoutSystem) -> &mut f64 {
        if sys.primary.axis() == SpecAxis::Horizontal {
            &mut self.width
        } else {
            &mut self.height
        }
    }

    fn secondary(self, sys: LayoutSystem) -> f64 {
        if sys.primary.axis() == SpecAxis::Horizontal {
            self.height
        } else {
            self.width
        }
    }

    fn secondary_mut(&mut self, sys: LayoutSystem) -> &mut f64 {
        if sys.primary.axis() == SpecAxis::Horizontal {
            &mut self.height
        } else {
            &mut self.width
        }
    }

    fn generalized(self, sys: LayoutSystem) -> Self {
        match sys.primary.axis() {
            SpecAxis::Horizontal => self,
            SpecAxis::Vertical => Self::new(self.height, self.width),
        }
    }

    fn specialized(self, sys: LayoutSystem) -> Self {
        // In fact, generalized is its own inverse. For reasons of clarity
        // at the call site, we still have this second function.
        self.generalized(sys)
    }

    fn fits(self, other: Self) -> bool {
        self.width >= other.width && self.height >= other.height
    }

    fn anchor(self, align: LayoutAlign, sys: LayoutSystem) -> Point {
        fn length_anchor(length: f64, align: GenAlign, dir: Dir) -> f64 {
            match (dir.is_positive(), align) {
                (true, GenAlign::Start) | (false, GenAlign::End) => 0.0,
                (_, GenAlign::Center) => length / 2.0,
                (true, GenAlign::End) | (false, GenAlign::Start) => length,
            }
        }

        Point::new(
            length_anchor(self.width, align.primary, sys.primary),
            length_anchor(self.height, align.secondary, sys.secondary),
        )
    }
}

/// Additional methods for [rectangles].
///
/// [rectangles]: ../../kurbo/struct.Rect.html
pub trait RectExt {
    /// Get a mutable reference to the value for the specified direction at the
    /// alignment.
    ///
    /// Center alignment is treated the same as origin alignment.
    fn get_mut(&mut self, dir: Dir, align: GenAlign) -> &mut f64;
}

impl RectExt for Rect {
    fn get_mut(&mut self, dir: Dir, align: GenAlign) -> &mut f64 {
        match if align == GenAlign::End { dir.inv() } else { dir } {
            Dir::LTR => &mut self.x0,
            Dir::TTB => &mut self.y0,
            Dir::RTL => &mut self.x1,
            Dir::BTT => &mut self.y1,
        }
    }
}

/// A generic container for `[left, top, right, bottom]` values.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
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
    /// Create a new box from four sizes.
    pub fn new(left: T, top: T, right: T, bottom: T) -> Self {
        Self { left, top, right, bottom }
    }

    /// Create an instance with all four components set to the same `value`.
    pub fn uniform(value: T) -> Self
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

    /// Get a mutable reference to the value for the specified direction at the
    /// alignment.
    ///
    /// Center alignment is treated the same as origin alignment.
    pub fn get_mut(&mut self, dir: Dir, align: GenAlign) -> &mut T {
        match if align == GenAlign::End { dir.inv() } else { dir } {
            Dir::LTR => &mut self.left,
            Dir::RTL => &mut self.right,
            Dir::TTB => &mut self.top,
            Dir::BTT => &mut self.bottom,
        }
    }
}
