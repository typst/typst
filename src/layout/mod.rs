//! The core layouting engine.

use std::io::{self, Write};

use smallvec::SmallVec;

use toddle::query::{FontClass, SharedFontLoader};
use toddle::Error as FontError;

use crate::func::Command;
use crate::size::{Size, Size2D, SizeBox};
use crate::style::{PageStyle, TextStyle};
use crate::syntax::{FuncCall, Node, SyntaxTree};

mod actions;
mod tree;
mod flex;
mod stacked;
mod text;

/// Different kinds of layouters (fully re-exported).
pub mod layouters {
    pub use super::tree::layout_tree;
    pub use super::flex::{FlexLayouter, FlexContext};
    pub use super::stacked::{StackLayouter, StackContext};
    pub use super::text::{layout_text, TextContext};
}

pub use actions::{LayoutAction, LayoutActionList};
pub use layouters::*;

/// A sequence of layouting actions inside a box.
#[derive(Debug, Clone)]
pub struct Layout {
    /// The size of the box.
    pub dimensions: Size2D,
    /// The actions composing this layout.
    pub actions: Vec<LayoutAction>,
    /// Whether to debug-render this box.
    pub debug_render: bool,
}

impl Layout {
    /// Serialize this layout into an output buffer.
    pub fn serialize<W: Write>(&self, f: &mut W) -> io::Result<()> {
        writeln!(
            f,
            "{:.4} {:.4}",
            self.dimensions.x.to_pt(),
            self.dimensions.y.to_pt()
        )?;
        writeln!(f, "{}", self.actions.len())?;
        for action in &self.actions {
            action.serialize(f)?;
            writeln!(f)?;
        }
        Ok(())
    }
}

/// A collection of layouts.
#[derive(Debug, Clone)]
pub struct MultiLayout {
    pub layouts: Vec<Layout>,
}

impl MultiLayout {
    /// Create an empty multi-layout.
    pub fn new() -> MultiLayout {
        MultiLayout { layouts: vec![] }
    }

    /// Extract the single sublayout. This panics if the layout does not have
    /// exactly one child.
    pub fn into_single(mut self) -> Layout {
        if self.layouts.len() != 1 {
            panic!("into_single: contains not exactly one layout");
        }
        self.layouts.pop().unwrap()
    }

    /// Add a sublayout.
    pub fn add(&mut self, layout: Layout) {
        self.layouts.push(layout);
    }

    /// The count of sublayouts.
    pub fn count(&self) -> usize {
        self.layouts.len()
    }

    /// Whether this layout contains any sublayouts.
    pub fn is_empty(&self) -> bool {
        self.layouts.is_empty()
    }
}

impl MultiLayout {
    /// Serialize this collection of layouts into an output buffer.
    pub fn serialize<W: Write>(&self, f: &mut W) -> io::Result<()> {
        writeln!(f, "{}", self.count())?;
        for layout in self {
            layout.serialize(f)?;
        }
        Ok(())
    }
}

impl IntoIterator for MultiLayout {
    type Item = Layout;
    type IntoIter = std::vec::IntoIter<Layout>;

    fn into_iter(self) -> Self::IntoIter {
        self.layouts.into_iter()
    }
}

impl<'a> IntoIterator for &'a MultiLayout {
    type Item = &'a Layout;
    type IntoIter = std::slice::Iter<'a, Layout>;

    fn into_iter(self) -> Self::IntoIter {
        self.layouts.iter()
    }
}

/// The general context for layouting.
#[derive(Debug, Clone)]
pub struct LayoutContext<'a, 'p> {
    /// The font loader to retrieve fonts from when typesetting text
    /// using [`layout_text`].
    pub loader: &'a SharedFontLoader<'p>,

    /// Whether this layouting process handles the top-level pages.
    pub top_level: bool,

    /// The style to set text with. This includes sizes and font classes
    /// which determine which font from the loaders selection is used.
    pub text_style: &'a TextStyle,

    /// The current size and margins of the top-level pages.
    pub page_style: PageStyle,

    /// The spaces to layout in.
    pub spaces: LayoutSpaces,

    /// The axes to flow on.
    pub axes: LayoutAxes,

    /// Whether layouts should expand to the full dimensions of the space
    /// they lie on or whether should tightly fit the content.
    pub expand: bool,
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
}

impl LayoutSpace {
    /// The actually usable area (dimensions minus padding).
    pub fn usable(&self) -> Size2D {
        self.dimensions.unpadded(self.padding)
    }

    /// The offset from the origin to the start of content, that is,
    /// `(padding.left, padding.top)`.
    pub fn start(&self) -> Size2D {
        Size2D::new(self.padding.left, self.padding.right)
    }

    /// A layout space without padding and dimensions reduced by the padding.
    pub fn usable_space(&self) -> LayoutSpace {
        LayoutSpace {
            dimensions: self.usable(),
            padding: SizeBox::zero(),
        }
    }
}

/// The axes along which the content is laid out.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct LayoutAxes {
    pub primary: AlignedAxis,
    pub secondary: AlignedAxis,
}

impl LayoutAxes {
    /// Returns the generalized version of a `Size2D` dependent on
    /// the layouting axes, that is:
    /// - The x coordinate describes the primary axis instead of the horizontal one.
    /// - The y coordinate describes the secondary axis instead of the vertical one.
    pub fn generalize(&self, size: Size2D) -> Size2D {
        if self.primary.axis.is_horizontal() {
            size
        } else {
            Size2D { x: size.y, y: size.x }
        }
    }

    /// Returns the specialized version of this generalized Size2D.
    /// (Inverse to `generalized`).
    pub fn specialize(&self, size: Size2D) -> Size2D {
        // In fact, generalized is its own inverse. For reasons of clarity
        // at the call site, we still have this second function.
        self.generalize(size)
    }

    /// The position of the anchor specified by the two aligned axes
    /// in the given generalized space.
    pub fn anchor(&self, space: Size2D) -> Size2D {
        Size2D::new(self.primary.anchor(space.x), self.secondary.anchor(space.y))
    }

    /// This axes with `expand` set to the given value for both axes.
    pub fn expanding(&self, expand: bool) -> LayoutAxes {
        LayoutAxes {
            primary: self.primary.expanding(expand),
            secondary: self.secondary.expanding(expand),
        }
    }
}

/// An axis with an alignment.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct AlignedAxis {
    pub axis: Axis,
    pub alignment: Alignment,
    pub expand: bool,
}

impl AlignedAxis {
    /// Creates an aligned axis from its three components.
    pub fn new(axis: Axis, alignment: Alignment, expand: bool) -> AlignedAxis {
        AlignedAxis { axis, alignment, expand }
    }

    /// The position of the anchor specified by this axis on the given line.
    pub fn anchor(&self, line: Size) -> Size {
        use Alignment::*;
        match (self.axis.is_positive(), self.alignment) {
            (true, Origin) | (false, End) => Size::zero(),
            (_, Center) => line / 2,
            (true, End) | (false, Origin) => line,
        }
    }

    /// This axis with `expand` set to the given value.
    pub fn expanding(&self, expand: bool) -> AlignedAxis {
        AlignedAxis { expand, ..*self }
    }

    /// Whether this axis needs expansion.
    pub fn needs_expansion(&self) -> bool {
        self.expand || self.alignment != Alignment::Origin
    }
}

/// Where to put content.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Axis {
    LeftToRight,
    RightToLeft,
    TopToBottom,
    BottomToTop,
}

impl Axis {
    /// Whether this is a horizontal axis.
    pub fn is_horizontal(&self) -> bool {
        match self {
            Axis::LeftToRight | Axis::RightToLeft => true,
            Axis::TopToBottom | Axis::BottomToTop => false,
        }
    }

    /// Whether this axis points into the positive coordinate direction.
    pub fn is_positive(&self) -> bool {
        match self {
            Axis::LeftToRight | Axis::TopToBottom => true,
            Axis::RightToLeft | Axis::BottomToTop => false,
        }
    }

    /// The direction factor for this axis.
    ///
    /// - 1 if the axis is positive.
    /// - -1 if the axis is negative.
    pub fn factor(&self) -> i32 {
        if self.is_positive() { 1 } else { -1 }
    }
}

/// Where to align content.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Alignment {
    Origin,
    Center,
    End,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SpaceState {
    Soft(Size),
    Forbidden,
    Allowed,
}

impl SpaceState {
    fn soft_or_zero(&self) -> Size {
        if let SpaceState::Soft(space) = self { *space } else { Size::zero() }
    }
}

/// The error type for layouting.
pub enum LayoutError {
    /// An action is unallowed in the active context.
    Unallowed(&'static str),
    /// There is not enough space to add an item.
    NotEnoughSpace(&'static str),
    /// There was no suitable font for the given character.
    NoSuitableFont(char),
    /// An error occured while gathering font data.
    Font(FontError),
}

/// The result type for layouting.
pub type LayoutResult<T> = Result<T, LayoutError>;

error_type! {
    err: LayoutError,
    show: f => match err {
        LayoutError::Unallowed(desc) => write!(f, "unallowed: {}", desc),
        LayoutError::NotEnoughSpace(desc) => write!(f, "not enough space: {}", desc),
        LayoutError::NoSuitableFont(c) => write!(f, "no suitable font for '{}'", c),
        LayoutError::Font(err) => write!(f, "font error: {}", err),
    },
    source: match err {
        LayoutError::Font(err) => Some(err),
        _ => None,
    },
    from: (std::io::Error, LayoutError::Font(FontError::Io(err))),
    from: (FontError, LayoutError::Font(err)),
}
