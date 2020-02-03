//! Layouting types and engines.

use std::io::{self, Write};
use std::fmt::{self, Display, Formatter};
use smallvec::SmallVec;
use toddle::query::FontIndex;

use crate::size::{Size, Size2D, SizeBox};
use self::prelude::*;

pub mod line;
pub mod stack;
pub mod text;

pub_use_mod!(actions);
pub_use_mod!(model);

/// Basic types used across the layouting engine.
pub mod prelude {
    pub use super::{
        LayoutContext, layout, LayoutSpace,
        Layouted, Commands,
        LayoutAxes, LayoutAlignment, LayoutExpansion
    };
    pub use super::GenericAxis::{self, *};
    pub use super::SpecificAxis::{self, *};
    pub use super::Direction::{self, *};
    pub use super::Alignment::{self, *};
}


/// A collection of layouts.
pub type MultiLayout = Vec<Layout>;

/// A finished box with content at fixed positions.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    /// The size of the box.
    pub dimensions: Size2D,
    /// How to align this layout in a parent container.
    pub alignment: LayoutAlignment,
    /// The actions composing this layout.
    pub actions: Vec<LayoutAction>,
}

impl Layout {
    /// Returns a vector with all used font indices.
    pub fn find_used_fonts(&self) -> Vec<FontIndex> {
        let mut fonts = Vec::new();
        for action in &self.actions {
            if let LayoutAction::SetFont(index, _) = action {
                if !fonts.contains(index) {
                    fonts.push(*index);
                }
            }
        }
        fonts
    }
}

/// Layout components that can be serialized.
pub trait Serialize {
    /// Serialize the data structure into an output writable.
    fn serialize<W: Write>(&self, f: &mut W) -> io::Result<()>;
}

impl Serialize for Layout {
    fn serialize<W: Write>(&self, f: &mut W) -> io::Result<()> {
        writeln!(f, "{:.4} {:.4}", self.dimensions.x.to_pt(), self.dimensions.y.to_pt())?;
        writeln!(f, "{}", self.actions.len())?;
        for action in &self.actions {
            action.serialize(f)?;
            writeln!(f)?;
        }
        Ok(())
    }
}

impl Serialize for MultiLayout {
    fn serialize<W: Write>(&self, f: &mut W) -> io::Result<()> {
        writeln!(f, "{}", self.len())?;
        for layout in self {
            layout.serialize(f)?;
        }
        Ok(())
    }
}

/// A vector of layout spaces, that is stack allocated as long as it only
/// contains at most 2 spaces.
pub type LayoutSpaces = SmallVec<[LayoutSpace; 2]>;

/// The space into which content is laid out.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct LayoutSpace {
    /// The maximum size of the box to layout in.
    pub dimensions: Size2D,
    /// Padding that should be respected on each side.
    pub padding: SizeBox,
    /// Whether to expand the dimensions of the resulting layout to the full
    /// dimensions of this space or to shrink them to fit the content.
    pub expansion: LayoutExpansion,
}

impl LayoutSpace {
    /// The offset from the origin to the start of content, that is,
    /// `(padding.left, padding.top)`.
    pub fn start(&self) -> Size2D {
        Size2D::new(self.padding.left, self.padding.top)
    }

    /// The actually usable area (dimensions minus padding).
    pub fn usable(&self) -> Size2D {
        self.dimensions.unpadded(self.padding)
    }

    /// A layout space without padding and dimensions reduced by the padding.
    pub fn usable_space(&self) -> LayoutSpace {
        LayoutSpace {
            dimensions: self.usable(),
            padding: SizeBox::ZERO,
            expansion: LayoutExpansion::new(false, false),
        }
    }
}

/// The two generic layouting axes.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum GenericAxis {
    /// The primary axis along which words are laid out.
    Primary,
    /// The secondary axis along which lines and paragraphs are laid out.
    Secondary,
}

impl GenericAxis {
    /// The specific version of this axis in the given system of axes.
    pub fn to_specific(self, axes: LayoutAxes) -> SpecificAxis {
        axes.get(self).axis()
    }
}

impl Display for GenericAxis {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Primary => write!(f, "primary"),
            Secondary => write!(f, "secondary"),
        }
    }
}

/// The two specific layouting axes.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum SpecificAxis {
    /// The horizontal layouting axis.
    Horizontal,
    /// The vertical layouting axis.
    Vertical,
}

impl SpecificAxis {
    /// The generic version of this axis in the given system of axes.
    pub fn to_generic(self, axes: LayoutAxes) -> GenericAxis {
        if self == axes.primary.axis() { Primary } else { Secondary }
    }
}

impl Display for SpecificAxis {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Horizontal => write!(f, "horizontal"),
            Vertical => write!(f, "vertical"),
        }
    }
}

/// Specifies along which directions content is laid out.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct LayoutAxes {
    /// The primary layouting direction.
    pub primary: Direction,
    /// The secondary layouting direction.
    pub secondary: Direction,
}

impl LayoutAxes {
    /// Create a new instance from the two values.
    ///
    /// # Panics
    /// This function panics if the directions are aligned, that is, they are
    /// on the same axis.
    pub fn new(primary: Direction, secondary: Direction) -> LayoutAxes {
        if primary.axis() == secondary.axis() {
            panic!("LayoutAxes::new: invalid aligned axes \
                    {} and {}", primary, secondary);
        }

        LayoutAxes { primary, secondary }
    }

    /// Return the direction of the specified generic axis.
    pub fn get(self, axis: GenericAxis) -> Direction {
        match axis {
            Primary => self.primary,
            Secondary => self.secondary,
        }
    }

    /// Borrow the direction of the specified generic axis mutably.
    pub fn get_mut(&mut self, axis: GenericAxis) -> &mut Direction {
        match axis {
            Primary => &mut self.primary,
            Secondary => &mut self.secondary,
        }
    }
}

/// Directions along which content is laid out.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[allow(missing_docs)]
pub enum Direction {
    LeftToRight,
    RightToLeft,
    TopToBottom,
    BottomToTop,
}

impl Direction {
    /// The specific axis this direction belongs to.
    pub fn axis(self) -> SpecificAxis {
        match self {
            LeftToRight | RightToLeft => Horizontal,
            TopToBottom | BottomToTop => Vertical,
        }
    }

    /// Whether this axis points into the positive coordinate direction.
    ///
    /// The positive directions are left-to-right and top-to-bottom.
    pub fn is_positive(self) -> bool {
        match self {
            LeftToRight | TopToBottom => true,
            RightToLeft | BottomToTop => false,
        }
    }

    /// The factor for this direction.
    ///
    /// - `1` if the direction is positive.
    /// - `-1` if the direction is negative.
    pub fn factor(self) -> i32 {
        if self.is_positive() { 1 } else { -1 }
    }

    /// The inverse axis.
    pub fn inv(self) -> Direction {
        match self {
            LeftToRight => RightToLeft,
            RightToLeft => LeftToRight,
            TopToBottom => BottomToTop,
            BottomToTop => TopToBottom,
        }
    }
}

impl Display for Direction {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            LeftToRight => write!(f, "left-to-right"),
            RightToLeft => write!(f, "right-to-left"),
            TopToBottom => write!(f, "top-to-bottom"),
            BottomToTop => write!(f, "bottom-to-top"),
        }
    }
}

/// Specifies where to align a layout in a parent container.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct LayoutAlignment {
    /// The alignment along the primary axis.
    pub primary: Alignment,
    /// The alignment along the secondary axis.
    pub secondary: Alignment,
}

impl LayoutAlignment {
    /// Create a new instance from the two values.
    pub fn new(primary: Alignment, secondary: Alignment) -> LayoutAlignment {
        LayoutAlignment { primary, secondary }
    }

    /// Return the alignment of the specified generic axis.
    pub fn get(self, axis: GenericAxis) -> Alignment {
        match axis {
            Primary => self.primary,
            Secondary => self.secondary,
        }
    }

    /// Borrow the alignment of the specified generic axis mutably.
    pub fn get_mut(&mut self, axis: GenericAxis) -> &mut Alignment {
        match axis {
            Primary => &mut self.primary,
            Secondary => &mut self.secondary,
        }
    }
}

/// Where to align content.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Alignment {
    /// Align content at the start of the axis.
    Origin,
    /// Align content centered on the axis.
    Center,
    /// Align content at the end of the axis.
    End,
}

impl Alignment {
    /// The inverse alignment.
    pub fn inv(self) -> Alignment {
        match self {
            Origin => End,
            Center => Center,
            End => Origin,
        }
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
    pub fn get(self, axis: SpecificAxis) -> bool {
        match axis {
            Horizontal => self.horizontal,
            Vertical => self.vertical,
        }
    }

    /// Borrow the expansion value for the given specific axis mutably.
    pub fn get_mut(&mut self, axis: SpecificAxis) -> &mut bool {
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
    Soft(Size, u32),
    /// The last item was not spacing.
    None,
}

impl LastSpacing {
    /// The size of the soft space if this is a soft space or zero otherwise.
    fn soft_or_zero(self) -> Size {
        match self {
            LastSpacing::Soft(space, _) => space,
            _ => Size::ZERO,
        }
    }
}
