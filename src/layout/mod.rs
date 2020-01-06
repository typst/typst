//! The core layouting engine.

use std::io::{self, Write};
use smallvec::SmallVec;
use toddle::query::{SharedFontLoader, FontIndex};

use crate::size::{Size, Size2D, SizeBox};
use crate::style::LayoutStyle;

mod actions;
mod tree;
mod line;
mod stack;
mod text;

/// Common types for layouting.
pub mod prelude {
    pub use super::{
        layout, LayoutResult,
        MultiLayout, Layout, LayoutContext, LayoutSpaces, LayoutSpace,
        LayoutExpansion, LayoutAxes, GenericAxis, SpecificAxis, Direction,
        LayoutAlignment, Alignment, SpacingKind,
    };
    pub use GenericAxis::*;
    pub use SpecificAxis::*;
    pub use Direction::*;
    pub use Alignment::*;
}

/// Different kinds of layouters (fully re-exported).
pub mod layouters {
    pub use super::tree::layout;
    pub use super::line::{LineLayouter, LineContext};
    pub use super::stack::{StackLayouter, StackContext};
    pub use super::text::{layout_text, TextContext};
}

pub use self::actions::{LayoutAction, LayoutActions};
pub use self::layouters::*;
pub use self::prelude::*;


/// The result type for layouting.
pub type LayoutResult<T> = crate::TypesetResult<T>;

/// A collection of layouts.
pub type MultiLayout = Vec<Layout>;

/// A sequence of layouting actions inside a box.
#[derive(Debug, Clone)]
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

/// The general context for layouting.
#[derive(Debug, Clone)]
pub struct LayoutContext<'a, 'p> {
    /// The font loader to retrieve fonts from when typesetting text
    /// using [`layout_text`].
    pub loader: &'a SharedFontLoader<'p>,
    /// The style for pages and text.
    pub style: &'a LayoutStyle,
    /// The base unpadded dimensions of this container (for relative sizing).
    pub base: Size2D,
    /// The spaces to layout in.
    pub spaces: LayoutSpaces,
    /// Whether to have repeated spaces or to use only the first and only once.
    pub repeat: bool,
    /// The initial axes along which content is laid out.
    pub axes: LayoutAxes,
    /// The alignment of the finished layout.
    pub alignment: LayoutAlignment,
    /// Whether the layout that is to be created will be nested in a parent
    /// container.
    pub nested: bool,
    /// Whether to debug render a box around the layout.
    pub debug: bool,
}

/// A possibly stack-allocated vector of layout spaces.
pub type LayoutSpaces = SmallVec<[LayoutSpace; 2]>;

/// Spacial layouting constraints.
#[derive(Debug, Copy, Clone)]
pub struct LayoutSpace {
    /// The maximum size of the box to layout in.
    pub dimensions: Size2D,
    /// Padding that should be respected on each side.
    pub padding: SizeBox,
    /// Whether to expand the dimensions of the resulting layout to the full
    /// dimensions of this space or to shrink them to fit the content for the
    /// horizontal and vertical axis.
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

/// Whether to fit to content or expand to the space's size.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct LayoutExpansion {
    pub horizontal: bool,
    pub vertical: bool,
}

impl LayoutExpansion {
    pub fn new(horizontal: bool, vertical: bool) -> LayoutExpansion {
        LayoutExpansion { horizontal, vertical }
    }

    /// Borrow the specified component mutably.
    pub fn get_mut(&mut self, axis: SpecificAxis) -> &mut bool {
        match axis {
            Horizontal => &mut self.horizontal,
            Vertical => &mut self.vertical,
        }
    }
}

/// The axes along which the content is laid out.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct LayoutAxes {
    pub primary: Direction,
    pub secondary: Direction,
}

impl LayoutAxes {
    pub fn new(primary: Direction, secondary: Direction) -> LayoutAxes {
        if primary.axis() == secondary.axis() {
            panic!("LayoutAxes::new: invalid aligned axes {:?} and {:?}",
                primary, secondary);
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

    /// Return the direction of the specified specific axis.
    pub fn get_specific(self, axis: SpecificAxis) -> Direction {
        self.get(axis.to_generic(self))
    }
}

/// The two generic layouting axes.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum GenericAxis {
    Primary,
    Secondary,
}

impl GenericAxis {
    /// The specific version of this axis in the given system of axes.
    pub fn to_specific(self, axes: LayoutAxes) -> SpecificAxis {
        axes.get(self).axis()
    }

    /// The other axis.
    pub fn inv(self) -> GenericAxis {
        match self {
            Primary => Secondary,
            Secondary => Primary,
        }
    }
}

/// The two specific layouting axes.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum SpecificAxis {
    Horizontal,
    Vertical,
}

impl SpecificAxis {
    /// The generic version of this axis in the given system of axes.
    pub fn to_generic(self, axes: LayoutAxes) -> GenericAxis {
        if self == axes.primary.axis() { Primary } else { Secondary }
    }

    /// The other axis.
    pub fn inv(self) -> SpecificAxis {
        match self {
            Horizontal => Vertical,
            Vertical => Horizontal,
        }
    }
}

/// Directions along which content is laid out.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
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
    pub fn is_positive(self) -> bool {
        match self {
            LeftToRight | TopToBottom => true,
            RightToLeft | BottomToTop => false,
        }
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

    /// The factor for this direction.
    ///
    /// - `1` if the direction is positive.
    /// - `-1` if the direction is negative.
    pub fn factor(self) -> i32 {
        if self.is_positive() { 1 } else { -1 }
    }
}

/// Where to align a layout in a container.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct LayoutAlignment {
    pub primary: Alignment,
    pub secondary: Alignment,
}

impl LayoutAlignment {
    pub fn new(primary: Alignment, secondary: Alignment) -> LayoutAlignment {
        LayoutAlignment { primary, secondary }
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
    Origin,
    Center,
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

/// Whitespace between boxes with different interaction properties.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum SpacingKind {
    /// A hard space consumes surrounding soft spaces and is always layouted.
    Hard,
    /// A soft space consumes surrounding soft spaces with higher value.
    Soft(u32),
}

/// The standard spacing kind used for paragraph spacing.
const PARAGRAPH_KIND: SpacingKind = SpacingKind::Soft(1);

/// The standard spacing kind used for line spacing.
const LINE_KIND: SpacingKind = SpacingKind::Soft(2);

/// The standard spacing kind used for word spacing.
const WORD_KIND: SpacingKind = SpacingKind::Soft(1);

/// The last appeared spacing.
#[derive(Debug, Copy, Clone, PartialEq)]
enum LastSpacing {
    Hard,
    Soft(Size, u32),
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
