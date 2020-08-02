//! Layouting types and engines.

use std::fmt::{self, Display, Formatter};

#[cfg(feature = "serialize")]
use serde::Serialize;

use fontdock::FaceId;
use crate::geom::{Size, Margins};
use self::prelude::*;

pub mod line;
pub mod stack;
pub mod text;
pub_use_mod!(actions);
pub_use_mod!(model);

/// Basic types used across the layouting engine.
pub mod prelude {
    pub use super::{
        layout, LayoutContext, LayoutSpace, Command, Commands,
        LayoutAxes, LayoutAlign, LayoutExpansion,
    };
    pub use super::Dir::{self, *};
    pub use super::GenAxis::{self, *};
    pub use super::SpecAxis::{self, *};
    pub use super::GenAlign::{self, *};
    pub use super::SpecAlign::{self, *};
}

/// A collection of layouts.
pub type MultiLayout = Vec<Layout>;

/// A finished box with content at fixed positions.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize))]
pub struct Layout {
    /// The size of the box.
    pub dimensions: Size,
    /// How to align this layout in a parent container.
    #[cfg_attr(feature = "serialize", serde(skip))]
    pub align: LayoutAlign,
    /// The actions composing this layout.
    pub actions: Vec<LayoutAction>,
}

impl Layout {
    /// Returns a vector with all used font indices.
    pub fn find_used_fonts(&self) -> Vec<FaceId> {
        let mut fonts = Vec::new();
        for action in &self.actions {
            if let &LayoutAction::SetFont(id, _) = action {
                if !fonts.contains(&id) {
                    fonts.push(id);
                }
            }
        }
        fonts
    }
}

/// A vector of layout spaces, that is stack allocated as long as it only
/// contains at most 2 spaces.
pub type LayoutSpaces = Vec<LayoutSpace>;

/// The space into which content is laid out.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct LayoutSpace {
    /// The maximum size of the box to layout in.
    pub dimensions: Size,
    /// Padding that should be respected on each side.
    pub padding: Margins,
    /// Whether to expand the dimensions of the resulting layout to the full
    /// dimensions of this space or to shrink them to fit the content.
    pub expansion: LayoutExpansion,
}

impl LayoutSpace {
    /// The offset from the origin to the start of content, that is,
    /// `(padding.left, padding.top)`.
    pub fn start(&self) -> Size {
        Size::new(self.padding.left, self.padding.top)
    }

    /// The actually usable area (dimensions minus padding).
    pub fn usable(&self) -> Size {
        self.dimensions.unpadded(self.padding)
    }

    /// A layout space without padding and dimensions reduced by the padding.
    pub fn usable_space(&self) -> LayoutSpace {
        LayoutSpace {
            dimensions: self.usable(),
            padding: Margins::ZERO,
            expansion: LayoutExpansion::new(false, false),
        }
    }
}

/// Specifies along which axes content is laid out.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct LayoutAxes {
    /// The primary layouting direction.
    pub primary: Dir,
    /// The secondary layouting direction.
    pub secondary: Dir,
}

impl LayoutAxes {
    /// Create a new instance from the two values.
    ///
    /// # Panics
    /// This function panics if the axes are aligned, that is, they are
    /// on the same axis.
    pub fn new(primary: Dir, secondary: Dir) -> LayoutAxes {
        if primary.axis() == secondary.axis() {
            panic!("invalid aligned axes {} and {}", primary, secondary);
        }

        LayoutAxes { primary, secondary }
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
    LTT,
    RTL,
    TTB,
    BTT,
}

impl Dir {
    /// The specific axis this direction belongs to.
    pub fn axis(self) -> SpecAxis {
        match self {
            LTT | RTL => Horizontal,
            TTB | BTT => Vertical,
        }
    }

    /// Whether this axis points into the positive coordinate direction.
    ///
    /// The positive axes are left-to-right and top-to-bottom.
    pub fn is_positive(self) -> bool {
        match self {
            LTT | TTB => true,
            RTL | BTT => false,
        }
    }

    /// The factor for this direction.
    ///
    /// - `1` if the direction is positive.
    /// - `-1` if the direction is negative.
    pub fn factor(self) -> f64 {
        if self.is_positive() { 1.0 } else { -1.0 }
    }

    /// The inverse axis.
    pub fn inv(self) -> Dir {
        match self {
            LTT => RTL,
            RTL => LTT,
            TTB => BTT,
            BTT => TTB,
        }
    }
}

impl Display for Dir {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(match self {
            LTT => "ltr",
            RTL => "rtl",
            TTB => "ttb",
            BTT => "btt",
        })
    }
}

/// The two generic layouting axes.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum GenAxis {
    /// The primary axis along which words are laid out.
    Primary,
    /// The secondary axis along which lines and paragraphs are laid out.
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
    /// The alignment along the primary axis.
    pub primary: GenAlign,
    /// The alignment along the secondary axis.
    pub secondary: GenAlign,
}

impl LayoutAlign {
    /// Create a new instance from the two values.
    pub fn new(primary: GenAlign, secondary: GenAlign) -> LayoutAlign {
        LayoutAlign { primary, secondary }
    }

    /// Return the alignment of the specified generic axis.
    pub fn get(self, axis: GenAxis) -> GenAlign {
        match axis {
            Primary => self.primary,
            Secondary => self.secondary,
        }
    }

    /// Borrow the alignment of the specified generic axis mutably.
    pub fn get_mut(&mut self, axis: GenAxis) -> &mut GenAlign {
        match axis {
            Primary => &mut self.primary,
            Secondary => &mut self.secondary,
        }
    }
}

/// Where to align content along a generic context.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum GenAlign {
    Start,
    Center,
    End,
}

impl GenAlign {
    /// The inverse alignment.
    pub fn inv(self) -> GenAlign {
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

/// Where to align content in a specific context.
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
    /// Returns `None` if this is center.
    pub fn axis(self) -> Option<SpecAxis> {
        match self {
            Self::Left => Some(Horizontal),
            Self::Right => Some(Horizontal),
            Self::Top => Some(Vertical),
            Self::Bottom => Some(Vertical),
            Self::Center => None,
        }
    }

    /// Convert this to a generic alignment.
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
    pub fn new(horizontal: bool, vertical: bool) -> LayoutExpansion {
        LayoutExpansion { horizontal, vertical }
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

/// Defines how a given spacing interacts with (possibly existing) surrounding
/// spacing.
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
    pub const PARAGRAPH: SpacingKind = SpacingKind::Soft(1);

    /// The standard spacing kind used for line spacing.
    pub const LINE: SpacingKind = SpacingKind::Soft(2);

    /// The standard spacing kind used for word spacing.
    pub const WORD: SpacingKind = SpacingKind::Soft(1);
}

/// The spacing kind of the most recently inserted item in a layouting process.
/// This is not about the last _spacing item_, but the last _item_, which is why
/// this can be `None`.
#[derive(Debug, Copy, Clone, PartialEq)]
enum LastSpacing {
    /// The last item was hard spacing.
    Hard,
    /// The last item was soft spacing with the given width and level.
    Soft(f64, u32),
    /// The last item was not spacing.
    None,
}

impl LastSpacing {
    /// The width of the soft space if this is a soft space or zero otherwise.
    fn soft_or_zero(self) -> f64 {
        match self {
            LastSpacing::Soft(space, _) => space,
            _ => 0.0,
        }
    }
}
