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

impl LayoutAlign {
    /// The layout alignment that has both components set to `Start`.
    pub const START: Self = Self {
        primary: GenAlign::Start,
        secondary: GenAlign::Start,
    };
}

/// Whether to expand a layout to an area's full size or shrink it to fit its content.
pub type LayoutExpansion = Spec2<bool>;

/// The four directions into which content can be laid out.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
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
    /// The side this direction starts at.
    pub fn start(self) -> Side {
        match self {
            Self::LTR => Side::Left,
            Self::RTL => Side::Right,
            Self::TTB => Side::Top,
            Self::BTT => Side::Bottom,
        }
    }

    /// The side this direction ends at.
    pub fn end(self) -> Side {
        match self {
            Self::LTR => Side::Right,
            Self::RTL => Side::Left,
            Self::TTB => Side::Bottom,
            Self::BTT => Side::Top,
        }
    }

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
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
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
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
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

/// A side of a container.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Side {
    Left,
    Top,
    Right,
    Bottom,
}

/// Where to align content along an axis in a generic context.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
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
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
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

/// A generic container with two components for the two generic axes.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
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

    /// Borrow the component for the specified generic axis.
    pub fn get_ref(&mut self, axis: GenAxis) -> &T {
        match axis {
            GenAxis::Primary => &mut self.primary,
            GenAxis::Secondary => &mut self.secondary,
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
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
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

    /// Borrow the component for the given specific axis.
    pub fn get_ref(&mut self, axis: SpecAxis) -> &T {
        match axis {
            SpecAxis::Horizontal => &mut self.horizontal,
            SpecAxis::Vertical => &mut self.vertical,
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
