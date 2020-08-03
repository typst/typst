//! Layouting primitives.

use std::fmt::{self, Display, Formatter};

use super::prelude::*;

/// Specifies the axes along content is laid out.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct LayoutAxes {
    pub primary: Dir,
    pub secondary: Dir,
}

impl LayoutAxes {
    /// Create a new instance from the two directions.
    ///
    /// # Panics
    /// This function panics if the directions are aligned, i.e. if they are
    /// on the same axis.
    pub fn new(primary: Dir, secondary: Dir) -> Self {
        if primary.axis() == secondary.axis() {
            panic!("directions {} and {} are aligned", primary, secondary);
        }
        Self { primary, secondary }
    }

    /// Return the direction of the specified generic axis.
    pub fn get(self, axis: GenAxis) -> Dir {
        match axis {
            Primary => self.primary,
            Secondary => self.secondary,
        }
    }

    /// Borrow the direction of the specified generic axis mutably.
    pub fn get_mut(&mut self, axis: GenAxis) -> &mut Dir {
        match axis {
            Primary => &mut self.primary,
            Secondary => &mut self.secondary,
        }
    }
}

/// Directions along which content is laid out.
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
    /// The specific axis this direction belongs to.
    pub fn axis(self) -> SpecAxis {
        match self {
            LTR | RTL => Horizontal,
            TTB | BTT => Vertical,
        }
    }

    /// Whether this direction points into the positive coordinate direction.
    ///
    /// The positive directions are left-to-right and top-to-bottom.
    pub fn is_positive(self) -> bool {
        match self {
            LTR | TTB => true,
            RTL | BTT => false,
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
            LTR => RTL,
            RTL => LTR,
            TTB => BTT,
            BTT => TTB,
        }
    }
}

impl Display for Dir {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(match self {
            LTR => "ltr",
            RTL => "rtl",
            TTB => "ttb",
            BTT => "btt",
        })
    }
}

/// The two generic layouting axes.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum GenAxis {
    /// The primary layouting direction along which text and lines flow.
    Primary,
    /// The secondary layouting direction along which paragraphs grow.
    Secondary,
}

impl GenAxis {
    /// The specific version of this axis in the given system of axes.
    pub fn to_specific(self, axes: LayoutAxes) -> SpecAxis {
        axes.get(self).axis()
    }
}

impl Display for GenAxis {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(match self {
            Primary => "primary",
            Secondary => "secondary",
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
    /// The generic version of this axis in the given system of axes.
    pub fn to_generic(self, axes: LayoutAxes) -> GenAxis {
        if self == axes.primary.axis() { Primary } else { Secondary }
    }
}

impl Display for SpecAxis {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(match self {
            Horizontal => "horizontal",
            Vertical => "vertical",
        })
    }
}

/// Specifies where to align a layout in a parent container.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct LayoutAlign {
    pub primary: GenAlign,
    pub secondary: GenAlign,
}

impl LayoutAlign {
    /// Create a new instance from the two alignments.
    pub fn new(primary: GenAlign, secondary: GenAlign) -> Self {
        Self { primary, secondary }
    }

    /// Return the alignment for the specified generic axis.
    pub fn get(self, axis: GenAxis) -> GenAlign {
        match axis {
            Primary => self.primary,
            Secondary => self.secondary,
        }
    }

    /// Borrow the alignment for the specified generic axis mutably.
    pub fn get_mut(&mut self, axis: GenAxis) -> &mut GenAlign {
        match axis {
            Primary => &mut self.primary,
            Secondary => &mut self.secondary,
        }
    }
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
            Start => End,
            Center => Center,
            End => Start,
        }
    }
}

impl Display for GenAlign {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(match self {
            Start => "start",
            Center => "center",
            End => "end",
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
            Self::Left => Some(Horizontal),
            Self::Right => Some(Horizontal),
            Self::Top => Some(Vertical),
            Self::Bottom => Some(Vertical),
            Self::Center => None,
        }
    }

    /// The generic version of this alignment in the given system of axes.
    pub fn to_generic(self, axes: LayoutAxes) -> GenAlign {
        let get = |spec: SpecAxis, align: GenAlign| {
            let axis = spec.to_generic(axes);
            if axes.get(axis).is_positive() { align } else { align.inv() }
        };

        match self {
            Self::Left => get(Horizontal, Start),
            Self::Right => get(Horizontal, End),
            Self::Top => get(Vertical, Start),
            Self::Bottom => get(Vertical, End),
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

/// Specifies whether to expand a layout to the full size of the space it is
/// laid out in or to shrink it to fit the content.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct LayoutExpansion {
    /// Whether to expand on the horizontal axis.
    pub horizontal: bool,
    /// Whether to expand on the vertical axis.
    pub vertical: bool,
}

impl LayoutExpansion {
    /// Create a new instance from the two values.
    pub fn new(horizontal: bool, vertical: bool) -> Self {
        Self { horizontal, vertical }
    }

    /// Return the expansion value for the given specific axis.
    pub fn get(self, axis: SpecAxis) -> bool {
        match axis {
            Horizontal => self.horizontal,
            Vertical => self.vertical,
        }
    }

    /// Borrow the expansion value for the given specific axis mutably.
    pub fn get_mut(&mut self, axis: SpecAxis) -> &mut bool {
        match axis {
            Horizontal => &mut self.horizontal,
            Vertical => &mut self.vertical,
        }
    }
}

/// Defines how spacing interacts with surrounding spacing.
///
/// There are two options for interaction: Hard and soft spacing. Typically,
/// hard spacing is used when a fixed amount of space needs to be inserted no
/// matter what. In contrast, soft spacing can be used to insert a default
/// spacing between e.g. two words or paragraphs that can still be overridden by
/// a hard space.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum SpacingKind {
    /// Hard spaces are always laid out and consume surrounding soft space.
    Hard,
    /// Soft spaces are not laid out if they are touching a hard space and
    /// consume neighbouring soft spaces with higher levels.
    Soft(u32),
}

impl SpacingKind {
    /// The standard spacing kind used for paragraph spacing.
    pub const PARAGRAPH: Self = Self::Soft(1);

    /// The standard spacing kind used for line spacing.
    pub const LINE: Self = Self::Soft(2);

    /// The standard spacing kind used for word spacing.
    pub const WORD: Self = Self::Soft(1);
}

/// The spacing kind of the most recently inserted item in a layouting process.
///
/// Since the last inserted item may not be spacing at all, this can be `None`.
#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) enum LastSpacing {
    /// The last item was hard spacing.
    Hard,
    /// The last item was soft spacing with the given width and level.
    Soft(f64, u32),
    /// The last item wasn't spacing.
    None,
}

impl LastSpacing {
    /// The width of the soft space if this is a soft space or zero otherwise.
    pub fn soft_or_zero(self) -> f64 {
        match self {
            LastSpacing::Soft(space, _) => space,
            _ => 0.0,
        }
    }
}
