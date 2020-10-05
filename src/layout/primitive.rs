//! Layouting primitives.

use std::fmt::{self, Display, Formatter};

/// Specifies the directions into which content is laid out.
///
/// The primary component defines into which direction text and lines flow and the
/// secondary into which paragraphs and pages grow.
pub type LayoutSystem = Gen2<Dir>;

impl Default for LayoutSystem {
    fn default() -> Self {
        Self::new(Dir::LTR, Dir::TTB)
    }
}

/// Specifies where to align a layout in a parent container.
pub type LayoutAlign = Gen2<GenAlign>;

impl Default for LayoutAlign {
    fn default() -> Self {
        Self::new(GenAlign::Start, GenAlign::Start)
    }
}

/// Whether to expand a layout to an area's full size or shrink it to fit its content.
pub type LayoutExpansion = Spec2<bool>;

/// The four directions into which content can be laid out.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Dir {
    /// Left to right.
    LTR,
    /// Right to left.
    RTL,
    /// Top to bottom.
    TTB,
    /// Bottom to top.
    BTT,
}

impl Dir {
    /// The specific axis this direction belongs to.
    pub fn axis(self) -> SpecAxis {
        match self {
            Self::LTR | Self::RTL => SpecAxis::Horizontal,
            Self::TTB | Self::BTT => SpecAxis::Vertical,
        }
    }

    /// Whether this direction points into the positive coordinate direction.
    ///
    /// The positive directions are left-to-right and top-to-bottom.
    pub fn is_positive(self) -> bool {
        match self {
            Self::LTR | Self::TTB => true,
            Self::RTL | Self::BTT => false,
        }
    }

    /// The factor for this direction.
    ///
    /// - `1.0` if the direction is positive.
    /// - `-1.0` if the direction is negative.
    pub fn factor(self) -> f64 {
        if self.is_positive() { 1.0 } else { -1.0 }
    }

    /// The inverse direction.
    pub fn inv(self) -> Self {
        match self {
            Self::LTR => Self::RTL,
            Self::RTL => Self::LTR,
            Self::TTB => Self::BTT,
            Self::BTT => Self::TTB,
        }
    }

    /// The side of this direction the alignment identifies.
    ///
    /// `Center` alignment is treated the same as `Start` alignment.
    pub fn side(self, align: GenAlign) -> Side {
        match if align == GenAlign::End { self.inv() } else { self } {
            Self::LTR => Side::Left,
            Self::RTL => Side::Right,
            Self::TTB => Side::Top,
            Self::BTT => Side::Bottom,
        }
    }
}

impl Display for Dir {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(match self {
            Self::LTR => "ltr",
            Self::RTL => "rtl",
            Self::TTB => "ttb",
            Self::BTT => "btt",
        })
    }
}

/// The two generic layouting axes.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum GenAxis {
    /// The primary layouting direction into which text and lines flow.
    Primary,
    /// The secondary layouting direction into which paragraphs grow.
    Secondary,
}

impl GenAxis {
    /// The specific version of this axis in the given layout system.
    pub fn to_spec(self, sys: LayoutSystem) -> SpecAxis {
        sys.get(self).axis()
    }
}

impl Display for GenAxis {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(match self {
            Self::Primary => "primary",
            Self::Secondary => "secondary",
        })
    }
}

/// The two specific layouting axes.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum SpecAxis {
    /// The horizontal layouting axis.
    Horizontal,
    /// The vertical layouting axis.
    Vertical,
}

impl SpecAxis {
    /// The generic version of this axis in the given layout system.
    pub fn to_gen(self, sys: LayoutSystem) -> GenAxis {
        if self == sys.primary.axis() {
            GenAxis::Primary
        } else {
            GenAxis::Secondary
        }
    }
}

impl Display for SpecAxis {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(match self {
            Self::Horizontal => "horizontal",
            Self::Vertical => "vertical",
        })
    }
}

/// Where to align content along an axis in a generic context.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum GenAlign {
    Start,
    Center,
    End,
}

impl GenAlign {
    /// The inverse alignment.
    pub fn inv(self) -> Self {
        match self {
            Self::Start => Self::End,
            Self::Center => Self::Center,
            Self::End => Self::Start,
        }
    }
}

impl Display for GenAlign {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(match self {
            Self::Start => "start",
            Self::Center => "center",
            Self::End => "end",
        })
    }
}

/// Where to align content along an axis in a specific context.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum SpecAlign {
    Left,
    Right,
    Top,
    Bottom,
    Center,
}

impl SpecAlign {
    /// The specific axis this alignment refers to.
    ///
    /// Returns `None` if this is `Center` since the axis is unknown.
    pub fn axis(self) -> Option<SpecAxis> {
        match self {
            Self::Left => Some(SpecAxis::Horizontal),
            Self::Right => Some(SpecAxis::Horizontal),
            Self::Top => Some(SpecAxis::Vertical),
            Self::Bottom => Some(SpecAxis::Vertical),
            Self::Center => None,
        }
    }

    /// The generic version of this alignment in the given layout system.
    pub fn to_gen(self, sys: LayoutSystem) -> GenAlign {
        let get = |spec: SpecAxis, positive: GenAlign| {
            if sys.get(spec.to_gen(sys)).is_positive() {
                positive
            } else {
                positive.inv()
            }
        };

        match self {
            Self::Left => get(SpecAxis::Horizontal, GenAlign::Start),
            Self::Right => get(SpecAxis::Horizontal, GenAlign::End),
            Self::Top => get(SpecAxis::Vertical, GenAlign::Start),
            Self::Bottom => get(SpecAxis::Vertical, GenAlign::End),
            Self::Center => GenAlign::Center,
        }
    }
}

impl Display for SpecAlign {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(match self {
            Self::Left => "left",
            Self::Right => "right",
            Self::Top => "top",
            Self::Bottom => "bottom",
            Self::Center => "center",
        })
    }
}

/// A side of a container.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Side {
    Left,
    Top,
    Right,
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

/// A generic container with two components for the two generic axes.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub struct Gen2<T> {
    /// The primary component.
    pub primary: T,
    /// The secondary component.
    pub secondary: T,
}

impl<T> Gen2<T> {
    /// Create a new instance from the two components.
    pub fn new(primary: T, secondary: T) -> Self {
        Self { primary, secondary }
    }

    /// Return the component for the specified generic axis.
    pub fn get(self, axis: GenAxis) -> T {
        match axis {
            GenAxis::Primary => self.primary,
            GenAxis::Secondary => self.secondary,
        }
    }

    /// Borrow the component for the specified generic axis mutably.
    pub fn get_mut(&mut self, axis: GenAxis) -> &mut T {
        match axis {
            GenAxis::Primary => &mut self.primary,
            GenAxis::Secondary => &mut self.secondary,
        }
    }
}

/// A generic container with two components for the two specific axes.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub struct Spec2<T> {
    /// The horizontal component.
    pub horizontal: T,
    /// The vertical component.
    pub vertical: T,
}

impl<T> Spec2<T> {
    /// Create a new instance from the two components.
    pub fn new(horizontal: T, vertical: T) -> Self {
        Self { horizontal, vertical }
    }

    /// Return the component for the given specific axis.
    pub fn get(self, axis: SpecAxis) -> T {
        match axis {
            SpecAxis::Horizontal => self.horizontal,
            SpecAxis::Vertical => self.vertical,
        }
    }

    /// Borrow the component for the given specific axis mutably.
    pub fn get_mut(&mut self, axis: SpecAxis) -> &mut T {
        match axis {
            SpecAxis::Horizontal => &mut self.horizontal,
            SpecAxis::Vertical => &mut self.vertical,
        }
    }
}

/// A generic container with left, top, right and bottom components.
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

    /// Return the component for the given side.
    pub fn get(self, side: Side) -> T {
        match side {
            Side::Left => self.left,
            Side::Right => self.right,
            Side::Top => self.top,
            Side::Bottom => self.bottom,
        }
    }

    /// Borrow the component for the given side mutably.
    pub fn get_mut(&mut self, side: Side) -> &mut T {
        match side {
            Side::Left => &mut self.left,
            Side::Right => &mut self.right,
            Side::Top => &mut self.top,
            Side::Bottom => &mut self.bottom,
        }
    }
}
